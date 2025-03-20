use alloy::providers::Provider;
use tracing::*;

use ::rain_drops::env::Env;
use ::rain_drops::process_block_batch;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let env = Env::init();
    let orderbook = env.connect_contract()?;

    let csv_file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .open(env.csv_path)
        .unwrap();
    let mut csv_writer = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(csv_file);

    let latest_block = orderbook.provider().get_block_number().await?;
    info!("Latest block is {latest_block}");

    const BLOCKS_PER_REQ: u64 = 100_000;

    for start_block in
        (env.orderbookv4_deployment_block..latest_block).step_by(BLOCKS_PER_REQ as usize)
    {
        let end_block = start_block + BLOCKS_PER_REQ;
        process_block_batch(&mut csv_writer, &orderbook, start_block, end_block).await?;
    }

    Ok(())
}
