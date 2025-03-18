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
    #[sol(rpc)]
    interface IUniswapV2Pair {
        #[derive(Debug)]
        event Swap(
            address indexed sender,
            uint amount0In,
            uint amount1In,
            uint amount0Out,
            uint amount1Out,
            address indexed to
        );
    }
}

const DEPLOYMENT_BLOCK: u64 = 12345678;
const BLOCKS_PER_REQ: u64 = 32;

#[derive(Debug)]
enum Side {
    Buy,
    Sell,
}

struct TokenAmount<const DECIMALS: u8 = 18> {
    bucks: U256,
    cents: U256,
}

impl std::fmt::Debug for TokenAmount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.bucks, self.cents)
    }
}

impl<const DECIMALS: u8> From<U256> for TokenAmount<DECIMALS> {
    fn from(value: U256) -> Self {
        let (bucks, cents) = value.div_rem(U256::from(10).pow(U256::from(DECIMALS)));
        Self { bucks, cents }
    }
}

#[derive(Debug)]
struct Trade {
    address: Address,
    yourai: TokenAmount,
    weth: TokenAmount,
    side: Side,
    block_num: u64,
    tx_hash: FixedBytes<32>,
}

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

    let pool = address!("0000000000000000000000000000000000000000");
    let pool = IUniswapV2Pair::new(pool, provider);

    for start_block in (DEPLOYMENT_BLOCK..latest_block).step_by(BLOCKS_PER_REQ as usize) {
        let end_block = start_block + BLOCKS_PER_REQ;
        debug!("Fetching logs from {start_block} to {end_block}");
        let logs = pool.Swap_filter()
            .from_block(start_block)
            .to_block(end_block)
            .query()
            .await?;

        let swaps = logs
            .into_iter()
            .map(
                |(
                    swap,
                    Log {
                        block_number,
                        transaction_hash,
                        ..
                    },
                )| {
                    Trade {
                        address: swap.to,
                        weth: (swap.amount0In + swap.amount0Out).into(),
                        yourai: (swap.amount1In + swap.amount1Out).into(),
                        side: if swap.amount1In == U256::from(0) {
                            Side::Buy
                        } else {
                            Side::Sell
                        },
                        block_num: block_number.unwrap(),
                        tx_hash: transaction_hash.unwrap(),
                    }
                },
            )
            .collect::<Vec<_>>();

        info!("{swaps:#?}");
    }

    Ok(())
}
