use std::sync::{Arc, RwLock};

use crate::{
    dbcore::{DbValue, IsolationLevel, TransactionState},
    storage::Storage,
    txmanager::TransactionManager,
};

pub enum Command {
    Put(String, String),
    Get(String),
    Delete(String),
    Begin(IsolationLevel),
    Commit,
    Abort,
}

#[derive(Debug)]
pub enum CommandExecutionError {
    Todo,
    NoActiveTransaction,
    TransactionAlreadyActive,
}

pub struct Connection {
    cur_tx: Option<usize>,
    store: Arc<RwLock<Storage>>,
    tx_manager: Arc<RwLock<TransactionManager>>,
}

impl Connection {
    pub fn new(
        store: Arc<RwLock<Storage>>,
        tx_manager: Arc<RwLock<TransactionManager>>,
    ) -> Connection {
        Connection {
            cur_tx: None,
            store,
            tx_manager,
        }
    }

    pub fn begin(&mut self, isolation: IsolationLevel) -> Result<String, CommandExecutionError> {
        if self.cur_tx.is_some() {
            return Err(CommandExecutionError::TransactionAlreadyActive);
        }

        let mut tx_manager = self.tx_manager.write().unwrap();
        self.cur_tx = Some(tx_manager.new_transaction(isolation));

        Ok(String::new())
    }

    pub fn commit(&mut self) -> Result<String, CommandExecutionError> {
        if self.cur_tx.is_none() {
            return Err(CommandExecutionError::NoActiveTransaction);
        }

        let mut tx_manager = self.tx_manager.write().unwrap();
        tx_manager.complete_transaction(self.cur_tx.unwrap(), TransactionState::Committed);
        self.cur_tx = None;

        Ok(String::new())
    }

    pub fn abort(&mut self) -> Result<String, CommandExecutionError> {
        if self.cur_tx.is_none() {
            return Err(CommandExecutionError::NoActiveTransaction);
        }

        let mut tx_manager = self.tx_manager.write().unwrap();
        tx_manager.complete_transaction(self.cur_tx.unwrap(), TransactionState::Aborted);
        self.cur_tx = None;

        Ok(String::new())
    }

    pub fn put(&mut self, id: String, value: String) -> Result<String, CommandExecutionError> {
        let cur_tx = self.current_tx()?;
        self.tx_manager
            .write()
            .unwrap()
            .add_to_write_set(cur_tx, vec![id.clone()]);

        let tx_manager_read = self.tx_manager.read().unwrap();
        let mut storage_read = self.store.write().unwrap();
        let data = &mut storage_read.data;

        if let Some(values) = data.get_mut(&id) {
            for val in values.iter_mut() {
                if tx_manager_read.is_visible(cur_tx, &val) {
                    val.tx_end = cur_tx;
                }
            }

            values.push(DbValue::new(cur_tx, 0, value));
        } else {
            data.insert(id, vec![DbValue::new(cur_tx, 0, value)]);
        };

        Ok(String::new())
    }

    pub fn get(&mut self, id: String) -> Result<String, CommandExecutionError> {
        let cur_tx = self.current_tx()?;
        self.tx_manager
            .write()
            .unwrap()
            .add_to_read_set(cur_tx, vec![id.clone()]);

        let tx_manager_read = self.tx_manager.read().unwrap();
        let storage_read = self.store.read().unwrap();
        let data = &storage_read.data;

        let Some(values) = data.get(&id) else {
            return Err(CommandExecutionError::Todo);
        };

        for val in values {
            if tx_manager_read.is_visible(cur_tx, &val) {
                return Ok(val.value.clone());
            }
        }
        Err(CommandExecutionError::Todo)
    }

    pub fn delete(&mut self, id: String) -> Result<String, CommandExecutionError> {
        let cur_tx = self.current_tx()?;
        self.tx_manager
            .write()
            .unwrap()
            .add_to_write_set(cur_tx, vec![id.clone()]);

        let tx_manager_read = self.tx_manager.read().unwrap();
        let mut storage_read = self.store.write().unwrap();
        let data = &mut storage_read.data;

        if let Some(values) = data.get_mut(&id) {
            let mut found = false;
            for val in values.iter_mut() {
                if tx_manager_read.is_visible(cur_tx, &val) {
                    val.tx_end = cur_tx;
                    found = true;
                }
            }

            if found {
                Ok(String::new())
            } else {
                Err(CommandExecutionError::Todo)
            }
        } else {
            Err(CommandExecutionError::Todo)
        }
    }

    fn current_tx(&self) -> Result<usize, CommandExecutionError> {
        self.cur_tx
            .ok_or(CommandExecutionError::NoActiveTransaction)
    }

    #[cfg(test)]
    pub fn get_tx_manager(&mut self) -> &mut Arc<RwLock<TransactionManager>> {
        &mut self.tx_manager
    }
}

pub fn execute_command(
    con: &mut Connection,
    command: Command,
) -> Result<String, CommandExecutionError> {
    match command {
        Command::Put(id, value) => con.put(id, value),
        Command::Get(id) => con.get(id),
        Command::Delete(id) => con.delete(id),
        Command::Begin(isolation) => con.begin(isolation),
        Command::Commit => con.commit(),
        Command::Abort => con.abort(),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[test]
    fn test_begin() {
        let store = Arc::new(RwLock::new(Storage::new()));
        let tx_manager = Arc::new(RwLock::new(TransactionManager::new()));

        let mut con = Connection::new(store, tx_manager);
        execute_command(&mut con, Command::Begin(IsolationLevel::ReadUncommitted)).unwrap();

        assert!(
            execute_command(&mut con, Command::Begin(IsolationLevel::ReadUncommitted)).is_err()
        );
        assert!(con.cur_tx.is_some());
        assert_eq!(con.cur_tx.unwrap(), 1);
    }

    #[test]
    fn test_commit() {
        let store = Arc::new(RwLock::new(Storage::new()));
        let tx_manager = Arc::new(RwLock::new(TransactionManager::new()));

        let mut con = Connection::new(store, tx_manager);
        assert!(execute_command(&mut con, Command::Commit).is_err());

        execute_command(&mut con, Command::Begin(IsolationLevel::ReadUncommitted)).unwrap();

        assert!(execute_command(&mut con, Command::Commit).is_ok());
        assert!(con.cur_tx.is_none());

        let mut tx_m = con.get_tx_manager().write().unwrap();
        let txs = tx_m.get_transactions();
        assert!(txs.len() == 1);

        let my_tx = txs.get(&1).unwrap();
        assert!(*my_tx.get_state() == TransactionState::Committed);
    }

    #[test]
    fn test_abort() {
        let store = Arc::new(RwLock::new(Storage::new()));
        let tx_manager = Arc::new(RwLock::new(TransactionManager::new()));

        let mut con = Connection::new(store, tx_manager);
        assert!(execute_command(&mut con, Command::Commit).is_err());

        execute_command(&mut con, Command::Begin(IsolationLevel::ReadUncommitted)).unwrap();

        assert!(execute_command(&mut con, Command::Abort).is_ok());
        assert!(con.cur_tx.is_none());

        let mut tx_m = con.get_tx_manager().write().unwrap();
        let txs = tx_m.get_transactions();
        assert!(txs.len() == 1);

        let my_tx = txs.get(&1).unwrap();
        assert!(*my_tx.get_state() == TransactionState::Aborted);
    }
}
