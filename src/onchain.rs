use std::collections::BTreeMap;

use alloy::network::{AnyHeader, AnyTxEnvelope};
use alloy::primitives::BlockNumber;
use alloy::rpc::types::{Block, Header, Transaction};

use crate::{logs::TradeLog, OrderbookContract};

pub(crate) trait OnChain {
    async fn get_block_number(&self) -> anyhow::Result<BlockNumber>;

    async fn get_block_by_number(
        &self,
        block_number: BlockNumber,
    ) -> anyhow::Result<Block<Transaction<AnyTxEnvelope>, Header<AnyHeader>>>;

    async fn fetch_clearv2_trades(
        start_block: u64,
        end_block: u64,
        orderbook: &OrderbookContract,
    ) -> anyhow::Result<BTreeMap<BlockNumber, Vec<TradeLog>>>;

    async fn fetch_takeorderv2_trades(
        start_block: u64,
        end_block: u64,
        orderbook: &OrderbookContract,
    ) -> anyhow::Result<BTreeMap<BlockNumber, Vec<TradeLog>>>;
}
