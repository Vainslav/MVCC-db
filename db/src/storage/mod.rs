use std::{io, sync::Arc};

use crate::storage::pages::buffer_pool::ClockBufferPool;

mod background;
mod disk;
mod pages;

#[derive(Debug)]
pub struct RecordId(pub u64);

#[derive(Debug)]
#[repr(C)]
pub struct NewRecordId {
    pub page_id: u16,
    pub file_id: u16,
    pub page_offset: u16,
}

#[derive(Debug)]
pub struct DbValue {
    pub tx_start: usize,
    pub tx_end: usize,
    pub prev: RecordId,
    pub value: String,
}

impl DbValue {
    pub fn new(tx_start: usize, tx_end: usize, value: String) -> DbValue {
        DbValue {
            tx_start,
            tx_end,
            prev: RecordId(0),
            value,
        }
    }
}

pub struct Storage {
    page_cache: Arc<ClockBufferPool>,
}

impl Storage {
    pub fn new(page_cache: Arc<ClockBufferPool>) -> Self {
        Storage { page_cache }
    }

    pub fn get_record_id_by_key(&self, key: &str) -> io::Result<Option<RecordId>> {
        todo!()
    }

    pub fn get_value_by_record_id(&self, record_id: &RecordId) -> io::Result<Option<DbValue>> {
        todo!()
    }

    pub fn set_tx_end(&self, record_id: &RecordId, tx_end: u32) -> io::Result<()> {
        todo!()
    }

    pub fn insert(&self, key: &str, value: DbValue) -> io::Result<RecordId> {
        todo!()
    }
}
