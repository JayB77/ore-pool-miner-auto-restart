mod busses;
mod open;
mod utils;
use colored::*;
use drillx::{
    equix::{self, SolutionArray},
    seed, DrillxError, Hash, Solution,
};
use ore_api::{
    consts::{BUS_ADDRESSES, BUS_COUNT, EPOCH_DURATION},
    state::{Config, Proof},
};
use rand::rngs::OsRng;
use rand::Rng;
use reqwest::Client;
use serde::Deserialize;
use sha3::Digest;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;
use solana_rpc_client::spinner;
use std::{env, time::Duration};
use std::{sync::Arc, time::Instant};
use std::process::Command;

use crate::open::open;
use crate::utils::{amount_u64_to_string, get_clock, get_config, get_updated_proof_with_authority};

// Default is Alvarium Mining Pool. You can replace with a different mining pool address
pub const MINING_POOL: Pubkey =
    solana_program::pubkey!("Cdh9QF6NmxCxWDEmuusFVkhQSZuVMRXj9nnZQyGraCna");

// Default is Alvarium Mining Pool. Change this to your pool's API endpoint
pub const MINING_POOL_URL: &str = "https://alvarium.bifrost.technology/submitwork";
pub static mut DEBUG: bool = false;

#[tokio::main]
async fn main() {
    let mut miner_rpc: String = String::new();
    let mut miner_address: Pubkey =
        solana_program::pubkey!("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");

    let mut threads: u64 = 50;
    let mut _buffer: u64 = 8;
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        if let Ok(value) = args[1].parse::<String>() {
            miner_rpc = value;
        }
    }
    if args.len() > 2 {
        if let Ok(value) = args[2].parse() {
            miner_address = value;
        }
    }
    if args.len() > 3 {
        if let Ok(value) = args[3].parse::<u64>() {
            threads = value;
        }
    }
    if args.len() > 4 {
        if let Ok(value) = args[4].parse::<u64>() {
            if _buffer >= 11 {
                _buffer = 10;
            } else if _buffer <= 6 {
                _buffer = 7;
            } else {
                _buffer = value;
            }
        } else {
            _buffer = 8;
        }
    } else {
        _buffer = 8;
    }
    if args.len() > 5 {
        if let Ok(value) = args[5].parse::<u64>() {
            if value == 0 {
                unsafe {
                    DEBUG = false;
                }
            }
            if value == 1 {
                unsafe {
                    DEBUG = true;
                }
            }
        } else {
            unsafe {
                DEBUG = false;
            }
        }
    }

    let mut rng = OsRng;
    let mut living = true;
    let mut lives = 1000;
    while living { 
        let random_depth = rng.gen_range(1..=400);
        if let Err(e) = mine(threads, _buffer, random_depth, miner_address, miner_rpc.clone()).await {
            handle_error(e);
            lives -= 1;
            println!("Thread Collapsed! Rebooting miner!.....");
            println!("Remaining lives: {}", lives);
            if lives <= 0 {
                living = false;
                println!("Miner was killed due to exceeding error count!");
            }
        }
    }
}

fn handle_error(e: impl std::fmt::Debug) {
    // Log the error
    eprintln!("An error occurred: {:?}", e);

    // Run the CLI command
    let command = "./alvarium RPCHERE PUBLIC_KEY 32 8"; // Replace with your actual command for starting your Bifrost pool miner
    if let Err(cmd_err) = Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
    {
        eprintln!("Failed to execute command: {:?}", cmd_err);
    }
}

#[derive(Deserialize)]
struct BalanceStruct {
    value: u64,
}

pub async fn mine(_threads: u64, _buffer: u64, _depth: u64, miner: Pubkey, _rpc: String) -> Result<(), Box<dyn std::error::Error>> {
    let quickrpc: RpcClient = RpcClient::new(_rpc.clone());
    open(&quickrpc).await;
    let mut _previous_challenge: String = String::new();
    let mut _current_challenge: String = String::new();
    let mut _previous_balance: u64 = 0;
    let mut _current_balance: u64 = 0;
    let mut bad_wallet = false;

    let mut index = 0;
    if miner == solana_program::pubkey!("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA") {
        println!("Wallet Address is not configured correctly!");
        bad_wallet = true;
    }
    println!("\n Wallet Address: {}", miner.to_string());
    loop {
        if bad_wallet == true {
            break;
        }

        let webclient = Client::new();
        let rpc_client: RpcClient = RpcClient::new(_rpc.clone());
        let last_hash_at = 0;
        let proof = get_updated_proof_with_authority(&rpc_client, MINING_POOL, last_hash_at).await;

        _current_challenge = bs58::encode(proof.challenge.as_slice()).into_string();

        if _current_challenge != _previous_challenge {
            if index == 0 {
                println!("\n Current Challenge: {}", _current_challenge);
                println!(
                    "\n Mining Pool Stake balance: {} ORE",
                    amount_u64_to_string(proof.balance)
                );
            } else {
                println!(
                    "\n Current Challenge: {}",
                    _current_challenge.bright_white()
                );
                println!(
                    "\n Mining Pool Stake balance: {} ORE",
                    amount_u64_to_string(proof.balance).bright_cyan()
                );
            }

            // Calc cutoff time
            let mut cutoff_time = get_cutoff(&rpc_client, proof, _buffer).await;

            if cutoff_time > 55 {
                std::thread::sleep(Duration::from_millis(500));
                cutoff_time = get_cutoff(&rpc_client, proof, _buffer).await;
            }
            // Run drillx
            let config = get_config(&rpc_client).await;
            let (solution, _best_difficulty, _performance): (Solution, u32, u64) = find_hash_par(
                proof,
                cutoff_time,
                _threads,
                _depth,
                config.min_difficulty as u32,
            )
            .await;

            //Serialize work as a B58 hash
            let workhash: Vec<u8> = [
                solution.d.as_slice(),
                solution.n.as_slice(),
                miner.to_bytes().as_slice(),
                proof.challenge.as_slice(),
                _best_difficulty.to_le_bytes().as_slice(),
                _performance.to_le_bytes().as_slice(),
                _depth.to_le_bytes().as_slice(),
                _threads.to_le_bytes().as_slice(),
            ]
            .concat();

            submit_work(&webclient, MINING_POOL_URL, &workhash).await?;

            let _balance = get_bank_balance(&webclient, &miner).await;
            println!(
                "\n Bank Balance: {}",
                amount_u64_to_string(_balance).bright_green()
            );
            if index == 0 {
                _previous_balance = _balance;
                _current_balance = _balance;
                index += 1;
            } else {
                _previous_balance = _current_balance;
                _current_balance = _balance;
                let reward = _current_balance - _previous_balance;
                println!(
                    "\n Last Reward: {}",
                    amount_u64_to_string(reward).bright_green()
                );
            }
            _previous_challenge = _current_challenge;

            println!("\n Waiting for new work...");
            std::thread::sleep(Duration::from_millis(5000));
        } else {
            std::thread::sleep(Duration::from_millis(2000));
        }
    }

    Ok(())
}

pub async fn find_hash_par(
    proof: Proof,
    cutoff_time: u64,
    threads: u64,
    depth: u64,
    min_difficulty: u32,
) -> (Solution, u32, u64) {
    // Dispatch job to each thread
    let progress_bar = Arc::new(spinner::new_progress_bar());
    let challenge_region_ceiling = u64::MAX.saturating_div(400).saturating_mul(depth + 1);
    let challenge_region_base = u64::MAX.saturating_div(400).saturating_mul(depth);
    let challenge_void = challenge_region_ceiling.saturating_sub(challenge_region_base);
    unsafe {
        if DEBUG == true {
            println!("\n Challenge Region ID: {}", depth);
            println!("\n Challenge Region Ceiling: {}", challenge_region_ceiling);
            println!("\n Challenge Region Base: {}", challenge_region_base);
            println!("\n Challenge Region Range: {}", challenge_void);
        }
    }
    progress_bar.set_message("Mining...");
    let handles: Vec<_> = (0..threads)
        .map(|i| {
            std::thread::spawn({
                let proof = proof.clone();
                let progress_bar = progress_bar.clone();
                let mut memory = equix::SolverMemory::new();
                move || {
                    let timer = Instant::now();
                    let region_slice = challenge_void.saturating_div(threads).saturating_mul(i + 1);
                    let mut nonce = challenge_region_base.saturating_add(region_slice);
                    let seed = nonce;
                    unsafe {
                        if DEBUG == true {
                            println!("\n Thread Seed: {}", seed);
                        }
                    }
                    let mut best_nonce = nonce;
                    let mut best_difficulty = 0;
                    let mut best_hash = Hash::default();
                    let mut total_hashes: u64 = 0;
                    loop {
                        // Create hash
                        for hx in get_hashes_with_memory(
                            &mut memory,
                            &proof.challenge,
                            &nonce.to_le_bytes(),
                        ) {
                            total_hashes += 1;
                            let difficulty = hx.difficulty();
                            if difficulty.gt(&best_difficulty) {
                                best_nonce = nonce;
                                best_difficulty = difficulty;
                                best_hash = hx;
                            }
                        }

                        // Exit if time has elapsed

                        if timer.elapsed().as_secs().ge(&cutoff_time) {
                            if best_difficulty.gt(&min_difficulty) {
                                // Mine until min difficulty has been met
                                break;
                            }
                        } else if i == 0 {
                            progress_bar.set_message(format!(
                                "Mining... ({} sec remaining)",
                                cutoff_time.saturating_sub(timer.elapsed().as_secs()),
                            ));
                        }

                        // Increment nonce
                        nonce += 1;
                    }

                    // Return the best nonce
                    Some((best_nonce, best_difficulty, best_hash, total_hashes))
                }
            })
        })
        .collect();

    // Join handles and return best nonce

    let mut total_nonces = 0;
    let mut best_nonce = 0;
    let mut best_difficulty = 0;
    let mut best_hash = Hash::default();
    for h in handles {
        if let Ok(Some((nonce, difficulty, hash, count))) = h.join() {
            if difficulty > best_difficulty {
                best_difficulty = difficulty;
                best_nonce = nonce;
                best_hash = hash;
            }
            total_nonces += count;
        }
    }

    // Update log
    progress_bar.finish_with_message(format!(
        "\n Best hash: {} (difficulty: {})",
        bs58::encode(best_hash.h).into_string().bright_cyan(),
        format!("{}", best_difficulty).bright_cyan()
    ));
    if total_nonces != 0 {
        println!(
            "\n Hash Power: {} H/s | {} H/m",
            format!("{}", total_nonces.saturating_div(50)).bright_cyan(),
            format!("{}", total_nonces).bright_cyan()
        );
    }
    (
        Solution::new(best_hash.d, best_nonce.to_le_bytes()),
        best_difficulty,
        total_nonces,
    )
}

async fn get_bank_balance(webclient: &Client, miner: &Pubkey) -> u64 {
    let balance_url = format!(
        "https://alvarium.bifrost.technology/balance?miner={}",
        miner.to_string()
    );
    let mut bankbalance: u64 = 0;
    let resp: Result<BalanceStruct, _> = webclient
        .get(balance_url)
        .send()
        .await
        .expect("Failed to get response")
        .json()
        .await;
    match resp {
        Ok(balance) => bankbalance = balance.value,
        Err(e) => eprintln!("Failed to retrieve bank balance: {:?}", e),
    }
    bankbalance
}

async fn submit_work(client: &Client, mining_pool_url: &str, workhash: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let response = client
        .post(mining_pool_url)
        .json(&bs58::encode(workhash).into_string())
        .send()
        .await?;

    if response.status().is_success() {
        println!("\n Work Submission Received: {}", "true".bright_cyan());
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read response text".to_string());
        println!(
            "\n Work Submission Failed: HTTP {} - {}",
            response.status(),
            error_text.bright_red()
        );
    }

    Ok(())
}

pub struct Minersettings {
    _threads: u64,
    _buffer: u64,
    _depth: u64,
    _miner: Pubkey,
    _rpc: String,
}
unsafe impl Send for Minersettings {}

/// Generates drillx hashes from a challenge and nonce using pre-allocated memory.
#[inline(always)]
pub fn get_hashes_with_memory(
    memory: &mut equix::SolverMemory,
    challenge: &[u8; 32],
    nonce: &[u8; 8],
) -> Vec<Hash> {
    let mut hashes: Vec<Hash> = Vec::with_capacity(7);
    if let Ok(solutions) = get_digests_with_memory(memory, challenge, nonce) {
        for solution in solutions {
            let digest = solution.to_bytes();
            hashes.push(Hash {
                d: digest,
                h: hashv(&digest, nonce),
            });
        }
    }

    hashes
}

/// Sorts the provided digest as a list of u16 values.
#[inline(always)]
fn sorted(mut digest: [u8; 16]) -> [u8; 16] {
    unsafe {
        let u16_slice: &mut [u16; 8] = core::mem::transmute(&mut digest);
        u16_slice.sort_unstable();
        digest
    }
}

/// Calculates a hash from the provided digest and nonce.
/// The digest is sorted prior to hashing to prevent malleability.
#[cfg(not(feature = "solana"))]
#[inline(always)]
fn hashv(digest: &[u8; 16], nonce: &[u8; 8]) -> [u8; 32] {
    let mut hasher = sha3::Keccak256::new();
    hasher.update(&sorted(*digest));
    hasher.update(nonce);
    hasher.finalize().into()
}

/// Constructs a keccak digest from a challenge and nonce using equix hashes and pre-allocated memory.
#[inline(always)]
fn get_digests_with_memory(
    memory: &mut equix::SolverMemory,
    challenge: &[u8; 32],
    nonce: &[u8; 8],
) -> Result<SolutionArray, DrillxError> {
    let seed = seed(challenge, nonce);
    let equix = equix::EquiXBuilder::new()
        .runtime(equix::RuntimeOption::TryCompile)
        .build(&seed)
        .map_err(|_| DrillxError::BadEquix)?;
    Ok(equix.solve_with_memory(memory))
}

pub fn check_num_cores(threads: u64) {
    // Check num threads
    let num_cores = num_cpus::get() as u64;
    if threads.gt(&num_cores) {
        println!(
            "{} Number of threads ({}) exceeds available cores ({})",
            "WARNING".bold().yellow(),
            threads,
            num_cores
        );
    }
}

pub async fn should_reset(client: &RpcClient, config: Config) -> bool {
    let rpc_client: &RpcClient = client;
    let clock = get_clock(&rpc_client).await;
    config
        .last_reset_at
        .saturating_add(EPOCH_DURATION)
        .saturating_sub(5) // Buffer
        .le(&clock.unix_timestamp)
}

pub async fn get_cutoff(client: &RpcClient, proof: Proof, buffer_time: u64) -> u64 {
    let rpc_client: &RpcClient = client;
    let clock = get_clock(&rpc_client).await;
    proof
        .last_hash_at
        .saturating_add(60)
        .saturating_sub(buffer_time as i64)
        .saturating_sub(clock.unix_timestamp)
        .max(0) as u64
}

// TODO Pick a better strategy (avoid draining bus)
pub fn find_bus() -> Pubkey {
    let i = rand::thread_rng().gen_range(0..BUS_COUNT);
    BUS_ADDRESSES[i]
}
