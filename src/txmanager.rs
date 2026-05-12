use std::collections::{BTreeMap, BTreeSet};

use crate::dbcore::{DbValue, IsolationLevel, TransactionState};

#[derive(Debug)]
pub struct Transaction {
    id: usize,
    isolation: IsolationLevel,
    state: TransactionState,
    in_progress: BTreeSet<String>,
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

pub struct TransactionManager {
    transactions: BTreeMap<usize, Transaction>,
    next_transaction_id: usize,
}

impl TransactionManager {
    pub fn new() -> TransactionManager {
        TransactionManager { transactions: BTreeMap::new(), next_transaction_id: 1 }
    }

    pub fn new_transaction(&mut self, isolation: IsolationLevel) -> usize {
        let tx_id = self.next_transaction_id;
        self.next_transaction_id += 1;

        let tx = Transaction::new(tx_id, isolation);
        self.transactions.insert(tx_id, tx);
        dbg!("started transaction {}", tx_id);
        tx_id
    }

    pub fn complete_transaction(&mut self, id: usize, state: TransactionState) {
        let tx = self.transactions.get_mut(&id).unwrap();
        tx.state = state;
    }

    pub fn add_to_read_set(&mut self, id: usize, ids: Vec<String>) {
        let tx = self.transactions.get_mut(&id).unwrap();
        for i in ids {
            tx.reads.insert(i);
        }
    }

    pub fn add_to_write_set(&mut self, id: usize, ids: Vec<String>) {
        let tx = self.transactions.get_mut(&id).unwrap();
        for i in ids {
            tx.writes.insert(i);
        }
    }

    pub fn is_visible(&self, id: usize, db_value: &DbValue) -> bool {
        let tx = self.transactions.get(&id).unwrap();

        match tx.isolation {
            IsolationLevel::ReadUncommitted => {
                return db_value.tx_end == 0;
            }
            IsolationLevel::ReadCommitted => {
                if db_value.tx_start != id
                    && self.transactions.get(&db_value.tx_start).unwrap().state
                        != TransactionState::Committed
                {
                    return false;
                }

                if db_value.tx_end == id {
                    return false;
                }

                if db_value.tx_end > 0
                    && self.transactions.get(&db_value.tx_end).unwrap().state
                        == TransactionState::Committed
                {
                    return false;
                }

                true
            }
            IsolationLevel::RepeatableRead => todo!(),
            IsolationLevel::Serializable => todo!(),
        }
    }

    #[cfg(test)]
    pub fn get_transaction(&mut self) -> &mut BTreeMap<usize, Transaction> {
        &mut self.transactions
    }
}
