# Ore Pool Cli
 Ore V2 Pool client coded in Rust. Fork of Ore Cli modified for pools

### Default Ore Mining Pool
Alvarium Mining Pool is operated by Bifrost and is the default pool option for the mining client.

Alvarium is currently offline and in testing phase until reward distribution is fully tested.

### Custom Ore Mining Pool
There is no open source mining pool API template. A custom API server has to be built in order to use this client

## Modify Settings
Navigate to main.rs in the src folder and update the constants below
```
//Default is Alvarium Mining Pool. You can replace with a different mining pool address
pub const MINING_POOL: Pubkey = solana_program::pubkey!("Cdh9QF6NmxCxWDEmuusFVkhQSZuVMRXj9nnZQyGraCna");

//MUST BE CHANGED TO RECEIVE PAYOUT. Use your wallet address here
pub const MINER_PAYOUT_ADDRESS: Pubkey = solana_program::pubkey!("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");

//Replace with your favorite RPC provider
pub const MINING_POOL_RPC: &str = "REPLACE WITH RPC PROVIDER";

//Default is Alvarium Mining Pool. Change this to your pool's API endpoint
pub const MINING_POOL_URL: &str = "https://mining.bifrost.technology/submitwork";

//Update amount of threads
pub const THREADS: u64 = 50;

//Keep at 5 for Alvarium Mining Pool. Change this value if the pool requires you to hand in the work sooner
pub const POOL_BUFFER: u64 = 5;
```

## Build

To build the codebase from scratch, checkout the repo and use cargo to build:


```
cargo build --release
```

