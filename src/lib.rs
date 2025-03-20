use alloy::eips::BlockNumberOrTag;
use alloy::network::{AnyHeader, AnyNetwork, AnyTxEnvelope, BlockResponse, TransactionResponse};
use alloy::primitives::{Address, BlockNumber, FixedBytes};
use alloy::providers::{Provider, RootProvider};
use alloy::rpc::types::{Block, BlockTransactions, BlockTransactionsKind, Header, Transaction};
use alloy::{sol, transports::http};
use itertools::Itertools;
use std::collections::BTreeMap;
use tracing::*;

sol! {
    #[sol(rpc)]
    IOrderBookV4, "./abi/orderbookv4.json"
}

pub mod env;
pub mod logs;

#[derive(Debug, Clone)]
pub enum TradeEvent {
    ClearV2,
    TakeOrderV2,
}

/// A partial trade is a trade that has been parsed from a log event.
#[derive(Debug, Clone)]
pub struct PartialTrade {
    log_index: u64,
    event: TradeEvent,
    tx_hash: FixedBytes<32>,
    block_number: BlockNumber,
}

/// A trade with all required fields that combines partial trades
/// enriched with block data.
#[derive(Debug, Clone)]
pub struct Trade {
    timestamp: u64,
    event: TradeEvent,
    tx_hash: FixedBytes<32>,
    tx_origin: Address,
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

    let mut clearv2_trades = logs::fetch_clearv2_trades(start_block, end_block, orderbook).await?;

    let clearv2_trades_count: usize = clearv2_trades.values().map(|trades| trades.len()).sum();
    debug!("Blocks [{start_block}, {end_block}] emitted {clearv2_trades_count} ClearV2 events");

    let mut takeorderv2_trades =
        logs::fetch_takeorderv2_trades(start_block, end_block, orderbook).await?;

    let takeorderv2_trades_count: usize =
        takeorderv2_trades.values().map(|trades| trades.len()).sum();
    debug!(
        "Blocks [{start_block}, {end_block}] emitted {takeorderv2_trades_count} TakeOrderV2 events"
    );

    let blocks_with_trades = clearv2_trades
        .keys()
        .copied()
        .chain(takeorderv2_trades.keys().copied())
        .sorted()
        .collect_vec();

    let mut block_bodies = fetch_block_bodies(orderbook, blocks_with_trades.clone()).await?;

    let trades = blocks_with_trades
        .into_iter()
        .flat_map(|block_number| {
            let clearv2_trade = clearv2_trades.remove(&block_number).unwrap_or_default();
            let takeorderv2_trade = takeorderv2_trades.remove(&block_number).unwrap_or_default();

            clearv2_trade
                .into_iter()
                .chain(takeorderv2_trade.into_iter())
                .sorted_by_key(|trade| trade.log_index)
                .map(|trade| trade)
        })
        .map(|trade| {
            let block = block_bodies.remove(&trade.block_number).unwrap();

            let timestamp = block.header.timestamp;
            let tx_origin = block
                .transactions()
                .clone()
                .into_transactions()
                .find_map(|tx| {
                    if tx.tx_hash() == trade.tx_hash {
                        Some(tx.from)
                    } else {
                        None
                    }
                })
                .unwrap();

            Trade {
                timestamp,
                tx_origin,
                event: trade.event,
                tx_hash: trade.tx_hash,
            }
        })
        .collect_vec();

    // Block range to test event ordering:
    // Blocks 295976000 through 296076000 emitted 01 ClearV2 and 35 TakeOrderV2 events

    let trade_count = trades.len();
    info!("Blocks [{start_block}, {end_block}] emitted {trade_count} trade events");

    #[cfg(debug_assertions)]
    assert_eq!(trade_count, clearv2_trades_count + takeorderv2_trades_count);

    Ok(())
}

async fn fetch_block_bodies(
    orderbook: &OrderbookContract,
    block_numbers: impl IntoIterator<Item = BlockNumber>,
) -> anyhow::Result<BTreeMap<BlockNumber, Block<Transaction<AnyTxEnvelope>, Header<AnyHeader>>>> {
    let mut block_bodies = BTreeMap::new();

    for block_number in block_numbers {
        trace!("Fetching block #{block_number}");
        let block = orderbook
            .provider()
            .get_block_by_number(
                BlockNumberOrTag::Number(block_number),
                BlockTransactionsKind::Full,
            )
            .await?;

        match block {
            None => {
                error!("Get block with number {block_number} returned None");
                continue;
            }
            Some(block) => {
                let Block {
                    header,
                    uncles,
                    transactions,
                    withdrawals,
                } = block.inner;
                let transactions = transactions
                    .into_transactions()
                    .map(|tx| tx.inner)
                    .collect_vec();
                let transactions = BlockTransactions::Full(transactions);
                let block = Block {
                    header,
                    uncles,
                    transactions,
                    withdrawals,
                };

                block_bodies.insert(block_number, block);
            }
        }
    }

    Ok(block_bodies)
}
