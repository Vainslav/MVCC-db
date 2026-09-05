use std::{
    io,
    sync::{Arc, RwLock},
};

use crate::{
    storage::{NewRecordId, Record, Storage},
    transactions::{
        IsolationLevel, TransactionProcessingError, TransactionState, manager::TransactionManager,
    },
};

pub enum Command {
    Put(String, String),
    Get(String),
    Delete(String),
    Begin(IsolationLevel),
    Commit,
    Abort,
}

#[derive(Debug, PartialEq)]
pub enum CommandExecutionError {
    NotFound,
    NoneVisible,
    NoActiveTransaction,
    TransactionAlreadyActive,
    SerializationError,
    IoError(io::ErrorKind),
}

impl From<TransactionProcessingError> for CommandExecutionError {
    fn from(value: TransactionProcessingError) -> Self {
        match value {
            TransactionProcessingError::SerializableError => {
                CommandExecutionError::SerializationError
            }
        }
    }
}

impl From<io::Error> for CommandExecutionError {
    fn from(error: io::Error) -> Self {
        CommandExecutionError::IoError(error.kind())
    }
}

pub struct Connection {
    cur_tx: Option<u32>,
    storage: Arc<Storage>,
    tx_manager: Arc<RwLock<TransactionManager>>,
}

impl Connection {
    pub fn new(storage: Arc<Storage>, tx_manager: Arc<RwLock<TransactionManager>>) -> Connection {
        Connection {
            cur_tx: None,
            storage,
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
        let cur_tx = self.current_tx()?;
        let mut tx_manager = self.tx_manager.write().unwrap();
        let commit_result = tx_manager.complete_transaction(cur_tx, TransactionState::Committed);
        if let Err(err) = commit_result
            && err == TransactionProcessingError::SerializableError
        {
            tx_manager
                .complete_transaction(cur_tx, TransactionState::Aborted)
                .expect("Abort failed");
            self.cur_tx = None;
            return Err(CommandExecutionError::from(err));
        }
        self.cur_tx = None;
        Ok(String::new())
    }

    pub fn abort(&mut self) -> Result<String, CommandExecutionError> {
        let cur_tx = self.current_tx()?;
        let mut tx_manager = self.tx_manager.write().unwrap();
        tx_manager
            .complete_transaction(cur_tx, TransactionState::Aborted)
            .expect("Abort failed");
        self.cur_tx = None;
        Ok(String::new())
    }

    pub fn put(&mut self, id: String, value: String) -> Result<String, CommandExecutionError> {
        let cur_tx = self.current_tx()?;
        self.tx_manager
            .write()
            .unwrap()
            .add_to_write_set(cur_tx, vec![id.clone()]);

        let head = self
            .storage
            .get_record_id_by_key(&id)
            .map_err(CommandExecutionError::from)?;

        let record = Record {
            xmin: cur_tx,
            xmax: 0,
            prev: head,
            value,
        };
        self.storage
            .insert(&id, record)
            .map_err(CommandExecutionError::from)?;

        Ok(String::new())
    }

    pub fn get(&mut self, id: String) -> Result<String, CommandExecutionError> {
        let cur_tx = self.current_tx()?;
        self.tx_manager
            .write()
            .unwrap()
            .add_to_read_set(cur_tx, vec![id.clone()]);

        let Some(head) = self
            .storage
            .get_record_id_by_key(&id)
            .map_err(CommandExecutionError::from)?
        else {
            return Err(CommandExecutionError::NotFound);
        };

        match self.find_visible_record(head, cur_tx)? {
            Some(rid) => {
                let record = self
                    .storage
                    .get_value_by_record_id(&rid)?
                    .expect("record_id from chain must exist");
                Ok(record.value)
            }
            None => Err(CommandExecutionError::NoneVisible),
        }
    }

    pub fn delete(&mut self, id: String) -> Result<String, CommandExecutionError> {
        let cur_tx = self.current_tx()?;
        self.tx_manager
            .write()
            .unwrap()
            .add_to_write_set(cur_tx, vec![id.clone()]);

        let Some(head) = self
            .storage
            .get_record_id_by_key(&id)?
        else {
            return Err(CommandExecutionError::NotFound);
        };

        match self.find_visible_record(head, cur_tx)? {
            Some(rid) => {
                self.storage
                    .delete_record(&rid, cur_tx)?;
                Ok(String::new())
            }
            None => Err(CommandExecutionError::NoneVisible),
        }
    }

    fn find_visible_record(
        &self,
        mut rid: NewRecordId,
        cur_tx: u32,
    ) -> Result<Option<NewRecordId>, CommandExecutionError> {
        let tx_manager = self.tx_manager.read().unwrap();
        loop {
            let record = self
                .storage
                .get_value_by_record_id(&rid)?
                .expect("record_id must exist");

            if tx_manager.is_visible(cur_tx, &record) {
                return Ok(Some(rid));
            }

            match record.prev {
                Some(prev) => rid = prev,
                None => return Ok(None),
            }
        }
    }

    fn current_tx(&self) -> Result<u32, CommandExecutionError> {
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
fn make_test_storage() -> Arc<Storage> {
    use crate::storage::disk::DiskManager;
    use crate::storage::pages::buffer_pool::ClockBufferPool;

    use tempfile::tempdir;

    let dir = tempdir().unwrap();

    let index_path = dir.path().join("index.db");
    let data_path = dir.path().join("data.db");

    let disk_manager = DiskManager::init(
        vec![index_path.to_str().unwrap()],
        vec![data_path.to_str().unwrap()],
    );

    let page_cache = Arc::new(ClockBufferPool::new(64, Arc::new(disk_manager)));

    std::mem::forget(dir);

    Arc::new(Storage::new(page_cache))
}

#[cfg(test)]
mod command_tests {
    use super::*;

    #[test]
    fn test_begin() {
        let storage = make_test_storage();
        let tx_manager = Arc::new(RwLock::new(TransactionManager::new()));

        let mut con = Connection::new(storage, tx_manager);
        execute_command(&mut con, Command::Begin(IsolationLevel::ReadUncommitted)).unwrap();

        assert!(
            execute_command(&mut con, Command::Begin(IsolationLevel::ReadUncommitted)).is_err()
        );
        assert!(con.cur_tx.is_some());
        assert_eq!(con.cur_tx.unwrap(), 1);
    }

    #[test]
    fn test_commit() {
        let storage = make_test_storage();
        let tx_manager = Arc::new(RwLock::new(TransactionManager::new()));

        let mut con = Connection::new(storage, tx_manager);
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
        let storage = make_test_storage();
        let tx_manager = Arc::new(RwLock::new(TransactionManager::new()));

        let mut con = Connection::new(storage, tx_manager);
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

#[cfg(test)]
mod isolation_tests {
    use super::*;

    #[test]
    fn test_read_uncommitted() {
        let storage = make_test_storage();
        let tx_manager = Arc::new(RwLock::new(TransactionManager::new()));

        let mut con = Connection::new(storage.clone(), tx_manager.clone());
        let mut con2 = Connection::new(storage, tx_manager.clone());

        execute_command(&mut con, Command::Begin(IsolationLevel::ReadUncommitted)).unwrap();
        execute_command(&mut con2, Command::Begin(IsolationLevel::ReadUncommitted)).unwrap();

        execute_command(&mut con, Command::Put("123".to_string(), "123".to_string())).unwrap();
        assert!(execute_command(&mut con2, Command::Get("123".to_string())).unwrap() == "123");
        assert!(tx_manager.write().unwrap().get_transactions().len() == 2);
    }

    #[test]
    fn test_read_committed() {
        let storage = make_test_storage();
        let tx_manager = Arc::new(RwLock::new(TransactionManager::new()));

        let mut con = Connection::new(storage.clone(), tx_manager.clone());
        let mut con2 = Connection::new(storage.clone(), tx_manager.clone());

        execute_command(&mut con, Command::Begin(IsolationLevel::ReadCommitted)).unwrap();
        execute_command(&mut con2, Command::Begin(IsolationLevel::ReadCommitted)).unwrap();

        execute_command(&mut con, Command::Put("123".to_string(), "123".to_string())).unwrap();
        assert_eq!(
            execute_command(&mut con, Command::Get("123".to_string())).unwrap(),
            "123"
        );

        assert_eq!(
            execute_command(&mut con2, Command::Get("123".to_string()))
                .err()
                .unwrap(),
            CommandExecutionError::NoneVisible
        );

        execute_command(&mut con, Command::Commit).unwrap();
        assert_eq!(
            execute_command(&mut con2, Command::Get("123".to_string())).unwrap(),
            "123"
        );

        execute_command(&mut con, Command::Begin(IsolationLevel::ReadCommitted)).unwrap();

        execute_command(
            &mut con2,
            Command::Put("123".to_string(), "234".to_string()),
        )
        .unwrap();
        assert_eq!(
            execute_command(&mut con2, Command::Get("123".to_string())).unwrap(),
            "234"
        );

        assert_eq!(
            execute_command(&mut con, Command::Get("123".to_string())).unwrap(),
            "123"
        );

        execute_command(&mut con2, Command::Abort).unwrap();
        assert_eq!(
            execute_command(&mut con, Command::Get("123".to_string())).unwrap(),
            "123"
        );

        execute_command(&mut con2, Command::Begin(IsolationLevel::ReadCommitted)).unwrap();
        execute_command(&mut con2, Command::Delete("123".to_string())).unwrap();
        assert_eq!(
            execute_command(&mut con, Command::Get("123".to_string())).unwrap(),
            "123"
        );
        assert_eq!(
            execute_command(&mut con2, Command::Get("123".to_string()))
                .err()
                .unwrap(),
            CommandExecutionError::NoneVisible
        );

        execute_command(&mut con2, Command::Commit).unwrap();
        assert_eq!(
            execute_command(&mut con, Command::Get("123".to_string()))
                .err()
                .unwrap(),
            CommandExecutionError::NoneVisible
        );
    }

    #[test]
    fn test_repeatable_read() {
        let storage = make_test_storage();
        let tx_manager = Arc::new(RwLock::new(TransactionManager::new()));

        let mut con = Connection::new(storage.clone(), tx_manager.clone());
        let mut con2 = Connection::new(storage.clone(), tx_manager.clone());

        execute_command(&mut con, Command::Begin(IsolationLevel::RepeatableRead)).unwrap();
        execute_command(&mut con2, Command::Begin(IsolationLevel::RepeatableRead)).unwrap();

        execute_command(&mut con, Command::Put("123".to_string(), "123".to_string())).unwrap();
        assert_eq!(
            execute_command(&mut con, Command::Get("123".to_string())).unwrap(),
            "123"
        );
        assert_eq!(
            execute_command(&mut con2, Command::Get("123".to_string()))
                .err()
                .unwrap(),
            CommandExecutionError::NoneVisible
        );

        execute_command(&mut con, Command::Commit).unwrap();
        assert_eq!(
            execute_command(&mut con2, Command::Get("123".to_string()))
                .err()
                .unwrap(),
            CommandExecutionError::NoneVisible
        );

        let mut con3 = Connection::new(storage.clone(), tx_manager.clone());
        execute_command(&mut con3, Command::Begin(IsolationLevel::RepeatableRead)).unwrap();
        assert_eq!(
            execute_command(&mut con3, Command::Get("123".to_string())).unwrap(),
            "123"
        );

        execute_command(
            &mut con3,
            Command::Put("123".to_string(), "234".to_string()),
        )
        .unwrap();
        assert_eq!(
            execute_command(&mut con3, Command::Get("123".to_string())).unwrap(),
            "234"
        );
        assert_eq!(
            execute_command(&mut con2, Command::Get("123".to_string()))
                .err()
                .unwrap(),
            CommandExecutionError::NoneVisible
        );

        let mut con4 = Connection::new(storage.clone(), tx_manager.clone());
        execute_command(&mut con4, Command::Begin(IsolationLevel::RepeatableRead)).unwrap();
        assert_eq!(
            execute_command(&mut con4, Command::Get("123".to_string())).unwrap(),
            "123"
        );

        execute_command(&mut con4, Command::Delete("123".to_string())).unwrap();
        assert_eq!(
            execute_command(&mut con4, Command::Get("123".to_string()))
                .err()
                .unwrap(),
            CommandExecutionError::NoneVisible
        );
        assert_eq!(
            execute_command(&mut con2, Command::Get("123".to_string()))
                .err()
                .unwrap(),
            CommandExecutionError::NoneVisible
        );
        assert_eq!(
            execute_command(&mut con3, Command::Get("123".to_string())).unwrap(),
            "234"
        );
    }

    #[test]
    fn test_serializable() {
        let storage = make_test_storage();
        let tx_manager = Arc::new(RwLock::new(TransactionManager::new()));

        let mut con = Connection::new(storage.clone(), tx_manager.clone());
        let mut con2 = Connection::new(storage.clone(), tx_manager.clone());
        let mut con3 = Connection::new(storage.clone(), tx_manager.clone());

        execute_command(&mut con, Command::Begin(IsolationLevel::Serializable)).unwrap();
        execute_command(&mut con2, Command::Begin(IsolationLevel::Serializable)).unwrap();
        execute_command(&mut con3, Command::Begin(IsolationLevel::Serializable)).unwrap();

        execute_command(&mut con, Command::Put("123".to_string(), "123".to_string())).unwrap();
        assert_eq!(
            execute_command(&mut con, Command::Get("123".to_string())).unwrap(),
            "123"
        );
        execute_command(&mut con, Command::Commit).unwrap();

        assert_eq!(
            execute_command(&mut con2, Command::Get("123".to_string()))
                .err()
                .unwrap(),
            CommandExecutionError::NoneVisible
        );
        assert_eq!(
            execute_command(&mut con2, Command::Commit).err().unwrap(),
            CommandExecutionError::SerializationError
        );

        execute_command(
            &mut con3,
            Command::Put("234".to_string(), "Pupupu".to_string()),
        )
        .unwrap();
        execute_command(&mut con3, Command::Commit).unwrap();
    }
}
