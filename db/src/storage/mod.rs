use std::{io, ops::Range, sync::{Arc, atomic::AtomicU64}};

use crate::{
    hash::fnv1a, storage::pages::{
        PageId, PageType, bucket_page::{BucketChainIter, BucketPageView}, buffer_pool::ClockBufferPool, data_page::DataPageView,
    },
};

mod background;
mod disk;
mod pages;

#[derive(Debug)]
pub struct RecordId(pub u64);

#[derive(Debug)]
pub struct NewRecordId {
    pub page_id: u16,
    pub file_id: u16,
    pub page_offset: u16,
}

const RECORD_ID_SIZE: usize =
    std::mem::size_of::<u16>() + std::mem::size_of::<u16>() + std::mem::size_of::<u16>();

const PAGE_ID_RANGE: Range<usize> = 0..2;
const FILE_ID_RANGE: Range<usize> = 2..4;
const PAGE_OFFSET_RANGE: Range<usize> = 4..6;

impl NewRecordId {
    fn from_compacted_bytes(buf: &[u8; RECORD_ID_SIZE]) -> NewRecordId {
        let page_id = u16::from_le_bytes(buf[PAGE_ID_RANGE].try_into().unwrap());
        let file_id = u16::from_le_bytes(buf[FILE_ID_RANGE].try_into().unwrap());
        let page_offset = u16::from_le_bytes(buf[PAGE_OFFSET_RANGE].try_into().unwrap());

        NewRecordId {
            page_id,
            file_id,
            page_offset,
        }
    }

    fn to_le_bytes(&self) -> [u8; RECORD_ID_SIZE] {
        let mut buf = [0; RECORD_ID_SIZE];

        buf[PAGE_ID_RANGE].copy_from_slice(&self.page_id.to_le_bytes());
        buf[FILE_ID_RANGE].copy_from_slice(&self.file_id.to_le_bytes());
        buf[PAGE_OFFSET_RANGE].copy_from_slice(&self.page_offset.to_le_bytes());

        buf
    }
}

#[derive(Debug)]
pub struct KeyId {
    pub page_id: usize,
    pub file_id: usize,
    pub page_offset: usize,
}

#[derive(Debug)]
pub struct Record {
    pub xmin: u32,
    pub xmax: u32,
    pub prev: NewRecordId,
    pub value: String,
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

    pub fn get_value_by_record_id(&self, record_id: &NewRecordId) -> io::Result<Option<Record>> {
        let NewRecordId { page_id, file_id, page_offset } = *record_id;

        let page = self.page_cache.fetch(PageId { page_num: page_id, file_id }, PageType::Data)?;
        let read_guard = page.data.read().unwrap();

        let data_page = DataPageView::new(read_guard);

        Ok(Some(data_page.get_record(page_offset as usize)))
    }

    pub fn delete_record(&self, record_id: &NewRecordId, xmax: u32) -> io::Result<()> {
        let NewRecordId { page_id, file_id, page_offset } = *record_id;
        
        let page = self.page_cache.fetch(PageId { page_num: page_id, file_id }, PageType::Data)?;
        let write_guard = page.data.write().unwrap();

        let mut data_page = DataPageView::new(write_guard);

        data_page.change_xmax(page_offset as usize, xmax);

        Ok(())
    }

    pub fn insert(&self, key: &str, value: Record) -> io::Result<NewRecordId> {
        todo!()
    }
}
