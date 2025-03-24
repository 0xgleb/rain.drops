//! A CLI tool for fetching and parsing OrderbookV4 event logs from the
//! blockchain and saving them to a CSV file.

use alloy::network::{AnyNetwork, BlockResponse, TransactionResponse};
use alloy::primitives::{Address, BlockNumber, FixedBytes};
use alloy::providers::RootProvider;
use alloy::{sol, transports::http};
use itertools::Itertools;
use tracing::*;

sol! {
    #[sol(rpc)]
    IOrderBookV4, "./abi/orderbookv4.json"
}

pub mod env;
mod logs;
pub mod onchain;

use logs::{TradeEvent, TradeLog};
use onchain::OnChain;

/// Type alias for the OrderbookV4 contract instance connected to the
/// configured JSON-RPC HTTP URL.
pub type OrderbookContract = IOrderBookV4::IOrderBookV4Instance<
    http::Http<http::Client>,
    RootProvider<http::Http<http::Client>, AnyNetwork>,
    AnyNetwork,
>;

/// Create or append to a CSV file containing all trades from the deployed
/// OrderbookV4 contract.
#[allow(private_bounds)]
pub async fn update_trades_csv(
    env: &env::Env,
    onchain: &impl OnChain,
) -> anyhow::Result<()> {
    let file_exists = std::fs::metadata(&env.csv_path).is_ok();
    debug!("Does {} exist? {}", env.csv_path, file_exists);

    let start_block = get_start_block(env, onchain).await?;
    info!("Starting trade collection from block {start_block}");
    let latest_block = onchain.get_block_number().await?;
    info!("Latest block is {latest_block}");

    let csv_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&env.csv_path)
        .unwrap();

    let mut csv_writer =
        csv::WriterBuilder::new().has_headers(false).from_writer(csv_file);

    if !file_exists {
        csv_writer.write_record([
            "timestamp",
            "tx_origin",
            "tx_hash",
            "event",
        ])?;
    }

    debug!("Set up CSV writer for {}", env.csv_path);

    info!("Fetching trades from blocks {start_block} to {latest_block}");
    for block_batch_start in
        (start_block..latest_block).step_by(env.blocks_per_log_request as usize)
    {
        let block_batch_end = block_batch_start + env.blocks_per_log_request;
        process_block_batch(
            &mut csv_writer,
            onchain,
            block_batch_start,
            block_batch_end,
        )
        .await?;
    }

    Ok(())
}

async fn read_trades_csv(env: &env::Env) -> anyhow::Result<Vec<Trade>> {
    let mut csv_reader =
        csv::ReaderBuilder::new().has_headers(true).from_path(&env.csv_path)?;
    let saved_trades: Vec<Trade> =
        csv_reader.deserialize().collect::<Result<_, _>>()?;
    info!("Found {} saved trades", saved_trades.len());
    Ok(saved_trades)
}

/// Determine the starting block for fetching event logs from.
async fn get_start_block(
    env: &env::Env,
    onchain: &impl OnChain,
) -> anyhow::Result<BlockNumber> {
    if std::fs::metadata(&env.csv_path).is_err() {
        return Ok(env.orderbookv4_deployment_block);
    }

    let saved_trades = read_trades_csv(env).await?;
    let latest_trade = saved_trades.last();
    if latest_trade.is_none() {
        return Ok(env.orderbookv4_deployment_block);
    }

    let latest_trade = latest_trade.unwrap();
    debug!("Latest saved trade: {latest_trade:?}");

    let latest_trade_tx_hash = latest_trade.tx_hash;
    debug!("Fetching transaction with hash {latest_trade_tx_hash}");
    let start_block = onchain
        .get_block_number_by_tx_hash(latest_trade_tx_hash)
        .await?
        .unwrap_or(env.orderbookv4_deployment_block);

    Ok(start_block)
}

/// A trade with all required fields that combines partial trades
/// enriched with block data.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Trade {
    timestamp: u64,
    tx_origin: Address,
    tx_hash: FixedBytes<32>,
    event: TradeEvent,
}

/// Collect and store a batch of trade logs from the given block range.
async fn process_block_batch(
    csv_writer: &mut csv::Writer<std::fs::File>,
    onchain: &impl OnChain,
    start_block: u64,
    end_block: u64,
) -> anyhow::Result<()> {
    debug!("Fetching a batch of trade logs from blocks {start_block} to {end_block}");

    let mut clearv2_trades =
        onchain.fetch_clearv2_trades(start_block, end_block).await?;

    let clearv2_trades_count: usize =
        clearv2_trades.values().map(|trades| trades.len()).sum();
    debug!("Blocks [{start_block}, {end_block}] emitted {clearv2_trades_count} ClearV2 events");

    let mut takeorderv2_trades =
        onchain.fetch_takeorderv2_trades(start_block, end_block).await?;

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

    let mut block_bodies =
        onchain.fetch_block_bodies(blocks_with_trades.clone()).await?;

    let trades = blocks_with_trades
        .into_iter()
        .flat_map(|block_number| {
            let clearv2_trade =
                clearv2_trades.remove(&block_number).unwrap_or_default();
            let takeorderv2_trade =
                takeorderv2_trades.remove(&block_number).unwrap_or_default();

            clearv2_trade
                .into_iter()
                .chain(takeorderv2_trade.into_iter())
                .sorted_by_key(|trade| trade.log_index)
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

    let trade_count = trades.len();
    info!("Collected {trade_count:>2} trades from blocks [{start_block}, {end_block}]");

    #[cfg(debug_assertions)]
    assert_eq!(trade_count, clearv2_trades_count + takeorderv2_trades_count);

    for trade in trades {
        csv_writer.serialize(trade)?;
    }
    csv_writer.flush()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use env::Env;
    use onchain::mock::MockChain;

    #[tokio::test]
    async fn test_get_start_block() -> anyhow::Result<()> {
        let mut env = Env::init();
        env.csv_path = "test_trades.csv".to_string();
        env.json_rpc_http_url =
            std::env::var("ARBITRUM_JSON_RPC_HTTP_URL").unwrap();

        // fake deployment block to speed up the test
        env.orderbookv4_deployment_block = 267_500_000;

        let current_block: BlockNumber = 267_750_000;
        let orderbook = env.connect_contract()?;
        let mut onchain = MockChain::new(current_block, orderbook);

        if std::fs::metadata(&env.csv_path).is_ok() {
            std::fs::remove_file(&env.csv_path)?;
        }

        update_trades_csv(&env, &onchain).await?;
        assert!(std::fs::metadata(&env.csv_path).is_ok());

        let saved_trades = read_trades_csv(&env).await?;
        assert_eq!(saved_trades.len(), 17);

        let clearv2_trade_count = saved_trades
            .iter()
            .filter(|trade| trade.event == TradeEvent::ClearV2)
            .count();
        assert_eq!(clearv2_trade_count, 1);

        let takeorderv2_trade_count = saved_trades
            .iter()
            .filter(|trade| trade.event == TradeEvent::TakeOrderV2)
            .count();
        assert_eq!(takeorderv2_trade_count, 16);

        let current_block: BlockNumber = 268_000_000;
        onchain.set_current_block(current_block);
        update_trades_csv(&env, &onchain).await?;

        let saved_trades = read_trades_csv(&env).await?;
        assert_eq!(saved_trades.len(), 32);

        let clearv2_trade_count = saved_trades
            .iter()
            .filter(|trade| trade.event == TradeEvent::ClearV2)
            .count();
        assert_eq!(clearv2_trade_count, 1);

        let takeorderv2_trade_count = saved_trades
            .iter()
            .filter(|trade| trade.event == TradeEvent::TakeOrderV2)
            .count();
        assert_eq!(takeorderv2_trade_count, 31);

        std::fs::remove_file(&env.csv_path)?;

        Ok(())
    }
}
