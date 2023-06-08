use rusty_piv::{GetRawTransactionInfo, BlockChainInfo, MasternodeList, MasternodeCount, Block, FullBlock, PivxStatus, BitcoinRpcClient};
use serde::{Serialize, Serializer};
use rocksdb::{DB, Options};
use std::time::{Duration, Instant};

fn format_hashes(hash: String) -> String {
    format!("blockhash: {}", hash)
}

fn fetch_new_blocks(db: &DB, client: &BitcoinRpcClient) {
    // Total blocks
    let block_count_time = Instant::now();
    let best_block_height = match client.getblockcount() {
        Ok(best_block) => best_block,
        Err(err) => {
            eprintln!("Failed to get block count {:?}", err);
            return;
        }
    };
    let elapsed_block_count = block_count_time.elapsed();
    println!("Elapsed getblockcount time: {:?}", elapsed_block_count);
    // Grab time for benchmarking
    let db_time = Instant::now();
    // Check DB's block height
    let db_height = get_db_height(&db);
    let elapsed_db_time = db_time.elapsed();
    println!("Elapsed db time: {:?}", elapsed_db_time);
    println!("Db_height {}", db_height.to_string());
    // Blocks + 1 for starting new blocks
    let mut current_block_height = db_height + 1;
    let mut block_count = 0;

    // Batch write buffer
    let mut batch = rocksdb::WriteBatch::default();
    let block_hash_time = Instant::now();
    // Get current blockhash
    let mut block_hash = match client.getblockhash(current_block_height) {
        Ok(hash) => hash,
        Err(err) => {
            eprintln!("Failed to get block hash for height {}: {:?}", current_block_height.to_string(), err);
            return;
        }
    };
    let elapsed_block_hash = block_hash_time.elapsed();
    println!("Elapsed getblockhash time: {:?}", elapsed_block_hash);

    loop {
        // Fetch the block
        let loop_time = Instant::now();
        let get_block_time = Instant::now();
        let block = match client.getblock(block_hash.clone()) {
            Ok(block) => block,
            Err(err) => {
                eprintln!("Failed to get block {}: {:?}", block_hash.clone(), err);
                break;
            }
        };
        let elapsed_get_block = get_block_time.elapsed();
        println!("Elapsed getblock time: {:?}", elapsed_get_block);

        parse_block(&db, &block, &client);
        // Move to the next block using the nextblockhash field
        if let Some(next_block_hash) = block.nextblockhash {
            // Check for batch write every 100 blocks
            if current_block_height % 100 == 0 {
                // Write batch to the database
                let db_write_time = Instant::now();
                db.write(batch).unwrap();
                let elapsed_db_write_time = db_write_time.elapsed();
                println!("Elapsed db write time: {:?}", elapsed_db_write_time);
                // Clear the batch for reuse
                batch = rocksdb::WriteBatch::default();
            }
            // Increment end of loop to continue
            current_block_height += 1;
            // Update the block hash for the next iteration
            block_hash = next_block_hash.clone();
        } else {
            // Terminate the loop if there is no next block
            break;
        }
        let elapsed_loop_time = loop_time.elapsed();
        println!("Elapsed loop time: {:?}", elapsed_loop_time)
    }
    // Write remaining changes in the batch to the database
    db.write(batch).unwrap();
}

fn parse_block(db: &DB, block: &FullBlock, client: &BitcoinRpcClient) {
    println!("Block: {:#?}, {:#?}", block.hash, block.height);

    // Serialize the FullBlock struct into a byte array
    let value = bincode::serialize(&block).unwrap();

    // Use the block hash as the key
    let key = format_hashes(block.hash.clone());
    db.put(key.as_bytes(), value.as_slice()).unwrap();
    // Store transactions individually
    for transaction in &block.tx {
        let tx_key = format!("transaction:{}", transaction);

        // Fetch the transaction data using the transaction ID
        let raw_tx_time = Instant::now();
        let tx = match client.getrawtransaction(transaction.to_string(), true) {
            Ok(tx_data) => {
                let transaction_info = GetRawTransactionInfo {
                    txid: transaction.to_string(),
                    version: tx_data.version,
                    r#type: tx_data.r#type,
                    size: tx_data.size,
                    locktime: tx_data.locktime,
                    vin: tx_data.vin.clone(),
                    vout: tx_data.vout.clone(),
                    hex: tx_data.hex.clone(),
                    value_balance: tx_data.value_balance,
                    value_balance_sat: tx_data.value_balance_sat,
                    vshield_spend: tx_data.vshield_spend.clone(),
                    vshield_output: tx_data.vshield_output.clone(),
                    binding_sig: tx_data.binding_sig.clone(),
                    shielded_addresses: tx_data.shielded_addresses.clone(),
                    extra_payload_size: tx_data.extra_payload_size,
                    extra_payload: tx_data.extra_payload.clone(),
                    blockhash: tx_data.blockhash.clone(),
                    confirmations: tx_data.confirmations,
                    time: tx_data.time,
                    blocktime: tx_data.blocktime,
                };
                let elapsed_raw_tx_time = raw_tx_time.elapsed();
                println!("Elapsed getrawtransaction time: {:?}", elapsed_raw_tx_time);

                // Serialize the transaction data into a byte array
                let tx_bytes = bincode::serialize(&transaction_info).unwrap();

                // Store the transaction data in the database
                db.put(tx_key.as_bytes(), tx_bytes.as_slice()).unwrap();
                //println!("Txid: {:#?}", transaction_info);
                // Serialize the transaction data into a byte array
                let tx_bytes = bincode::serialize(&transaction_info).unwrap();

                // Get the length of the serialized transaction
                let tx_length = tx_bytes.len();

                // Store the transaction data in the database
                db.put(tx_key.as_bytes(), tx_bytes.as_slice()).unwrap();

                println!("Transaction Length: {}", tx_length);
                let stored_tx_value = db.get(tx_key.as_bytes()).unwrap();
                if let Some(tx_bytes) = stored_tx_value {
                    println!("Retrieved Transaction Length: {}", tx_bytes.len());
                } else {
                    eprintln!("Failed to retrieve transaction data");
                }
            }
            Err(err) => {
                eprintln!("Failed to get transaction {}: {:?}", transaction, err);
            }
        };
        
        //println!("Txid: {:#?}", tx);
    }
}

fn get_db_height(db: &DB) -> i64 {
    let mut iter = db.iterator(rocksdb::IteratorMode::Start);
    let mut max_height = -1;

    while let Some(Ok((_key, value))) = iter.next() {
        match bincode::deserialize::<FullBlock>(&value) {
            Ok(block) => {
                if block.height > max_height {
                    max_height = block.height;
                }
            }
            Err(err) => {
                if let bincode::ErrorKind::InvalidUtf8Encoding(utf8_error) = *err {
                    eprintln!("Invalid UTF-8 encoding in block data: {:?}", utf8_error);
                    // Skip this block or apply custom logic
                } else {
                    // Attempt to deserialize as TransactionInfo
                    match bincode::deserialize::<GetRawTransactionInfo>(&value) {
                        Ok(transaction) => {
                            println!("Transaction: {}", transaction);
                        }
                        Err(err) => {
                            eprintln!("Failed to deserialize transaction: {:?}", err);
                        }
                    }
                }
            }
        }
    }

    max_height
}

fn main() {
    let db_path = "DB_DIR";
    let db = DB::open_default(db_path).unwrap();
    //Rpc settings
    let rpchost = String::from("RPC_HOST");
    let rpcuser = String::from("RPC_USER");
    let rpcpass = String::from("RPC_PASS");
    
    let client = BitcoinRpcClient::new(
        rpchost,
        Some(rpcuser),
        Some(rpcpass),
        3,
        10,
        1000
    );

    fetch_new_blocks(&db, &client);
}
