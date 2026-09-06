use std::{
    collections::BTreeSet,
    io::{self, ErrorKind},
};

pub mod manager;
mod transaction_file;

#[derive(Debug, PartialEq)]
pub enum IsolationLevel {
    ReadUncommitted,
    ReadCommitted,
    RepeatableRead,
    Serializable,
}

#[repr(u8)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TransactionState {
    InProgress,
    Aborted,
    Committed,
}

impl From<u8> for TransactionState {
    fn from(value: u8) -> Self {
        match value {
            0 => TransactionState::InProgress,
            1 => TransactionState::Aborted,
            2 => TransactionState::Committed,
            _ => panic!("Invalid TransactionState value {}", value),
        }
    }
}

#[derive(Debug)]
pub struct Transaction {
    id: u32,
    isolation: IsolationLevel,
    state: TransactionState,
    in_progress: BTreeSet<u32>,
    writes: BTreeSet<String>,
    reads: BTreeSet<String>,
}

impl Transaction {
    pub fn new(id: u32, isolation: IsolationLevel) -> Transaction {
        Transaction {
            id,
            isolation,
            state: TransactionState::InProgress,
            in_progress: BTreeSet::new(),
            writes: BTreeSet::new(),
            reads: BTreeSet::new(),
        }
    }

    #[cfg(test)]
    pub fn get_state(&self) -> &TransactionState {
        &self.state
    }
}

#[derive(PartialEq, Debug)]
pub enum TransactionProcessingError {
    SerializableError,
    IoError(ErrorKind),
}

impl From<io::Error> for TransactionProcessingError {
    fn from(value: io::Error) -> Self {
        Self::IoError(value.kind())
    }
}
