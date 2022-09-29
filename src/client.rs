use crate::transaction::{Transaction, TransactionType};
use std::collections::{HashMap, HashSet};

#[derive(Debug, PartialEq)]
pub struct Funds {
    available: f32,
    held: f32,
}

impl Funds {
    pub fn new(tx: &Transaction) -> Self {
        match tx.tx_type() {
            TransactionType::Deposit => Funds {
                available: tx.amount().unwrap(),
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

pub type Transactions = HashMap<u32, Transaction>;
pub type DisputedTransactions = HashSet<u32>;

#[derive(Debug)]
pub struct Client {
    funds: Funds,
    transactions: Transactions,
    disputed_transactions: DisputedTransactions,
    past_tx: u32,
    locked: bool,
}

pub type Clients = HashMap<u16, Client>;

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

    pub fn funds(&self) -> &Funds {
        &self.funds
    }

    pub fn is_locked(&self) -> bool {
        self.locked
    }

    pub fn handle_transaction(&mut self, tx: Transaction) -> Result<(), String> {
        if self.is_locked() {
            return Err(format!("Account locked, ignoring {}", tx.tx_id()));
        }

        match tx.tx_type() {
            TransactionType::Deposit => self.deposit_amount(tx.tx_id(), tx),
            TransactionType::Withdrawal => self.withdraw_amount(tx.tx_id(), tx),
            TransactionType::Dispute => self.dispute_transaction(tx.tx_id()),
            TransactionType::Resolve => self.resolve_transaction(tx.tx_id()),
            TransactionType::Chargeback => self.chargeback_transaction(tx.tx_id()),
        }
    }

    pub fn get_record(&self, client_id: u16) -> Vec<String> {
        vec![
            client_id.to_string(),
            format!("{:.4}", self.funds.available),
            format!("{:.4}", self.funds.held),
            format!("{:.4}", self.funds.calculate_total()),
            self.locked.to_string(),
        ]
    }

    fn add_tx(&mut self, tx_id: u32, tx: Transaction) {
        self.transactions.insert(tx_id, tx);
        self.past_tx = tx_id;
    }

    // Transaction IDs (tx) are globally unique, though are also not guaranteed to be ordered.
    // Ensure txs arrive in chronological order per client
    fn ensure_future_tx(&self, tx_id: u32) -> Result<(), String> {
        if self.past_tx < tx_id {
            Ok(())
        } else {
            Err(format!("Tx {} is in the past!", tx_id))
        }
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

        self.funds.available += tx.amount().unwrap();
        self.add_tx(tx_id, tx);

        Ok(())
    }

    fn withdraw_amount(&mut self, tx_id: u32, tx: Transaction) -> Result<(), String> {
        self.ensure_future_tx(tx_id)?;

        let withdrawal_amount = tx.amount().unwrap();

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

        self.withhold_amount(tx.amount().unwrap());
        self.disputed_transactions.insert(tx_id);

        Ok(())
    }

    fn resolve_transaction(&mut self, tx_id: u32) -> Result<(), String> {
        self.tx_is_disputed(tx_id)?;
        let tx = self.get_tx(tx_id)?;

        self.resolve_amount(tx.amount().unwrap());
        self.disputed_transactions.remove(&tx_id);

        Ok(())
    }

    fn chargeback_transaction(&mut self, tx_id: u32) -> Result<(), String> {
        self.tx_is_disputed(tx_id)?;
        let tx = self.get_tx(tx_id)?;

        self.chargeback_amount(tx.amount().unwrap());
        self.locked = true;
        self.disputed_transactions.remove(&tx_id);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_calculate_total_funds() {
        let tx_1 = Transaction::new(TransactionType::Deposit, 1, 1, Some(1.5));
        let funds = Funds::new(&tx_1);
        assert_eq!(
            funds,
            Funds {
                available: 1.5,
                held: 0.0
            }
        );

        let tx_2 = Transaction::new(TransactionType::Chargeback, 1, 1, None);
        let funds = Funds::new(&tx_2);
        assert_eq!(
            funds,
            Funds {
                available: 0.0,
                held: 0.0
            }
        );
    }

    #[test]
    fn can_handle_deposit() {
        let client_id = 1;
        let initial_deposit = Transaction::new(TransactionType::Deposit, 1, client_id, Some(1.5));
        let next_deposit = Transaction::new(TransactionType::Deposit, 2, client_id, Some(1.5));

        let mut client = Client::new(1, initial_deposit);
        client.handle_transaction(next_deposit).unwrap();

        assert_eq!(
            *client.funds(),
            Funds {
                available: 3.0,
                held: 0.0
            }
        )
    }

    #[test]
    fn can_handle_withdrawal() {
        let client_id = 1;
        let initial_deposit = Transaction::new(TransactionType::Deposit, 1, client_id, Some(1.5));
        let withdrawal = Transaction::new(TransactionType::Withdrawal, 2, client_id, Some(1.5));

        let mut client = Client::new(1, initial_deposit);
        client.handle_transaction(withdrawal).unwrap();

        assert_eq!(
            *client.funds(),
            Funds {
                available: 0.0,
                held: 0.0
            }
        )
    }

    #[test]
    fn can_handle_dispute() {
        let client_id = 1;
        let initial_deposit = Transaction::new(TransactionType::Deposit, 1, client_id, Some(1.5));
        let dispute = Transaction::new(TransactionType::Dispute, 1, client_id, None);

        let mut client = Client::new(1, initial_deposit);
        client.handle_transaction(dispute).unwrap();

        assert_eq!(
            *client.funds(),
            Funds {
                available: 0.0,
                held: 1.5
            }
        )
    }

    #[test]
    fn can_handle_resolution() {
        let client_id = 1;
        let initial_deposit = Transaction::new(TransactionType::Deposit, 1, client_id, Some(1.5));
        let dispute = Transaction::new(TransactionType::Dispute, 1, client_id, None);
        let resolution = Transaction::new(TransactionType::Resolve, 1, client_id, None);

        let mut client = Client::new(1, initial_deposit);
        client.handle_transaction(dispute).unwrap();
        client.handle_transaction(resolution).unwrap();

        assert_eq!(
            *client.funds(),
            Funds {
                available: 1.5,
                held: 0.0
            }
        )
    }

    #[test]
    fn can_handle_chargeback() {
        let client_id = 1;
        let initial_deposit = Transaction::new(TransactionType::Deposit, 1, client_id, Some(1.5));
        let dispute = Transaction::new(TransactionType::Dispute, 1, client_id, None);
        let chargeback = Transaction::new(TransactionType::Chargeback, 1, client_id, None);

        let mut client = Client::new(1, initial_deposit);
        client.handle_transaction(dispute).unwrap();
        client.handle_transaction(chargeback).unwrap();

        assert_eq!(
            *client.funds(),
            Funds {
                available: 0.0,
                held: 0.0
            }
        );

        assert!(client.is_locked())
    }

    #[test]
    fn can_get_record() {
        let client_id = 1;
        let initial_deposit = Transaction::new(TransactionType::Deposit, 1, client_id, Some(1.5));
        let client = Client::new(1, initial_deposit);

        assert_eq!(
            client.get_record(client_id),
            vec!["1", "1.5000", "0.0000", "1.5000", "false"]
        )
    }

    #[test]
    fn fails_dispute_when_tx_does_not_exist() {
        let client_id = 1;
        let initial_deposit = Transaction::new(TransactionType::Deposit, 1, client_id, Some(1.5));
        let dispute = Transaction::new(TransactionType::Dispute, 2, client_id, None);

        let mut client = Client::new(1, initial_deposit);
        if let Err(_error) = client.handle_transaction(dispute) {
            assert!(true)
        } else {
            assert!(false)
        }
    }

    #[test]
    fn fails_resolve_when_tx_does_not_exist() {
        let client_id = 1;
        let initial_deposit = Transaction::new(TransactionType::Deposit, 1, client_id, Some(1.5));
        let resolve = Transaction::new(TransactionType::Resolve, 2, client_id, None);

        let mut client = Client::new(1, initial_deposit);
        if let Err(_error) = client.handle_transaction(resolve) {
            assert!(true)
        } else {
            assert!(false)
        }
    }

    #[test]
    fn fails_chargeback_when_tx_does_not_exist() {
        let client_id = 1;
        let initial_deposit = Transaction::new(TransactionType::Deposit, 1, client_id, Some(1.5));
        let chargeback = Transaction::new(TransactionType::Chargeback, 2, client_id, None);

        let mut client = Client::new(1, initial_deposit);
        if let Err(_error) = client.handle_transaction(chargeback) {
            assert!(true)
        } else {
            assert!(false)
        }
    }

    #[test]
    fn fails_withdrawal_on_insufficient_funds() {
        let client_id = 1;
        let initial_deposit = Transaction::new(TransactionType::Deposit, 1, client_id, Some(1.5));
        let withdrawal = Transaction::new(TransactionType::Withdrawal, 2, client_id, Some(2.0));

        let mut client = Client::new(1, initial_deposit);
        if let Err(_error) = client.handle_transaction(withdrawal) {
            assert!(true)
        } else {
            assert!(false)
        }
    }

    #[test]
    fn fails_when_tx_not_in_future() {
        let client_id = 1;
        let tx_id = 1;
        let initial_deposit =
            Transaction::new(TransactionType::Deposit, tx_id, client_id, Some(1.5));
        let next_deposit = Transaction::new(TransactionType::Deposit, tx_id, client_id, Some(1.5));

        let mut client = Client::new(tx_id, initial_deposit);
        if let Err(_error) = client.handle_transaction(next_deposit) {
            assert!(true)
        } else {
            assert!(false)
        }
    }
}
