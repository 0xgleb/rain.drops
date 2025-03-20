use ::rain_drops::env::Env;
use ::rain_drops::update_trades_csv;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let env = Env::init();
    let orderbook = env.connect_contract()?;

    update_trades_csv(&env, &orderbook).await?;

    Ok(())
}
