#![warn(clippy::complexity)]

use ::rain_drops::env::Env;
use ::rain_drops::onchain::real::RealChain;
use ::rain_drops::update_trades_csv;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let env = Env::init();
    let orderbook = env.connect_contract()?;
    let onchain = RealChain::new(orderbook);

    update_trades_csv(&env, &onchain).await?;

    Ok(())
}
