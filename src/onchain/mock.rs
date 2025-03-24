//! A mock implementation of the [`OnChain`] trait that allows for
//! deterministic testing by mocking the current block number.

use alloy::network::{AnyHeader, AnyTxEnvelope};
use alloy::primitives::{BlockNumber, FixedBytes};
use alloy::rpc::types::{Block, Header, Transaction};
use std::collections::BTreeMap;

use super::real::RealChain;
use super::OnChain;
use crate::logs::TradeLog;
use crate::OrderbookContract;

/// A wrapper around the real chain that allows for mocking the block number
/// for deterministic testing
pub(crate) struct MockChain {
    current_block: BlockNumber,
    real_chain: RealChain,
}

impl MockChain {
    /// Create a new [`MockChain`] wrapper around the given orderbook
    /// contract.
    pub(crate) fn new(
        current_block: BlockNumber,
        orderbook_contract: OrderbookContract,
    ) -> Self {
        Self { current_block, real_chain: RealChain::new(orderbook_contract) }
    }

    /// Set the current block number.
    pub(crate) fn set_current_block(&mut self, block_number: BlockNumber) {
        self.current_block = block_number;
    }
}

impl OnChain for MockChain {
    async fn get_block_number(&self) -> anyhow::Result<BlockNumber> {
        Ok(self.current_block)
    }

    async fn get_block_number_by_tx_hash(
        &self,
        tx_hash: FixedBytes<32>,
    ) -> anyhow::Result<Option<BlockNumber>> {
        self.real_chain.get_block_number_by_tx_hash(tx_hash).await
    }

    async fn fetch_clearv2_trades(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> anyhow::Result<BTreeMap<BlockNumber, Vec<TradeLog>>> {
        self.real_chain.fetch_clearv2_trades(start_block, end_block).await
    }

    async fn fetch_takeorderv2_trades(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> anyhow::Result<BTreeMap<BlockNumber, Vec<TradeLog>>> {
        self.real_chain.fetch_takeorderv2_trades(start_block, end_block).await
    }

    async fn fetch_block_bodies(
        &self,
        block_numbers: Vec<BlockNumber>,
    ) -> anyhow::Result<
        BTreeMap<
            BlockNumber,
            Block<Transaction<AnyTxEnvelope>, Header<AnyHeader>>,
        >,
    > {
        self.real_chain.fetch_block_bodies(block_numbers).await
    }
}
