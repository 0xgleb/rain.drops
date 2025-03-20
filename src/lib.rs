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

pub async fn update_trades_csv(
    env: &env::Env,
    orderbook: &OrderbookContract,
) -> anyhow::Result<()> {
    let start_block = get_start_block(env, orderbook).await?;

    let csv_file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .open(&env.csv_path)
        .unwrap();

    let mut csv_writer = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(csv_file);

    let latest_block = orderbook.provider().get_block_number().await?;
    info!("Latest block is {latest_block}");

    for block_batch_start in
        (start_block..latest_block).step_by(env.blocks_per_log_request as usize)
    {
        let block_batch_end = block_batch_start + env.blocks_per_log_request;
        process_block_batch(
            &mut csv_writer,
            &orderbook,
            block_batch_start,
            block_batch_end,
        )
        .await?;
    }

    Ok(())
}

async fn get_start_block(
    env: &env::Env,
    orderbook: &OrderbookContract,
) -> anyhow::Result<BlockNumber> {
    if std::fs::metadata(&env.csv_path).is_err() {
        return Ok(env.orderbookv4_deployment_block);
    }

    let mut csv_reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_path(&env.csv_path)?;
    let saved_trades: Vec<Trade> = csv_reader.deserialize().collect::<Result<_, _>>()?;
    info!("Found {} saved trades", saved_trades.len());
    let latest_trade = saved_trades.last();

    if latest_trade.is_none() {
        return Ok(env.orderbookv4_deployment_block);
    }

    let latest_trade = latest_trade.unwrap();
    info!("Latest saved trade: {latest_trade:?}");

    let latest_trade_tx_hash = latest_trade.tx_hash;
    debug!("Fetching transaction with hash {latest_trade_tx_hash}");
    let tx = orderbook
        .provider()
        .get_transaction_by_hash(latest_trade_tx_hash)
        .await?;

    let start_block = tx
        .map(|tx| tx.block_number)
        .flatten()
        .map(|block_num| block_num + 1)
        .unwrap_or(env.orderbookv4_deployment_block);

    info!("Starting from block {start_block}");
    Ok(start_block)
}

/// A partial trade is a trade that has been parsed from a log event.
#[derive(Debug, Clone)]
pub struct PartialTrade {
    log_index: u64,
    block_number: BlockNumber,
    tx_hash: FixedBytes<32>,
    event: TradeEvent,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TradeEvent {
    ClearV2,
    TakeOrderV2,
}

// impl serde::Serialize for TradeEvent {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: serde::Serializer,
//     {
//         match self {
//             TradeEvent::ClearV2 => serializer.serialize_str("ClearV2"),
//             TradeEvent::TakeOrderV2 => serializer.serialize_str("TakeOrderV2"),
//         }
//     }
// }

// impl<'de> serde::Deserialize<'de> for TradeEvent {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: serde::Deserializer<'de>,
//     {
//         let s = String::deserialize(deserializer)?;
//         match s.as_str() {
//             "ClearV2" => Ok(TradeEvent::ClearV2),
//             "TakeOrderV2" => Ok(TradeEvent::TakeOrderV2),
//             _ => Err(serde::de::Error::custom("Invalid trade event")),
//         }
//     }
// }

/// A trade with all required fields that combines partial trades
/// enriched with block data.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Trade {
    timestamp: u64,
    tx_origin: Address,
    tx_hash: FixedBytes<32>,
    event: TradeEvent,
}

type OrderbookContract = IOrderBookV4::IOrderBookV4Instance<
    http::Http<http::Client>,
    RootProvider<http::Http<http::Client>, AnyNetwork>,
    AnyNetwork,
>;

async fn process_block_batch(
    csv_writer: &mut csv::Writer<std::fs::File>,
    orderbook: &OrderbookContract,
    start_block: u64,
    end_block: u64,
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

    for trade in trades {
        csv_writer.serialize(trade)?;
    }
    csv_writer.flush()?;

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
