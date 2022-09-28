use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::env;
use std::io;

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, Deserialize, Clone)]
struct Transaction {
    #[serde(rename = "type")]
    tx_type: TransactionType,
    #[serde(rename = "client")]
    client_id: u16,
    #[serde(rename = "tx")]
    tx_id: u32,
    amount: Option<f32>,
}

#[derive(Debug)]
struct Funds {
    available: f32,
    held: f32,
}

impl Funds {
    pub fn new(tx: &Transaction) -> Self {
        match tx.tx_type {
            TransactionType::Deposit => Funds {
                available: tx.amount.unwrap(),
                held: 0f32,
            },
            _ => Funds {
                available: 0f32,
                held: 0f32,
            },
        }
    }

    fn calculate_total(&self) -> f32 {
        self.available + self.held
    }
}

type Transactions = HashMap<u32, Transaction>;
type DisputedTransactions = HashSet<u32>;

#[derive(Debug)]
struct Client {
    funds: Funds,
    transactions: Transactions,
    disputed_transactions: DisputedTransactions,
    past_tx: u32,
    locked: bool,
}

impl Client {
    pub fn new(tx_id: u32, tx: Transaction) -> Self {
        Client {
            funds: Funds::new(&tx),
            transactions: Transactions::from([(tx_id, tx)]),
            disputed_transactions: DisputedTransactions::new(),
            past_tx: tx_id,
            locked: false,
        }
    }

    fn add_tx(&mut self, tx_id: u32, tx: Transaction) {
        self.transactions.insert(tx_id, tx);
        self.past_tx = tx_id;
    }

    fn ensure_future_tx(&self, tx_id: u32) -> Result<(), String> {
        if self.past_tx < tx_id {
            Ok(())
        } else {
            Err(format!("Tx {} is in the past!", tx_id))
        }
    }

    fn is_locked(&self) -> bool {
        self.locked
    }

    fn should_tx_be_disputed(&self, tx_id: u32, should_be_disputed: bool) -> bool {
        self.disputed_transactions.contains(&tx_id) == should_be_disputed
    }

    fn get_tx(&self, tx_id: u32) -> Result<&Transaction, String> {
        match self.transactions.get(&tx_id) {
            Some(tx) => Ok(tx),
            None => Err(format!("Tx {} does not exist for client", tx_id)),
        }
    }

    fn tx_is_not_disputed(&self, tx_id: u32) -> Result<(), String> {
        if self.should_tx_be_disputed(tx_id, false) {
            Ok(())
        } else {
            Err(format!(
                "Tx {} should not have been disputed already",
                tx_id
            ))
        }
    }

    fn tx_is_disputed(&self, tx_id: u32) -> Result<(), String> {
        if self.should_tx_be_disputed(tx_id, true) {
            Ok(())
        } else {
            Err(format!("Tx {} should have been disputed already", tx_id))
        }
    }

    fn can_withdraw(&self, withdrawal_amount: f32) -> bool {
        self.funds.available >= withdrawal_amount
    }

    fn deposit_amount(&mut self, tx_id: u32, tx: Transaction) -> Result<(), String> {
        self.ensure_future_tx(tx_id)?;

        self.funds.available += tx.amount.unwrap();
        self.add_tx(tx_id, tx);

        Ok(())
    }

    fn withdraw_amount(&mut self, tx_id: u32, tx: Transaction) -> Result<(), String> {
        self.ensure_future_tx(tx_id)?;

        let withdrawal_amount = tx.amount.unwrap();

        if self.can_withdraw(withdrawal_amount) {
            self.funds.available -= withdrawal_amount;
            self.add_tx(tx_id, tx);

            Ok(())
        } else {
            Err(format!(
                "Insufficient funds to withdraw {}",
                withdrawal_amount
            ))
        }
    }

    fn resolve_amount(&mut self, resolve_amount: f32) {
        self.funds.held -= resolve_amount;
        self.funds.available += resolve_amount;
    }

    fn withhold_amount(&mut self, disputed_amount: f32) {
        self.funds.available -= disputed_amount;
        self.funds.held += disputed_amount;
    }

    fn chargeback_amount(&mut self, chargeback_amount: f32) {
        self.funds.held -= chargeback_amount;
    }

    fn dispute_transaction(&mut self, tx_id: u32) -> Result<(), String> {
        self.tx_is_not_disputed(tx_id)?;
        let tx = self.get_tx(tx_id)?;

        self.withhold_amount(tx.amount.unwrap());
        self.disputed_transactions.insert(tx_id);

        Ok(())
    }

    fn resolve_transaction(&mut self, tx_id: u32) -> Result<(), String> {
        self.tx_is_disputed(tx_id)?;
        let tx = self.get_tx(tx_id)?;

        self.resolve_amount(tx.amount.unwrap());
        self.disputed_transactions.remove(&tx_id);

        Ok(())
    }

    fn chargeback_transaction(&mut self, tx_id: u32) -> Result<(), String> {
        self.tx_is_disputed(tx_id)?;
        let tx = self.get_tx(tx_id)?;

        self.chargeback_amount(tx.amount.unwrap());
        self.locked = true;
        self.disputed_transactions.remove(&tx_id);

        Ok(())
    }

    fn handle_transaction(&mut self, tx: Transaction) -> Result<(), String> {
        if self.is_locked() {
            return Err(format!("Account locked, ignoring {}", tx.tx_id));
        }

        match tx.tx_type {
            TransactionType::Deposit => self.deposit_amount(tx.tx_id, tx),
            TransactionType::Withdrawal => self.withdraw_amount(tx.tx_id, tx),
            TransactionType::Dispute => self.dispute_transaction(tx.tx_id),
            TransactionType::Resolve => self.resolve_transaction(tx.tx_id),
            TransactionType::Chargeback => self.chargeback_transaction(tx.tx_id),
        }
    }

    fn get_record(&self, client_id: u16) -> Vec<String> {
        vec![
            client_id.to_string(),
            format!("{:.4}", self.funds.available),
            format!("{:.4}", self.funds.held),
            format!("{:.4}", self.funds.calculate_total()),
            self.locked.to_string(),
        ]
    }
}

type Clients = HashMap<u16, Client>;

fn write_client_funds(clients: Clients) -> Result<(), String> {
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

fn read_transaction_file() -> Result<String, String> {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.len() == 1 {
        Ok(args[0].clone())
    } else {
        Err(format!(
            "Correct Usage: cargo run -- Records.csv > accounts.csv"
        ))
    }
}

fn parse_transactions(file: String) -> Result<Vec<Transaction>, String> {
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

fn process_transactions(transactions: Vec<Transaction>) -> Result<Clients, String> {
    let mut clients: Clients = HashMap::new();

    for tx in transactions.into_iter() {
        clients
            .entry(tx.client_id)
            .and_modify(|client| {
                if let Err(error) = client.handle_transaction(tx.clone()) {
                    eprintln!("error handling tx: {}", error)
                }
            })
            .or_insert(Client::new(tx.tx_id, tx));
    }

    Ok(clients)
}

fn main() {
    if let Err(error) = read_transaction_file()
        .and_then(parse_transactions)
        .and_then(process_transactions)
        .and_then(write_client_funds)
    {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
