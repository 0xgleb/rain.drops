use alloy::primitives::BlockNumber;
use itertools::Itertools;
use std::collections::BTreeMap;
use tracing::*;

use crate::logs::TradeLog;
use crate::onchain::BlockMetadata;
use crate::Trade;

pub(crate) fn enrich_and_merge(
    mut clearv2_trades: BTreeMap<BlockNumber, Vec<TradeLog>>,
    mut takeorderv2_trades: BTreeMap<BlockNumber, Vec<TradeLog>>,
    block_bodies: BTreeMap<BlockNumber, BlockMetadata>,
) -> Vec<Trade> {
    let blocks_with_trades = clearv2_trades
        .keys()
        .copied()
        .chain(takeorderv2_trades.keys().copied())
        .sorted()
        .collect_vec();

    if blocks_with_trades.is_empty() {
        return vec![];
    }

    let start_block = blocks_with_trades[0];
    let end_block = blocks_with_trades[blocks_with_trades.len() - 1];

    let clearv2_trades_count: usize =
        clearv2_trades.values().map(|trades| trades.len()).sum();
    debug!("Blocks [{start_block}, {end_block}] emitted {clearv2_trades_count} ClearV2 events");

    let takeorderv2_trades_count: usize =
        takeorderv2_trades.values().map(|trades| trades.len()).sum();
    debug!(
        "Blocks [{start_block}, {end_block}] emitted {takeorderv2_trades_count} TakeOrderV2 events"
    );

    let trades = blocks_with_trades
        .into_iter()
        .flat_map(|block_number| {
            let clearv2_trade =
                clearv2_trades.remove(&block_number).unwrap_or_default();
            let takeorderv2_trade =
                takeorderv2_trades.remove(&block_number).unwrap_or_default();

            clearv2_trade
                .into_iter()
                .chain(takeorderv2_trade)
                .sorted_by_key(|trade| trade.log_index)
        })
        .map(|trade| {
            let BlockMetadata { timestamp, transactions } =
                block_bodies.get(&trade.block_number).unwrap().to_owned();

            let tx_origin = transactions
                .into_iter()
                .find_map(|tx| {
                    if tx.hash == trade.tx_hash {
                        Some(tx.origin)
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

    trades
}

#[cfg(test)]
mod tests {
    use alloy::{
        hex::FromHex,
        primitives::{Address, FixedBytes, TxHash},
    };
    use proptest::prelude::*;

    use super::*;
    use crate::{onchain::TxMetadata, TradeEvent};

    proptest! {
        #[test]
        fn test_enrich_and_merge(
            (clearv2_trades, takeorderv2_trades, block_bodies) in arb_enrich_and_merge_args()
        ) {
            println!("clearv2_trades: {:?}", clearv2_trades);
            println!("takeorderv2_trades: {:?}", takeorderv2_trades);
            println!("block_bodies: {:?}", block_bodies);
        }
    }

    fn arb_enrich_and_merge_args() -> impl Strategy<
        Value = (
            BTreeMap<BlockNumber, Vec<TradeLog>>,
            BTreeMap<BlockNumber, Vec<TradeLog>>,
            BTreeMap<BlockNumber, BlockMetadata>,
        ),
    > {
        arb_trade_logs_and_hashes().prop_flat_map(
            |(clearv2_trades, takeorderv2_trades, block_num_to_tx_hashes)| {
                // Create a strategy for the block metadata map
                let block_numbers: Vec<BlockNumber> =
                    block_num_to_tx_hashes.keys().copied().collect();
                let block_metadata_strategy = prop::collection::btree_map(
                    Just(block_numbers),
                    arb_block_metadata(
                        block_num_to_tx_hashes.values().next().unwrap().clone(),
                    ),
                    1..=block_num_to_tx_hashes.len(),
                )
                .prop_map(|map| {
                    map.into_iter()
                        .map(|(k, v)| (k[0], v))
                        .collect::<BTreeMap<_, _>>()
                });

                // Combine the trade logs with the block metadata
                (
                    Just(clearv2_trades),
                    Just(takeorderv2_trades),
                    block_metadata_strategy,
                )
                    .prop_map(
                        |(clearv2, takeorderv2, blocks)| {
                            (clearv2, takeorderv2, blocks)
                        },
                    )
            },
        )
    }

    prop_compose! {
        fn arb_trade_logs_and_hashes()(
            clearv2_logs in arb_trade_logs(TradeEvent::ClearV2),
            takeorderv2_logs in arb_trade_logs(TradeEvent::TakeOrderV2)
        ) -> (
            BTreeMap<BlockNumber, Vec<TradeLog>>,
            BTreeMap<BlockNumber, Vec<TradeLog>>,
            BTreeMap<BlockNumber, Vec<TxHash>>,
        ) {
            let mut block_num_to_tx_hashes =
                BTreeMap::<BlockNumber, Vec<(u64, TxHash)>>::new();

            for log in
                clearv2_logs.clone().into_iter().chain(takeorderv2_logs.clone())
            {
                block_num_to_tx_hashes
                    .entry(log.block_number)
                    .and_modify(|hashes| hashes.push((log.log_index, log.tx_hash)))
                    .or_insert(vec![(log.log_index, log.tx_hash)]);
            }

            let clearv2_trades: BTreeMap<BlockNumber, Vec<TradeLog>> = clearv2_logs
                .into_iter()
                .map(|log| (log.block_number, vec![log]))
                .collect();

            let takeorderv2_trades: BTreeMap<BlockNumber, Vec<TradeLog>> =
                takeorderv2_logs
                    .into_iter()
                    .map(|log| (log.block_number, vec![log]))
                    .collect();

            let block_num_to_tx_hashes = block_num_to_tx_hashes
                .into_iter()
                .map(|(block_number, hashes_with_indices)| {
                    let sorted_hashes = hashes_with_indices
                        .into_iter()
                        .sorted_by_key(|(index, _)| *index)
                        .map(|(_, hash)| hash)
                        .collect_vec();
                    (block_number, sorted_hashes)
                })
                .collect::<BTreeMap<_, _>>();

            (clearv2_trades, takeorderv2_trades, block_num_to_tx_hashes)
        }
    }

    prop_compose! {
        fn arb_block_metadata(required_tx_hashes: Vec<TxHash>)(
            timestamp in 0u64..10000,
            transactions in arb_transactions(required_tx_hashes)
        ) -> BlockMetadata {
            BlockMetadata { timestamp, transactions }
        }
    }

    fn arb_transactions(
        required_tx_hashes: Vec<TxHash>,
    ) -> impl Strategy<Value = Vec<TxMetadata>> {
        // First generate the required transactions using arb_tx_metadata_from_hash
        let required_txs = required_tx_hashes
            .into_iter()
            .map(arb_tx_metadata_from_hash)
            .collect::<Vec<_>>();

        // Then generate some arbitrary transactions
        let arbitrary_txs = prop::collection::vec(arb_tx_metadata(), 0..5);

        // Combine them using prop_flat_map
        (required_txs, arbitrary_txs).prop_flat_map(|(required, arbitrary)| {
            // Combine the required and arbitrary transactions
            let mut all_txs = required;
            all_txs.extend(arbitrary);
            Just(all_txs)
        })
    }

    fn arb_trade_logs(
        event: TradeEvent,
    ) -> impl Strategy<Value = Vec<TradeLog>> {
        prop::collection::vec(arb_trade_log(event), 1..11)
    }

    prop_compose! {
        fn arb_trade_log(event: TradeEvent)(
            log_index in 0u64..1000,
            block_number in 0u64..10000,
            tx_hash in arb_tx_hash(),
        ) -> TradeLog {
            TradeLog { log_index, block_number, tx_hash, event: event.clone() }
        }
    }

    prop_compose! {
        fn arb_tx_metadata_from_hash(hash: TxHash)(
            origin in arb_address()
        ) -> TxMetadata {
            TxMetadata { hash, origin }
        }
    }

    prop_compose! {
        fn arb_tx_metadata()(
            hash in arb_tx_hash(),
            origin in arb_address()
        ) -> TxMetadata {
            TxMetadata { hash, origin }
        }
    }

    prop_compose! {
        fn arb_tx_hash()(hash in "0x[a-f0-9]{64}") -> FixedBytes<32> {
            FixedBytes::from_hex(hash).unwrap()
        }
    }

    prop_compose! {
        fn arb_address()(address in "0x[a-f0-9]{40}") -> Address {
            Address::from_hex(address).unwrap()
        }
    }
}
