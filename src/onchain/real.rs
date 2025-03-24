//! A real implementation of the [`OnChain`] trait that interacts with the
//! blockchain.

use alloy::eips::BlockNumberOrTag;
use alloy::network::{AnyHeader, AnyTxEnvelope};
use alloy::primitives::{BlockNumber, FixedBytes};
use alloy::providers::Provider;
use alloy::rpc::types::{
    Block, BlockTransactions, BlockTransactionsKind, Header, Transaction,
};
use itertools::Itertools;
use std::collections::BTreeMap;
use tracing::*;

use super::OnChain;
use crate::{OrderbookContract, TradeLog};

/// A wrapper around the connected orderbook contract that implements the
/// [`OnChain`] trait.
pub struct RealChain {
    contract: OrderbookContract,
}

impl RealChain {
    /// Create a new [`RealChain`] wrapper around the given orderbook
    /// contract.
    pub fn new(contract: OrderbookContract) -> Self {
        Self { contract }
    }
}

impl OnChain for RealChain {
    async fn get_block_number(&self) -> anyhow::Result<BlockNumber> {
        Ok(self.contract.provider().get_block_number().await?)
    }

    async fn get_block_number_by_tx_hash(
        &self,
        tx_hash: FixedBytes<32>,
    ) -> anyhow::Result<Option<BlockNumber>> {
        let tx =
            self.contract.provider().get_transaction_by_hash(tx_hash).await?;

        let block_number =
            tx.and_then(|tx| tx.block_number).map(|block_num| block_num + 1);

        Ok(block_number)
    }

    async fn fetch_clearv2_trades(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> anyhow::Result<BTreeMap<BlockNumber, Vec<TradeLog>>> {
        debug!(
            "Fetching ClearV2 trades from blocks {start_block} to {end_block}"
        );
        crate::logs::fetch_clearv2_trades(
            start_block,
            end_block,
            &self.contract,
        )
        .await
    }

    async fn fetch_takeorderv2_trades(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> anyhow::Result<BTreeMap<BlockNumber, Vec<TradeLog>>> {
        debug!(
            "Fetching TakeOrderV2 trades from blocks {start_block} to {end_block}"
        );
        crate::logs::fetch_takeorderv2_trades(
            start_block,
            end_block,
            &self.contract,
        )
        .await
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
        debug!("Fetching block bodies for blocks {block_numbers:?}");
        let mut block_bodies = BTreeMap::new();

        for block_number in block_numbers {
            trace!("Fetching block #{block_number}");
            let block = self
                .contract
                .provider()
                .get_block_by_number(
                    BlockNumberOrTag::Number(block_number),
                    BlockTransactionsKind::Full,
                )
                .await?;

            match block {
                None => {
                    error!(
                        "Get block with number {block_number} returned None"
                    );
                    continue;
                }
                Some(block) => {
                    let Block { header, uncles, transactions, withdrawals } =
                        block.inner;
                    let transactions = transactions
                        .into_transactions()
                        .map(|tx| tx.inner)
                        .collect_vec();
                    let transactions = BlockTransactions::Full(transactions);
                    let block =
                        Block { header, uncles, transactions, withdrawals };

                    block_bodies.insert(block_number, block);
                }
            }
        }

        Ok(block_bodies)
    }
}
