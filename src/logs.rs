//! A module for fetching and parsing OrderbookV4 event logs from the blockchain.

use alloy::primitives::BlockNumber;
use alloy::primitives::FixedBytes;
use alloy::rpc::types::Log;
use backon::ExponentialBuilder;
use backon::Retryable;
use std::collections::BTreeMap;
use tracing::*;

use crate::OrderbookContract;

/// A partial trade is a trade that has been parsed from a log event.
#[derive(Debug, Clone)]
pub(crate) struct TradeLog {
    pub(crate) log_index: u64,
    pub(crate) block_number: BlockNumber,
    pub(crate) tx_hash: FixedBytes<32>,
    pub(crate) event: TradeEvent,
}

/// An enum representing the kind of trade event that occurred.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum TradeEvent {
    ClearV2,
    TakeOrderV2,
}

/// Fetch all ClearV2 trades from the given block range.
pub(crate) async fn fetch_clearv2_trades(
    start_block: u64,
    end_block: u64,
    orderbook: &OrderbookContract,
) -> anyhow::Result<BTreeMap<BlockNumber, Vec<TradeLog>>> {
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

    let mut clearv2_trades = BTreeMap::<BlockNumber, Vec<TradeLog>>::new();

    let clearv2_trades_iter = clearv2_logs.into_iter().filter_map(
        |(
            _event,
            Log {
                log_index,
                block_number,
                transaction_hash,
                ..
            },
        )| {
            trace!(
                "ClearV2 log: log_index={log_index:?} block_number={block_number:?} \
                    transaction_hash={transaction_hash:?}"
            );

            let log_index = log_index?;
            let tx_hash = transaction_hash?;
            let block_number = block_number?;

            let trade = TradeLog {
                log_index,
                event: TradeEvent::ClearV2,
                tx_hash,
                block_number,
            };

            Some((block_number, trade))
        },
    );

    for (block_number, trade) in clearv2_trades_iter {
        clearv2_trades
            .entry(block_number)
            .and_modify(|trades| trades.push(trade.clone()))
            .or_insert(vec![trade]);
    }

    Ok(clearv2_trades)
}

/// Fetch all TakeOrderV2 trades from the given block range.
pub(crate) async fn fetch_takeorderv2_trades(
    start_block: u64,
    end_block: u64,
    orderbook: &OrderbookContract,
) -> anyhow::Result<BTreeMap<BlockNumber, Vec<TradeLog>>> {
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

    let mut takeorderv2_trades = BTreeMap::<BlockNumber, Vec<TradeLog>>::new();

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

                    let trade = TradeLog {
                        log_index,
                        event: TradeEvent::TakeOrderV2,
                        tx_hash,
                        block_number,
                    };

                    Some((block_number, trade))
                },
            );

    for (block_number, trade) in takeorderv2_trades_iter {
        takeorderv2_trades
            .entry(block_number)
            .and_modify(|trades| trades.push(trade.clone()))
            .or_insert(vec![trade]);
    }

    Ok(takeorderv2_trades)
}
