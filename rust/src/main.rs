//! Chidinma's Rust Bitcoin Transaction Capstone Project
//!
//! This Rust program connects to a Bitcoin Core regtest node, creates two wallets ('Miner' and 'Trader'), mines spendable coins,
//! sends a transaction from Miner to Trader, confirms it by mining a block, extracts relevant transaction details, and writes
//! the results to a file (`out.txt`) for test evaluation.

#![allow(unused)]
use bitcoin::hex::DisplayHex;
use bitcoincore_rpc::bitcoin::{Amount, BlockHash};
use bitcoincore_rpc::{Auth, Client, RpcApi};
use serde::Deserialize;
use serde_json::json;
use std::error::Error;
use std::fmt::Debug;
use std::fs::File;
use std::io::{stdout, Write};
use std::result::Result;
use std::{thread, time::Duration};

// Node access params; these are constants necessary for connecting to RPC core
const RPC_URL: &str = "http://127.0.0.1:18443"; // Default regtest RPC port
const RPC_USER: &str = "alice";
const RPC_PASS: &str = "password";

// Raw RPC call demo (not used in final code)
fn send(rpc: &Client, addr: &str) -> bitcoincore_rpc::Result<String> {
    let args = [
        json!([{addr : 100 }]),
        json!(null),
        json!(null),
        json!(null),
        json!(null),
    ];

    #[derive(Deserialize)]
    struct SendResult {
        complete: bool,
        txid: String,
    }
    let send_result = rpc.call::<SendResult>("send", &args)?;
    assert!(send_result.complete);
    Ok(send_result.txid)
}

fn main() -> Result<(), Box<dyn Error>> {
    // Connect to Bitcoin Core RPC; the bitcoincore_rpc crate, wraps the JSON-RPC API into Rust methods.
    let rpc = Client::new(
        RPC_URL,
        Auth::UserPass(RPC_USER.to_owned(), RPC_PASS.to_owned()),
    )?;

    println!("\n Connected to Bitcoin Core RPC at {}", RPC_URL);

    // Fetch and display blockchain info using get blockchain info
    let blockchain_info = rpc.get_blockchain_info()?;
    println!("Blockchain Info: {:?}", blockchain_info);

    // Ensure 'Miner' and 'Trader' wallets exist; this function is to ensure a wallet exists. If not, create it.
    fn ensure_wallet_exists(rpc: &Client, wallet_name: &str) -> Result<(), Box<dyn Error>> {
        let loaded_wallets = rpc.list_wallets()?;
        if !loaded_wallets.contains(&wallet_name.to_string()) {
            println!("Creating wallet: {}", wallet_name);
            rpc.create_wallet(wallet_name, None, None, None, None)?;
        } else {
            println!("Wallet already exists: {}", wallet_name);
        }
        Ok(())
    }

    ensure_wallet_exists(&rpc, "Miner")?;
    ensure_wallet_exists(&rpc, "Trader")?;

    // Create wallet-specific clients (This function checks if a wallet is already loaded, and if not, creates it. )
    //Wallets in Bitcoin Core must be explicitly referenced in the RPC endpoint like `/wallet/Miner` because Bitcoin Core does not automatically create wallets.
    //You must manually create and load them by name.

    let miner = Client::new(
        &format!("{}/wallet/Miner", RPC_URL),
        Auth::UserPass(RPC_USER.to_string(), RPC_PASS.to_string()),
    )?;
    let trader = Client::new(
        &format!("{}/wallet/Trader", RPC_URL),
        Auth::UserPass(RPC_USER.to_string(), RPC_PASS.to_string()),
    )?;

    println!("Wallets Miner and Trader are ready.");

    // Generate spendable balance by mining until matured coinbase; this has to do with how many blocks are to be mined for coinbase rewards
    // Coinbase rewards in Bitcoin require 100 block confirmations before they can be spent preventing miners from re-orging the chain to reclaim their own rewards.
    // Generate address with the exact label "Mining Reward" as specified in instructions
    let miner_address = miner
        .get_new_address(Some("Mining Reward"), None)? // Changed to exact label required by instructions
        .require_network(bitcoincore_rpc::bitcoin::Network::Regtest)?;

    println!("Miner address: {}", miner_address);

    // Mine blocks until coinbase reward is spendable (requires maturity of 100 blocks)
    let mut blocks_mined = 0;
    let max_blocks = 150; // Safety limit

    // Mine 1 block to the miner's address
    loop {
        if blocks_mined >= max_blocks {
            return Err("Failed to achieve spendable balance after mining maximum blocks".into());
        }

        miner.generate_to_address(1, &miner_address)?;
        blocks_mined += 1;

        // Check spendable balance
        let balance = miner.get_balance(None, None)?;
        println!("Block {} → Balance: {} BTC", blocks_mined, balance.to_btc());

        if balance.to_btc() > 0.0 {
            println!(
                "Spendable balance achieved after {} blocks mined.",
                blocks_mined
            );
            break;
        }
    }

    // Generate Trader receiving address (this is the recipient of the 20 BTC transaction.)
    // Generate address with exact label "Received" as specified in instructions
    let trader_address = trader
        .get_new_address(Some("Received"), None)? //Generates a fresh BTC address from Trader wallet with correct label
        .require_network(bitcoincore_rpc::bitcoin::Network::Regtest)?;
    println!("Trader receiving address: {}", trader_address);

    // Send 20 BTC from Miner wallet to Trader's receivinf address (Defines 20.0 BTC using the Amount::from_btc() helper.)
    let amount_to_send = Amount::from_btc(20.0)?;

    // The send_to_address RPC sends the specified amount to the given address (Sends that amount from the Miner wallet to the Trader's address using `send_to_address`. This broadcasts the transaction but doesn't confirm it yet.)
    let txid = miner.send_to_address(
        &trader_address,
        amount_to_send,
        Some("Payment to Trader"),
        None,
        None,
        None,
        None,
        None,
    )?;
    println!("You have Sent 20 BTC 🪙 to Trader. TxID: {}", txid);

    // Check if TX is in mempool (`get_raw_mempool()` is used to confirm if a transaction is pending (i.e., unconfirmed). If it's listed, that means it is awaiting inclusion in a block.)
    let mempool = miner.get_raw_mempool()?;
    if mempool.contains(&txid) {
        println!("Transaction is in the mempool.");
        // Fetch the unconfirmed transaction from the node's mempool as requested in instructions (using getmempoolentry)
        let mempool_entry = miner.get_mempool_entry(&txid)?;
        println!("Mempool entry details: {:?}", mempool_entry);
    } else {
        println!("⚠️ Transaction not found in mempool.");
    }

    fn play_celebration_animation() {
        let spinner = [
            "🌕", "🌖", "😮", "🌗", "🌘", "🤭", "🌑", "🌒", "🥰", "🌓", "😆", "😅", "😂", "🤣",
            "🌔", "🤑",
        ];
        let delay = Duration::from_millis(150);
        let mut stdout = stdout();

        print!("Celebrating success ");
        for i in 0..spinner.len() * 3 {
            print!("\rCelebrating success {}", spinner[i % spinner.len()]);
            print!("\x07"); // Play bell sound
            stdout.flush().unwrap();
            thread::sleep(delay);
        }

        println!("\r Your Transaction is confirmed and saved successfully! 🙂, Now you can go 🙄");
    }

    // Mine 1 block to confirm the transaction
    let _ = miner.generate_to_address(1, &miner_address)?;
    println!("1 block has been mined to confirm your transaction");

    // Extract transaction details
    let raw = miner.get_raw_transaction_info(&txid, None)?;
    let decoded_tx = &raw.transaction()?; // Access transaction directly, not call .transaction()

    // Trace miner's tx input address using the vin source
    if decoded_tx.input.is_empty() {
        return Err("Transaction has no inputs".into());
    }

    let input_txid = decoded_tx.input[0].previous_output.txid;
    let input_vout = decoded_tx.input[0].previous_output.vout;
    let prev_tx = miner.get_raw_transaction_info(&input_txid, None)?;

    if prev_tx.vout.len() <= input_vout as usize {
        return Err("Invalid input reference".into());
    } else {
        let prev_output = &prev_tx.vout[input_vout as usize];

        let miner_input_address = format!("{:?}", prev_output.script_pub_key);

        //Using amount as Amount type
        let miner_input_amount = prev_output.value;

        // Identify Trader output and Miner change output
        let mut trader_output_address = String::new();
        let mut trader_output_amount = Amount::ZERO; // Fix 3: Use Amount::ZERO
        let mut miner_change_address = String::new();
        let mut miner_change_amount = Amount::ZERO; // Fix 4: Use Amount::ZERO

        // Match address to identify which is Trader and which is change back to Miner
        for output in decoded_tx.output.iter() {
            let value = output.value;

            let address = format!("{:?}", output.script_pubkey);

            let trader_script = format!("{:?}", trader_address.script_pubkey());

            if address == trader_script {
                trader_output_address = address;
                trader_output_amount = value; // Fix 6: Keep as Amount
            } else {
                miner_change_address = address;
                miner_change_amount = value; // Fix 7: Keep as Amount
            }
        }
        // Extract other required fields
        // Fee calculation with Amount types
        let total_output: Amount = decoded_tx.output.iter().map(|out| out.value).sum();
        let fee = miner_input_amount
            .checked_sub(total_output)
            .unwrap_or(Amount::ZERO);

        // Get block info from raw transaction result
        let tx_block_hash = raw.blockhash.ok_or("Transaction not in a block")?;
        let block_info = miner.get_block_info(&tx_block_hash)?;
        let block_height = block_info.height;
        let block_hash = tx_block_hash.to_string();

        // Print all details to terminal for verification
        println!("\nTransaction Details:");
        println!("Transaction ID: {}", txid);
        println!("Miner Input Address: {}", miner_input_address);
        println!("Miner Input Amount: {:.8} BTC", miner_input_amount.to_btc()); // values are formatted to 8 decimal places using `{:.8}` for Bitcoin precision.
        println!("Trader Output Address: {}", trader_output_address);
        println!(
            "Trader Output Amount: {:.8} BTC",
            trader_output_amount.to_btc()
        );
        println!("Miner Change Address: {}", miner_change_address);
        println!(
            "Miner Change Amount: {:.8} BTC",
            miner_change_amount.to_btc()
        );
        println!("Fee: {:.8} BTC", fee.to_btc());
        println!("Block Height: {}", block_height);
        println!("Block Hash: {}", block_hash);

        // Carefullly write all 10 required transaction fields and blok info to out.txt file (changed from ../out.txt to current directory)
        let mut file = File::create("out.txt")?; // Fixed: Write to current directory instead of ../out.txt
        writeln!(file, "{}", txid)?;
        writeln!(file, "{}", miner_input_address)?;
        writeln!(file, "{}", miner_input_amount.to_btc())?; // Use .to_btc() for proper decimal formatting
        writeln!(file, "{}", trader_output_address)?;
        writeln!(file, "{}", trader_output_amount.to_btc())?; // Use .to_btc() for proper decimal formatting
        writeln!(file, "{}", miner_change_address)?;
        writeln!(file, "{}", miner_change_amount.to_btc())?; // Use .to_btc() for proper decimal formatting
        writeln!(file, "{}", fee.to_btc())?; // Use .to_btc() for proper decimal formatting
        writeln!(file, "{}", block_height)?;
        writeln!(file, "{}", block_hash)?;

        println!("\n All required values written to out.txt for test evaluation"); // Updated message to reflect correct file location
        play_celebration_animation();
        Ok(())

        /*
        Each line maps directly to the required fields in the test file:
           1. Transaction ID
           2. Input address (ASM)
           3. Input amount
           4. Trader's address
           5. Trader's amount
           6. Miner's change address
           7. Miner's change amount
           8. Fee (BTC)
           9. Block height
          10. Block hash

        This completes the pipeline from wallet → transaction → confirmation → file output.
        */
    }
}