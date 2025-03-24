//! A module for parsing the environment variables and initializing the
//! [`Env`] struct.

use alloy::network::AnyNetwork;
use alloy::primitives::Address;
use alloy::providers::ProviderBuilder;
use clap::Parser;

use crate::{IOrderBookV4, OrderbookContract};

/// Configuration options for the CLI tool.
///
/// The options can be set by environment variables or command line arguments.
#[derive(Debug, Parser)]
pub struct Env {
    /// The log level to use.
    #[clap(long, env, default_value = "DEBUG")]
    pub log_level: tracing::Level,

    /// The path to the CSV file to read/write trades to/from.
    #[clap(long, env, default_value = "trades.csv")]
    pub csv_path: String,

    /// The URL of the JSON-RPC HTTP endpoint to use.
    #[clap(long, env)]
    pub json_rpc_http_url: String,

    /// The address of the deployed OrderbookV4 contract.
    #[clap(long, env)]
    pub orderbookv4_deployment_address: String,

    /// The block number when the OrderbookV4 contract was deployed.
    #[clap(long, env)]
    pub orderbookv4_deployment_block: u64,

    /// The number of blocks to fetch event logs from at a time.
    #[clap(long, env, default_value = "100000")]
    pub blocks_per_log_request: u64,
}

impl Env {
    /// Read the configuration from the environment and set up logging.
    pub fn init() -> Self {
        dotenv::dotenv().ok();
        let env = Env::parse();
        let env_filter =
            format!("none,rain_drops={log_level}", log_level = &env.log_level);

        tracing_subscriber::fmt()
            .with_max_level(env.log_level)
            .with_env_filter(tracing_subscriber::EnvFilter::new(env_filter))
            .init();

        env
    }

    /// Create an instance of the orderbook contract connected to the blockchain
    /// via the configured JSON-RPC HTTP URL.
    pub fn connect_contract(&self) -> anyhow::Result<OrderbookContract> {
        let rpc_url = self.json_rpc_http_url.parse()?;
        let provider =
            ProviderBuilder::new().network::<AnyNetwork>().on_http(rpc_url);

        let orderbook =
            self.orderbookv4_deployment_address.parse::<Address>()?;
        let orderbook = IOrderBookV4::new(orderbook, provider.clone());

        Ok(orderbook)
    }
}
