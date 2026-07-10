use std::collections::BTreeSet;

pub mod manager;

#[derive(Debug, PartialEq)]
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

#[derive(Debug)]
pub struct Transaction {
    id: usize,
    isolation: IsolationLevel,
    state: TransactionState,
    in_progress: BTreeSet<usize>,
    writes: BTreeSet<String>,
    reads: BTreeSet<String>,
}

impl Transaction {
    pub fn new(id: usize, isolation: IsolationLevel) -> Transaction {
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

#[derive(PartialEq)]
pub enum TransactionProcessingError {
    SerializableError,
}
