use alloy::primitives::{Address, FixedBytes};
use alloy::providers::{Provider, ProviderBuilder};
use alloy::rpc::types::Log;
use alloy::sol;
use backon::ExponentialBuilder;
use backon::Retryable;
use clap::Parser;
use itertools::Itertools;
use std::collections::BTreeMap;
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

#[derive(Debug)]
enum TradeEvent {
    ClearV2,
    TakeOrderV2,
}

#[derive(Debug)]
struct Trade {
    // timestamp: u64,
    event: TradeEvent,
    tx_hash: FixedBytes<32>,
    // tx_origin: Address,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    let env = Env::parse();
    let env_filter = format!("none,rain_drops={log_level}", log_level = &env.log_level);
    tracing_subscriber::fmt()
        .with_max_level(env.log_level)
        .with_env_filter(tracing_subscriber::EnvFilter::new(env_filter))
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

        let mut clearv2_trades = clearv2_logs
            .into_iter()
            .filter_map(
                |(
                    _event,
                    Log {
                        log_index,
                        block_number,
                        transaction_hash,
                        ..
                    },
                )| {
                    trace!("ClearV2 log: log_index={log_index:?} block_number={block_number:?} transaction_hash={transaction_hash:?}");

                    let log_index = log_index?;
                    let tx_hash = transaction_hash?;
                    let block_number = block_number?;

                    let trade = Trade {
                        event: TradeEvent::ClearV2,
                        tx_hash,
                    };

                    Some((block_number, (log_index, trade)))
                },
            )
            .collect::<BTreeMap<_, _>>();

        let clearv2_trades_count = clearv2_trades.len();
        debug!(
            "Blocks [{start_block}, {end_block}] emitted {clearv2_trades_count:>02} ClearV2 events"
        );

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

        let mut takeorderv2_trades = takeorderv2_logs
            .into_iter()
            .filter_map(
                |(
                    _event,
                    Log {
                        log_index,
                        block_number,
                        transaction_hash,
                        ..
                    },
                )| {
                    trace!("TakeOrderV2 log: log_index={log_index:?} block_number={block_number:?} transaction_hash={transaction_hash:?}");

                    let log_index = log_index?;
                    let tx_hash = transaction_hash?;
                    let block_number = block_number?;

                    let trade = Trade {
                        event: TradeEvent::TakeOrderV2,
                        tx_hash,
                    };

                    Some((block_number, (log_index, trade)))
                },
            )
            .collect::<BTreeMap<_, _>>();

        let takeorderv2_trades_count = takeorderv2_trades.len();
        debug!(
            "Blocks [{start_block}, {end_block}] emitted {takeorderv2_trades_count:>02} TakeOrderV2 events"
        );

        let blocks_with_trades = clearv2_trades
            .keys()
            .copied()
            .chain(takeorderv2_trades.keys().copied())
            .sorted();

        let trades = blocks_with_trades
            .flat_map(|block_number| {
                let clearv2_trade = clearv2_trades.remove(&block_number);
                let takeorderv2_trade = takeorderv2_trades.remove(&block_number);

                clearv2_trade
                    .into_iter()
                    .chain(takeorderv2_trade.into_iter())
                    .sorted_by_key(|(log_index, _)| *log_index)
                    .map(|(_, trade)| trade)
            })
            .collect_vec();

        // Block range to test event ordering:
        // Blocks 295976000 through 296076000 emitted 01 ClearV2 and 35 TakeOrderV2 events

        let trade_count = trades.len();
        info!("Blocks [{start_block}, {end_block}] emitted {trade_count:>02} trade events");
    }

    Ok(())
}
