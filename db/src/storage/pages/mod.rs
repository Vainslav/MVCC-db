pub mod bucket_page;
pub mod data_page;
pub mod page_cache;

use std::sync::{
    RwLock,
    atomic::{AtomicBool, AtomicUsize},
};

use crate::storage::{RecordId, pages::bucket_page::get_bucket_page_init_bytes};

pub const PAGE_SIZE: usize = 16 * 1024;
pub const NUM_PAGES: usize = (100 * 1000 * 1024) / PAGE_SIZE;

const PAGE_TYPE_OFFSET: usize = 0;

#[repr(u8)]
pub enum PageType {
    Bucket = 1,
    Data = 2,
}

pub struct Page {
    pub id: usize,
    pub pin_count: AtomicUsize, // todo: Replace pin_count and dirty with one atomic
    pub dirty: AtomicBool, // todo: Think about adding io_inprogress flag, if there is a posibility for multiple page writers
    pub data: RwLock<[u8; PAGE_SIZE]>,
}

impl Page {
    pub fn get_init_page_bytes(page_type: PageType) -> [u8; PAGE_SIZE] {
        match page_type {
            PageType::Bucket => get_bucket_page_init_bytes(),
            PageType::Data => todo!(),
        }
    }
}
