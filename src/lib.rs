use alloy::eips::BlockNumberOrTag;
use alloy::network::AnyNetwork;
use alloy::primitives::FixedBytes;
use alloy::providers::{Provider, RootProvider};
use alloy::rpc::types::{BlockTransactionsKind, Log};
use alloy::{sol, transports::http};
use backon::ExponentialBuilder;
use backon::Retryable;
use itertools::Itertools;
use std::collections::BTreeMap;
use tracing::*;

sol! {
    #[sol(rpc)]
    IOrderBookV4, "./abi/orderbookv4.json"
}

pub mod env;

#[derive(Debug, Clone)]
pub enum TradeEvent {
    ClearV2,
    TakeOrderV2,
}

#[derive(Debug, Clone)]
pub struct Trade {
    // timestamp: u64,
    event: TradeEvent,
    tx_hash: FixedBytes<32>,
    // tx_origin: Address,
}

type OrderbookContract = IOrderBookV4::IOrderBookV4Instance<
    http::Http<http::Client>,
    RootProvider<http::Http<http::Client>, AnyNetwork>,
    AnyNetwork,
>;

pub async fn process_block_batch(
    start_block: u64,
    end_block: u64,
    orderbook: &OrderbookContract,
) -> anyhow::Result<()> {
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

    let mut clearv2_trades = BTreeMap::<u64, Vec<(u64, Trade)>>::new();

    let clearv2_trades_iter = clearv2_logs
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
            );

    for (block_number, (log_index, trade)) in clearv2_trades_iter {
        clearv2_trades
            .entry(block_number)
            .and_modify(|trades| trades.push((log_index, trade.clone())))
            .or_insert(vec![(log_index, trade)]);
    }

    let clearv2_trades_count: usize = clearv2_trades.values().map(|trades| trades.len()).sum();
    debug!("Blocks [{start_block}, {end_block}] emitted {clearv2_trades_count:>02} ClearV2 events");

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

    let mut takeorderv2_trades = BTreeMap::<u64, Vec<(u64, Trade)>>::new();

    let takeorderv2_trades_iter = takeorderv2_logs
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
            );

    for (block_number, (log_index, trade)) in takeorderv2_trades_iter {
        takeorderv2_trades
            .entry(block_number)
            .and_modify(|trades| trades.push((log_index, trade.clone())))
            .or_insert(vec![(log_index, trade)]);
    }

    let takeorderv2_trades_count: usize =
        takeorderv2_trades.values().map(|trades| trades.len()).sum();
    debug!(
            "Blocks [{start_block}, {end_block}] emitted {takeorderv2_trades_count:>02} TakeOrderV2 events"
        );

    let blocks_with_trades = clearv2_trades
        .keys()
        .copied()
        .chain(takeorderv2_trades.keys().copied())
        .sorted()
        .collect_vec();

    let mut block_bodies = BTreeMap::new();

    for block_number in &blocks_with_trades {
        debug!("Fetching block #{block_number}");
        let block = orderbook
            .provider()
            .get_block_by_number(
                BlockNumberOrTag::Number(*block_number),
                BlockTransactionsKind::Full,
            )
            .await?;

        match block {
            None => {
                error!("Get block with number {block_number} returned None");
                continue;
            }
            Some(block) => {
                block_bodies.insert(block_number, block);
            }
        }
    }

    let trades = blocks_with_trades
        .into_iter()
        .flat_map(|block_number| {
            let clearv2_trade = clearv2_trades.remove(&block_number).unwrap_or_default();
            let takeorderv2_trade = takeorderv2_trades.remove(&block_number).unwrap_or_default();

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

    Ok(())
}
