use alloy::primitives::Address;
use alloy::providers::{Provider, ProviderBuilder};
// use alloy::rpc::types::Log;
use alloy::sol;
use backon::ExponentialBuilder;
use backon::Retryable;
use clap::Parser;
use tracing::*;

#[derive(Debug, Parser)]
struct Env {
    #[clap(long, env, default_value = "INFO")]
    log_level: Level,
    #[clap(long, env)]
    json_rpc_http_url: String,
    #[clap(long, env)]
    orderbookv4_deployment_address: String,
    #[clap(long, env)]
    orderbookv4_deployment_block: u64,
}

sol! {
    #[sol(rpc)]
    IOrderBookV4, "./abi/orderbookv4.json"
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    let env = Env::parse();
    tracing_subscriber::fmt()
        .with_max_level(env.log_level)
        .init();

    let rpc_url = env.json_rpc_http_url.parse()?;
    let provider = ProviderBuilder::new().on_http(rpc_url);
    let latest_block = provider.get_block_number().await?;

    info!("Latest block is {latest_block}");

    let orderbook = env.orderbookv4_deployment_address.parse::<Address>()?;
    let orderbook = IOrderBookV4::new(orderbook, provider);

    const BLOCKS_PER_REQ: u64 = 100_000;

    for start_block in
        (env.orderbookv4_deployment_block..latest_block).step_by(BLOCKS_PER_REQ as usize)
    {
        let end_block = start_block + BLOCKS_PER_REQ;
        debug!("Fetching logs from {start_block} to {end_block}");

        let clearv2_query = || async {
            orderbook
                .ClearV2_filter()
                .from_block(start_block)
                .to_block(end_block)
                .query()
                .await
        };

        let clearv2_logs = clearv2_query
            .retry(ExponentialBuilder::default())
            .notify(|err, dur| {
                warn!("Retrying querying ClearV2 logs from {start_block} to {end_block} in {dur:?} due to {err:?}");
            })
            .await?;

        let takeorderv2_query = || async {
            orderbook
                .TakeOrderV2_filter()
                .from_block(start_block)
                .to_block(end_block)
                .query()
                .await
        };

        let takeorderv2_logs = takeorderv2_query
            .retry(ExponentialBuilder::default())
            .notify(|err, dur| {
                warn!("Retrying querying TakeOrderV2 logs from {start_block} to {end_block} in {dur:?} due to {err:?}");
            })
            .await?;

        let clearv2_log_count = clearv2_logs.len();
        let takeorderv2_log_count = takeorderv2_logs.len();

        info!(
            "Blocks {start_block} through {end_block} emitted \
            {clearv2_log_count:>02} ClearV2 and {takeorderv2_log_count:>02} TakeOrderV2 events"
        );

        //     let swaps = logs
        //         .into_iter()
        //         .map(
        //             |(
        //                 swap,
        //                 Log {
        //                     block_number,
        //                     transaction_hash,
        //                     ..
        //                 },
        //             )| {
        //                 Trade {
        //                     address: swap.to,
        //                     weth: (swap.amount0In + swap.amount0Out).into(),
        //                     yourai: (swap.amount1In + swap.amount1Out).into(),
        //                     side: if swap.amount1In == U256::from(0) {
        //                         Side::Buy
        //                     } else {
        //                         Side::Sell
        //                     },
        //                     block_num: block_number.unwrap(),
        //                     tx_hash: transaction_hash.unwrap(),
        //                 }
        //             },
        //         )
        //         .collect::<Vec<_>>();

        //     info!("{swaps:#?}");
    }

    Ok(())
}
