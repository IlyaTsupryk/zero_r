// use rust_decimal::Decimal;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use sqlx::{MySql, Pool};
use std::collections::HashMap;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use commons::dlmm::accounts::{BinArray, BinArrayBitmapExtension, LbPair};
use commons::{
    derive_bin_array_bitmap_extension, get_bin_array_pubkeys_for_swap, quote_exact_in,
    rpc_client_extension::RpcClientExtension,
};
use solana_sdk::account::Account;

struct TradeConfig {
    pub pool_pubkey: Pubkey,
    pub _precision: u64,
}

fn get_trade_pairs() -> HashMap<String, TradeConfig> {
    let mut map = HashMap::new();
    map.insert(
        "TRUMPUSDC".to_string(),
        TradeConfig {
            pool_pubkey: Pubkey::from_str_const("9d9mb8kooFfaD3SctgZtkxQypkshx6ezhbKio89ixyy2"),
            _precision: 6,
        },
    );
    map
}

/// Helper struct to hold all accounts needed for swap quote calculation
pub struct SwapQuoteAccounts {
    pub lb_pair_state: LbPair,
    pub clock: solana_sdk::clock::Clock,
    pub mint_x_account: Account,
    pub mint_y_account: Account,
    pub bin_arrays: HashMap<Pubkey, BinArray>,
}

pub struct MeteoraScreener {
    pub db_pool: Pool<MySql>,
    pub rpc_client: RpcClient,
    pub shutdown: Arc<AtomicBool>,
}

impl MeteoraScreener {
    pub fn new(db_pool: Pool<MySql>) -> Self {
        let helus_api_key = std::env::var("HELIUS_API_KEY").expect("HELIUS_API_KEY must be set");
        let rpc_client = RpcClient::new_with_commitment(
            format!("https://mainnet.helius-rpc.com/?api-key={}", helus_api_key),
            CommitmentConfig::confirmed(),
        );
        Self {
            db_pool,
            rpc_client,
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.get_price("TRUMPUSDC", 1_000_000).await?;
        Ok(())
    }

    pub async fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.shutdown.store(true, Ordering::Relaxed);
        Ok(())
    }

    pub async fn get_price(
        &self,
        symbol: &str,
        amount_in: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let trade_pairs = get_trade_pairs();
        let trade_config = trade_pairs.get(symbol).ok_or("Trade config not found")?;
        let lb_pair = trade_config.pool_pubkey;
        let swap_for_y = false; // Swap TRUMP for USDC

        // Fetch the LB pair state from the chain
        let lb_pair_state: LbPair = self
            .rpc_client
            .get_account_and_deserialize(&lb_pair, |account| {
                Ok(bytemuck::pod_read_unaligned(&account.data[8..]))
            })
            .await?;

        // Get bitmap extension (optional, for pools with extended liquidity range)
        let bitmap_extension = self.fetch_bitmap_extension(lb_pair).await?;

        // Get bin arrays needed for the swap (4 is usually enough for most swaps)
        let bin_arrays_for_swap = get_bin_array_pubkeys_for_swap(
            lb_pair,
            &lb_pair_state,
            bitmap_extension.as_ref(),
            swap_for_y,
            4,
        )?;
        // Fetch required accounts for quote calculation
        let quote_accounts = self
            .fetch_quote_required_accounts(lb_pair, &lb_pair_state, bin_arrays_for_swap)
            .await?;

        // Calculate the swap quote using commons::quote_exact_in
        let quote = quote_exact_in(
            lb_pair,
            &quote_accounts.lb_pair_state,
            amount_in,
            swap_for_y,
            quote_accounts.bin_arrays,
            bitmap_extension.as_ref(),
            &quote_accounts.clock,
            &quote_accounts.mint_x_account,
            &quote_accounts.mint_y_account,
        )?;

        tracing::info!(
            "Swap quote: amount_in={}, amount_out={}, fee={}",
            amount_in,
            quote.amount_out,
            quote.fee
        );

        // Calculate price impact and other metrics
        let effective_price = if quote.amount_out > 0 {
            (amount_in as f64) / (quote.amount_out as f64)
        } else {
            0.0
        };

        tracing::info!("Effective price: {:.6}", effective_price);
        tracing::info!(
            "Fee percentage: {:.4}%",
            (quote.fee as f64 / amount_in as f64) * 100.0
        );

        Ok(())
    }

    /// Fetch all required accounts for swap quote calculation
    async fn fetch_quote_required_accounts(
        &self,
        lb_pair: Pubkey,
        lb_pair_state: &LbPair,
        bin_arrays_for_swap: Vec<Pubkey>,
    ) -> Result<SwapQuoteAccounts, Box<dyn std::error::Error>> {
        let prerequisite_accounts = [
            lb_pair,
            solana_sdk::sysvar::clock::ID,
            lb_pair_state.token_x_mint,
            lb_pair_state.token_y_mint,
        ];

        let accounts_to_fetch =
            [prerequisite_accounts.to_vec(), bin_arrays_for_swap.clone()].concat();

        let accounts = self
            .rpc_client
            .get_multiple_accounts(&accounts_to_fetch)
            .await?;

        // Parse accounts
        let mut index = 0;

        // Skip LB pair (we already have it)
        index += 1;

        // Clock account
        let clock_account = accounts
            .get(index)
            .and_then(ToOwned::to_owned)
            .ok_or("Failed to fetch clock account")?;
        let clock: solana_sdk::clock::Clock = bincode::deserialize(clock_account.data.as_ref())?;
        index += 1;

        // Mint X account
        let mint_x_account = accounts
            .get(index)
            .and_then(ToOwned::to_owned)
            .ok_or("Failed to fetch mint X account")?;
        index += 1;

        // Mint Y account
        let mint_y_account = accounts
            .get(index)
            .and_then(ToOwned::to_owned)
            .ok_or("Failed to fetch mint Y account")?;

        // Bin array accounts
        let bin_array_accounts = accounts
            .get(prerequisite_accounts.len()..)
            .ok_or("Failed to fetch bin array accounts")?
            .to_vec();

        let bin_arrays: HashMap<Pubkey, BinArray> = bin_array_accounts
            .into_iter()
            .zip(bin_arrays_for_swap.iter())
            .filter_map(|(account, &key)| {
                let account = account?;
                let bin_array: BinArray = bytemuck::pod_read_unaligned(&account.data[8..]);
                Some((key, bin_array))
            })
            .collect();

        Ok(SwapQuoteAccounts {
            lb_pair_state: *lb_pair_state,
            clock,
            mint_x_account,
            mint_y_account,
            bin_arrays,
        })
    }

    async fn fetch_bitmap_extension(
        &self,
        lb_pair: Pubkey,
    ) -> Result<Option<BinArrayBitmapExtension>, Box<dyn std::error::Error>> {
        let (bitmap_extension_key, _bump) = derive_bin_array_bitmap_extension(lb_pair);
        let bitmap_extension: Option<BinArrayBitmapExtension> = self
            .rpc_client
            .get_account_and_deserialize(&bitmap_extension_key, |account| {
                Ok(bytemuck::pod_read_unaligned(&account.data[8..]))
            })
            .await
            .ok();
        Ok(bitmap_extension)
    }
}
