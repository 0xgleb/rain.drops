//! A layer of abstraction for controlling interactions with the blockchain
//! depending on whether we are running in a test environment or not.

use alloy::network::{AnyHeader, AnyTxEnvelope};
use alloy::primitives::{BlockNumber, FixedBytes};
use alloy::rpc::types::{Block, Header, Transaction};
use std::collections::BTreeMap;

use crate::logs::TradeLog;

#[cfg(test)]
pub mod mock;
pub mod real;

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
        block_numbers: Vec<BlockNumber>,
    ) -> anyhow::Result<
        BTreeMap<
            BlockNumber,
            Block<Transaction<AnyTxEnvelope>, Header<AnyHeader>>,
        >,
    >;
}
