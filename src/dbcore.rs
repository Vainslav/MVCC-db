use std::collections::BTreeSet;

pub struct DbValue {
    pub tx_start: usize,
    pub tx_end: usize,
    pub value: String,
}

impl DbValue {
    pub fn new(tx_start: usize, tx_end: usize, value: String) -> DbValue {
        DbValue {
            tx_start,
            tx_end,
            value,
        }
    }
}

#[derive(Debug)]
pub enum IsolationLevel {
    ReadUncommitted,
    ReadCommitted,
    RepeatableRead,
    Serializable,
}

#[derive(Debug, PartialEq)]
pub enum TransactionState {
    InProgress,
    Aborted,
    Committed,
}
