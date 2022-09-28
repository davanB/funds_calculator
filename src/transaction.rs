use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Transaction {
    #[serde(rename = "type")]
    tx_type: TransactionType,
    #[serde(rename = "client")]
    client_id: u16,
    #[serde(rename = "tx")]
    tx_id: u32,
    amount: Option<f32>,
}

impl Transaction {
    pub fn tx_type(&self) -> &TransactionType {
        &self.tx_type
    }

    pub fn client_id(&self) -> u16 {
        self.client_id
    }

    pub fn tx_id(&self) -> u32 {
        self.tx_id
    }

    pub fn amount(&self) -> &Option<f32> {
        &self.amount
    }
}
