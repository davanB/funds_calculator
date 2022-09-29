use std::collections::HashMap;
use std::env;
use std::io;

mod client;
mod transaction;

use crate::client::{Client, Clients};
use crate::transaction::Transaction;

pub fn process_transactions(transactions: Vec<Transaction>) -> Result<Clients, String> {
    let mut clients: Clients = HashMap::new();

    for tx in transactions.into_iter() {
        clients
            .entry(tx.client_id())
            .and_modify(|client| {
                if let Err(error) = client.handle_transaction(tx.clone()) {
                    eprintln!("error handling tx: {}", error)
                }
            })
            .or_insert(Client::new(tx.tx_id(), tx));
    }

    Ok(clients)
}

pub fn write_client_funds(clients: Clients) -> Result<(), String> {
    let mut wtr = csv::Writer::from_writer(io::stdout());

    let headers = ["client", "available", "held", "total", "locked"];
    wtr.write_record(&headers)
        .map_err(|e| return Err::<(), String>(format!("Error writing to std out: {}", e)))
        .unwrap();

    for (client_id, client) in clients {
        let record = client.get_record(client_id);
        wtr.write_record(&record)
            .map_err(|e| return Err::<(), String>(format!("Error writing to std out: {}", e)))
            .unwrap();
    }

    wtr.flush()
        .map_err(|e| return Err::<(), String>(format!("Error writing to std out: {}", e)))
        .unwrap();

    Ok(())
}

pub fn read_transaction_file() -> Result<String, String> {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.len() < 1 {
        Err(format!(
            "Correct Usage: cargo run -- Records.csv > accounts.csv"
        ))
    } else {
        Ok(args[0].clone())
    }
}

pub fn parse_transactions(file: String) -> Result<Vec<Transaction>, String> {
    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .from_path(file)
        .map_err(|error| return error.to_string())
        .unwrap();

    let mut transactions = Vec::new();

    for result in rdr.deserialize() {
        match result {
            Ok(tx) => transactions.push(tx),
            Err(error) => return Err(format!("Error parsing csv line: {}", error)),
        }
    }

    Ok(transactions)
}
