use std::{
    hash::{DefaultHasher, Hash},
    io,
    sync::Arc,
};

use crate::{
    hash::fnv1a,
    storage::pages::{
        PageId,
        bucket_page::{BucketChainIter, BucketPageView},
        buffer_pool::ClockBufferPool,
    },
};

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

    pub fn get_record_id_by_key(&self, key: &str) -> io::Result<Option<NewRecordId>> {
        let bytes = key.as_bytes();
        let hash = fnv1a(bytes);
        let bucket_number = (hash % 32) as u16;

        let page_id = PageId {
            page_num: bucket_number,
            file_id: 0,
        };

        for page_result in BucketChainIter::new(&self.page_cache, page_id) {
            let page_handle = page_result?;
            let read_guard = page_handle.data.read().unwrap();

            let bucket = BucketPageView::new(read_guard);

            let record = bucket.find_entry(bytes, hash);

            if record.is_some() {
                return Ok(record);
            }
        }

        Ok(None)
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
