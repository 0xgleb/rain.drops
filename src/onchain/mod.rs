//! A layer of abstraction for controlling interactions with the blockchain
//! depending on whether we are running in a test environment or not.

use alloy::primitives::{Address, BlockNumber, FixedBytes};
use std::collections::BTreeMap;

use crate::logs::TradeLog;

#[cfg(test)]
pub mod mock;
pub mod real;

/// Simplified block representation that only includes metadata relevant to us.
/// This helps with auto-generating test data.
#[derive(Debug, Clone)]
pub(crate) struct BlockMetadata {
    pub timestamp: u64,
    pub transactions: Vec<TxMetadata>,
}

/// Simplified transaction representation that only includes relevant metadata.
/// This helps with auto-generating test data.
#[derive(Debug, Clone)]
pub(crate) struct TxMetadata {
    pub origin: Address,
    pub hash: FixedBytes<32>,
}

/// A trait for interacting with the blockchain and deployed orderbook contract.
pub(crate) trait OnChain {
    /// Get the current block number.
    async fn get_block_number(&self) -> anyhow::Result<BlockNumber>;

    /// Get the block number in which a transaction with the given hash was
    /// included.
    async fn get_block_number_by_tx_hash(
        &self,
        tx_hash: FixedBytes<32>,
    ) -> anyhow::Result<Option<BlockNumber>>;

    /// Fetch all ClearV2 trades from the given block range.
    async fn fetch_clearv2_trades(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> anyhow::Result<BTreeMap<BlockNumber, Vec<TradeLog>>>;

    /// Fetch all TakeOrderV2 trades from the given block range.
    async fn fetch_takeorderv2_trades(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> anyhow::Result<BTreeMap<BlockNumber, Vec<TradeLog>>>;

    /// Fetch block bodies for a sequence of block numbers.
    async fn fetch_block_bodies(
        &self,
        block_numbers: impl IntoIterator<Item = BlockNumber>,
    ) -> anyhow::Result<BTreeMap<BlockNumber, BlockMetadata>>;
}
