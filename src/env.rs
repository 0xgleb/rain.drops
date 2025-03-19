use alloy::network::AnyNetwork;
use alloy::primitives::Address;
use alloy::providers::ProviderBuilder;
use clap::Parser;

use crate::{IOrderBookV4, OrderbookContract};

#[derive(Debug, Parser)]
pub struct Env {
    #[clap(long, env, default_value = "DEBUG")]
    log_level: tracing::Level,
    #[clap(long, env)]
    json_rpc_http_url: String,
    #[clap(long, env)]
    orderbookv4_deployment_address: String,
    #[clap(long, env)]
    pub orderbookv4_deployment_block: u64,
}

impl Env {
    pub fn init() -> Self {
        dotenv::dotenv().ok();
        let env = Env::parse();
        let env_filter = format!("none,rain_drops={log_level}", log_level = &env.log_level);

        tracing_subscriber::fmt()
            .with_max_level(env.log_level)
            .with_env_filter(tracing_subscriber::EnvFilter::new(env_filter))
            .init();

        env
    }

    pub fn connect_contract(&self) -> anyhow::Result<OrderbookContract> {
        let rpc_url = self.json_rpc_http_url.parse()?;
        let provider = ProviderBuilder::new()
            .network::<AnyNetwork>()
            .on_http(rpc_url);

        let orderbook = self.orderbookv4_deployment_address.parse::<Address>()?;
        let orderbook = IOrderBookV4::new(orderbook, provider.clone());

        Ok(orderbook)
    }
}
