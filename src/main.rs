use alloy::primitives::{address, Address, FixedBytes, U256};
use alloy::providers::{Provider, ProviderBuilder};
use alloy::rpc::types::Log;
use alloy::sol;
use clap::Parser;
use tracing::*;

#[derive(Debug, Parser)]
struct Env {
    #[clap(long, env)]
    json_rpc_http_url: String,
    #[clap(long, env, default_value = "INFO")]
    log_level: Level,
}

sol! {
    /// Used in struct SignedContextV1.
    ///
    /// The chain of imports leads to SignedContextV1 that is deinfed in IInterpreterCallerV2.sol
    /// See the source here:
    /// https://github.com/rainlanguage/rain.interpreter.interface/blob/09352384381d1e3ab1118576f5656c911252f0d6/src/interface/deprecated/IInterpreterCallerV2.sol#L62
    #[derive(Debug)]
    struct SignedContextV1 {
        // The ordering of these fields is important and used in assembly offset
        // calculations and hashing.
        address signer;
        bytes32[] context;
        bytes signature;
    }

    /// Used in event TakeOrderV2.
    ///
    /// Definition copied from
    /// https://github.com/rainlanguage/rain.orderbook.interface/blob/79a9086c618387f61c42d35ff2adb351c47b9547/src/interface/IOrderBookV4.sol#L56
    #[derive(Debug)]
    struct TakeOrderConfigV3 {
        OrderV3 order;
        uint256 inputIOIndex;
        uint256 outputIOIndex;
        SignedContextV1[] signedContext;
    }

    /// Used in struct OrderV3.
    ///
    /// Definition copied from
    /// https://github.com/rainlanguage/rain.interpreter.interface/blob/0247f7e27df7097bf0d6ea25b086925b4c2747d2/src/interface/IInterpreterCallerV3.sol#L20
    #[derive(Debug)]
    struct EvaluableV3 {
        // NOTE: Compilation fails for the interface, so we use the address instead.
        // IInterpreterV3 interpreter;
        address interpreter;
        // NOTE: Compilation fails for the interface, so we use the address instead.
        // IInterpreterStoreV2 store;
        address store;
        bytes bytecode;
    }

    /// Used in struct OrderV3.
    ///
    /// Definition (following the chain of imports leading to V2) was copied from
    /// https://github.com/rainlanguage/rain.orderbook.interface/blob/79a9086c618387f61c42d35ff2adb351c47b9547/src/interface/deprecated/v2/IOrderBookV2.sol#L49
    #[derive(Debug)]
    struct IO {
        address token;
        uint8 decimals;
        uint256 vaultId;
    }

    /// Used in event ClearV2.
    ///
    /// Definition copied from
    /// https://github.com/rainlanguage/rain.orderbook.interface/blob/79a9086c618387f61c42d35ff2adb351c47b9547/src/interface/IOrderBookV4.sol#L79
    #[derive(Debug)]
    struct OrderV3 {
        address owner;
        EvaluableV3 evaluable;
        IO[] validInputs;
        IO[] validOutputs;
        bytes32 nonce;
    }

    /// Used in event ClearV2.
    ///
    /// Definition copied from
    /// https://github.com/rainlanguage/rain.orderbook.interface/blob/79a9086c618387f61c42d35ff2adb351c47b9547/src/interface/deprecated/v2/IOrderBookV2.sol#L143
    #[derive(Debug)]
    struct ClearConfig {
        uint256 aliceInputIOIndex;
        uint256 aliceOutputIOIndex;
        uint256 bobInputIOIndex;
        uint256 bobOutputIOIndex;
        uint256 aliceBountyVaultId;
        uint256 bobBountyVaultId;
    }

    #[sol(rpc)]
    interface IOrderBookV4 {
        /// Some order has been taken by `msg.sender`. This is the same as them
        /// placing inverse orders then immediately clearing them all, but costs less
        /// gas and is more convenient and reliable. Analogous to a market buy
        /// against the specified orders. Each order that is matched within a the
        /// `takeOrders` loop emits its own individual event.
        /// @param sender `msg.sender` taking the orders.
        /// @param config All config defining the orders to attempt to take.
        /// @param input The input amount from the perspective of sender.
        /// @param output The output amount from the perspective of sender.
        #[derive(Debug)]
        event TakeOrderV2(address sender, TakeOrderConfigV3 config, uint256 input, uint256 output);

        /// Emitted before two orders clear. Covers both orders and includes all the
        /// state before anything is calculated.
        /// @param sender `msg.sender` clearing both orders.
        /// @param alice One of the orders.
        /// @param bob The other order.
        /// @param clearConfig Additional config required to process the clearance.
        #[derive(Debug)]
        event ClearV2(address sender, OrderV3 alice, OrderV3 bob, ClearConfig clearConfig);
    }
}

const DEPLOYMENT_BLOCK: u64 = 12345678;
const BLOCKS_PER_REQ: u64 = 32;

// #[derive(Debug)]
// enum Side {
//     Buy,
//     Sell,
// }

// struct TokenAmount<const DECIMALS: u8 = 18> {
//     bucks: U256,
//     cents: U256,
// }

// impl std::fmt::Debug for TokenAmount {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{}.{}", self.bucks, self.cents)
//     }
// }

// impl<const DECIMALS: u8> From<U256> for TokenAmount<DECIMALS> {
//     fn from(value: U256) -> Self {
//         let (bucks, cents) = value.div_rem(U256::from(10).pow(U256::from(DECIMALS)));
//         Self { bucks, cents }
//     }
// }

// #[derive(Debug)]
// struct Trade {
//     address: Address,
//     yourai: TokenAmount,
//     weth: TokenAmount,
//     side: Side,
//     block_num: u64,
//     tx_hash: FixedBytes<32>,
// }

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    let env = Env::parse();
    tracing_subscriber::fmt()
        .with_max_level(env.log_level)
        .init();

    let rpc_url = env.json_rpc_http_url.parse()?;
    let provider = ProviderBuilder::new().on_http(rpc_url);
    let latest_block = provider.get_block_number().await?;

    info!("Latest block is {latest_block}");

    // let pool = address!("0000000000000000000000000000000000000000");
    // let pool = IUniswapV2Pair::new(pool, provider);

    // for start_block in (DEPLOYMENT_BLOCK..latest_block).step_by(BLOCKS_PER_REQ as usize) {
    //     let end_block = start_block + BLOCKS_PER_REQ;
    //     debug!("Fetching logs from {start_block} to {end_block}");
    //     let logs = pool.Swap_filter()
    //         .from_block(start_block)
    //         .to_block(end_block)
    //         .query()
    //         .await?;

    //     let swaps = logs
    //         .into_iter()
    //         .map(
    //             |(
    //                 swap,
    //                 Log {
    //                     block_number,
    //                     transaction_hash,
    //                     ..
    //                 },
    //             )| {
    //                 Trade {
    //                     address: swap.to,
    //                     weth: (swap.amount0In + swap.amount0Out).into(),
    //                     yourai: (swap.amount1In + swap.amount1Out).into(),
    //                     side: if swap.amount1In == U256::from(0) {
    //                         Side::Buy
    //                     } else {
    //                         Side::Sell
    //                     },
    //                     block_num: block_number.unwrap(),
    //                     tx_hash: transaction_hash.unwrap(),
    //                 }
    //             },
    //         )
    //         .collect::<Vec<_>>();

    //     info!("{swaps:#?}");
    // }

    Ok(())
}
