# rain.drops

Raindex trade-level data collection pipeline.

This is a CLI tool that fetches and saves trades to a CSV file. If a file already exists, it will start from the last processed block and continue until the current block, appending to the file.

## Prerequisites

Install Nix

``` sh
curl --proto '=https' --tlsv1.2 -sSf -L https://install.determinate.systems/nix | sh -s -- install
```

For automatic shell configuration, install Direnv

``` sh
nix -v flake install nixpkgs#direnv
```

Hook Direnv to your shell, e.g. 

``` sh
# For bash
echo 'eval "$(direnv hook bash)"' >> ~/.bashrc
source ~/.bashrc

# For zsh
echo 'eval "$(direnv hook zsh)"' >> ~/.zshrc
source ~/.zshrc
```

Enable direnv for the local copy of the repo

``` sh
direnv allow
```

Alternatively, you can enter the dev environment manually using

``` sh
nix develop
```

## Running the CLI tool

Copy the `.env.example` file to `.env` and set the environment variables.

``` sh
cp .env.example .env
```

Run the CLI tool

``` sh
cargo run
```

You can find all configuration options by running

``` sh
cargo run -- --help
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.26s
     Running `target/debug/rain-drops --help`
Configuration options for the CLI tool.

The options can be set by environment variables or command line arguments.

Usage: rain-drops [OPTIONS] --json-rpc-http-url <JSON_RPC_HTTP_URL> --orderbookv4-deployment-address <ORDERBOOKV4_DEPLOYMENT_ADDRESS> --orderbookv4-deployment-block <ORDERBOOKV4_DEPLOYMENT_BLOCK>

Options:
      --log-level <LOG_LEVEL>
          The log level to use
          
          [env: LOG_LEVEL=]
          [default: DEBUG]

      --csv-path <CSV_PATH>
          The path to the CSV file to read/write trades to/from
          
          [env: CSV_PATH=]
          [default: trades.csv]

      --json-rpc-http-url <JSON_RPC_HTTP_URL>
          The URL of the JSON-RPC HTTP endpoint to use
          
          [env: JSON_RPC_HTTP_URL=]

      --orderbookv4-deployment-address <ORDERBOOKV4_DEPLOYMENT_ADDRESS>
          The address of the deployed OrderbookV4 contract

          [env: ORDERBOOKV4_DEPLOYMENT_ADDRESS=0x550878091b2B1506069F61ae59e3A5484Bca9166]

      --orderbookv4-deployment-block <ORDERBOOKV4_DEPLOYMENT_BLOCK>
          The block number when the OrderbookV4 contract was deployed
          
          [env: ORDERBOOKV4_DEPLOYMENT_BLOCK=267576000]

      --blocks-per-log-request <BLOCKS_PER_LOG_REQUEST>
          The number of blocks to fetch event logs from at a time
          
          [env: BLOCKS_PER_LOG_REQUEST=]
          [default: 100000]

  -h, --help
          Print help (see a summary with '-h')
```

