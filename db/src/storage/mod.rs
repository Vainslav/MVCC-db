use std::{
    io,
    ops::Range,
    sync::{Arc, atomic::Ordering},
};

use crate::{
    hash::fnv1a,
    storage::pages::{
        PageId, PageType,
        bucket_page::{BucketChainIter, BucketPageView},
        buffer_pool::ClockBufferPool,
        data_page::DataPageView,
    },
};

mod background;
pub mod disk;
pub mod pages;

#[derive(Debug, Clone, Copy)]
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
    fn from_compacted_bytes(buf: &[u8; RECORD_ID_SIZE]) -> Option<Self> {
        let page_id = u16::from_le_bytes(buf[PAGE_ID_RANGE].try_into().unwrap());
        let file_id = u16::from_le_bytes(buf[FILE_ID_RANGE].try_into().unwrap());
        let page_offset = u16::from_le_bytes(buf[PAGE_OFFSET_RANGE].try_into().unwrap());

        match page_offset {
            0 => None,
            _ => Some(Self {
                page_id,
                file_id,
                page_offset,
            }),
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
pub struct Record {
    pub xmin: u32,
    pub xmax: u32,
    pub prev: Option<NewRecordId>,
    pub value: String,
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
        let NewRecordId {
            page_id,
            file_id,
            page_offset,
        } = *record_id;

        let page = self.page_cache.fetch(
            PageId {
                page_num: page_id,
                file_id,
            },
            PageType::Data,
        )?;
        let read_guard = page.data.read().unwrap();

        let data_page = DataPageView::new(read_guard);

        Ok(Some(data_page.get_record(page_offset as usize)))
    }

    pub fn delete_record(&self, record_id: &NewRecordId, xmax: u32) -> io::Result<()> {
        let NewRecordId {
            page_id,
            file_id,
            page_offset,
        } = *record_id;

        let page = self
            .page_cache
            .fetch(
                PageId {
                    page_num: page_id,
                    file_id,
                },
                PageType::Data,
            )
            .unwrap();
        let write_guard = page.data.write().unwrap();

        let mut data_page = DataPageView::new(write_guard);

        page.dirty.store(true, Ordering::Release);

        data_page.change_xmax(page_offset as usize, xmax);

        Ok(())
    }

    pub fn insert(&self, key: &str, record: Record) -> io::Result<NewRecordId> {
        let key_bytes = key.as_bytes();
        let hash = fnv1a(key_bytes);
        let bucket_number = (hash % 32) as u16;

        let page_id = PageId {
            page_num: bucket_number,
            file_id: 0,
        };

        loop {
            let mut optional_key_page_id = None;

            for page_result in BucketChainIter::new(&self.page_cache, page_id) {
                let page_handle = page_result.unwrap();
                let read_guard = page_handle.data.read().unwrap();

                let bucket = BucketPageView::new(read_guard);

                let key = bucket.find_key(key_bytes, hash);

                if key.is_some() {
                    optional_key_page_id = Some((page_handle.id.clone(), key.unwrap()));
                    break;
                }
            }

            let writable_bucket = if let Some((key_page_id, _)) = optional_key_page_id {
                self.page_cache
                    .fetch(key_page_id, PageType::Bucket)
                    .unwrap()
            } else {
                self.page_cache
                    .fetch(page_id, PageType::Bucket)
                    .expect("FIX ME PLSSSSSSSSSSSSSSSSSSSSSSS")
            };
            let writable_data = self.page_cache.next_writable(PageType::Data).unwrap();

            let bucket_write_guard = writable_bucket.data.write().unwrap();
            let data_write_guard = writable_data.data.write().unwrap();

            let mut bucket_page = BucketPageView::new(bucket_write_guard);
            let mut data_page = DataPageView::new(data_write_guard);

            let recheck = bucket_page.find_key(key_bytes, hash);

            if recheck.is_some() != optional_key_page_id.is_some() {
                continue;
            }

            let offset = data_page
                .append_record(record)
                .expect("FIX ME PLSSSSSSSSSSSSSSSSSSSSSSs");

            writable_bucket.dirty.store(true, Ordering::Release);
            writable_data.dirty.store(true, Ordering::Release);

            let record_id = NewRecordId {
                page_id: writable_data.id.page_num,
                file_id: writable_data.id.file_id,
                page_offset: offset,
            };

            if let Some((_, offset)) = optional_key_page_id {
                bucket_page.change_entry_pointer(offset, record_id);
            } else {
                bucket_page
                    .append_entry(key_bytes, hash, record_id)
                    .expect("FIX ME PLSSSSSSSSSSSSSSSSSSSSSSs");
            }

            return Ok(record_id);
        }
    }
}
