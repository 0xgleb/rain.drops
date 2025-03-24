use alloy::network::{AnyHeader, AnyTxEnvelope};
use alloy::primitives::{BlockNumber, FixedBytes};
use alloy::rpc::types::{Block, Header, Transaction};
use std::collections::BTreeMap;

use crate::logs::TradeLog;

#[cfg(test)]
pub mod mock;
pub mod real;

pub(crate) trait OnChain {
    async fn get_block_number(&self) -> anyhow::Result<BlockNumber>;

    async fn get_block_number_by_tx_hash(
        &self,
        tx_hash: FixedBytes<32>,
    ) -> anyhow::Result<Option<BlockNumber>>;

    async fn fetch_clearv2_trades(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> anyhow::Result<BTreeMap<BlockNumber, Vec<TradeLog>>>;

    async fn fetch_takeorderv2_trades(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> anyhow::Result<BTreeMap<BlockNumber, Vec<TradeLog>>>;

    async fn fetch_block_bodies(
        &self,
        block_numbers: impl IntoIterator<Item = BlockNumber>,
    ) -> anyhow::Result<
        BTreeMap<
            BlockNumber,
            Block<Transaction<AnyTxEnvelope>, Header<AnyHeader>>,
        >,
    >;
}
