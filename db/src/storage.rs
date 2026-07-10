use std::{
    collections::HashMap, fs::File, ops::Deref, sync::{
        Arc, RwLock,
        atomic::{AtomicBool, AtomicUsize},
    },
};

use dashmap::DashMap;

use crate::dbcore::DbValue;

pub static PAGE_SIZE: usize = 16 * 1024;
static NUM_PAGES: usize = (100 * 1000 * 1024) / PAGE_SIZE;

#[derive(Debug)]
pub struct RecordId(pub u64);

#[derive(Debug)]
pub struct Storage {
    pub data: HashMap<String, Vec<DbValue>>,
}

impl Storage {
    pub fn new() -> Storage {
        Storage {
            data: HashMap::new(),
        }
    }

    pub fn get_data(&self) -> &HashMap<String, Vec<DbValue>> {
        &self.data
    }
}

struct Pager {
    file: File,
}

impl Pager {
    pub fn read_page(&mut self, page: usize) -> Page {
        todo!()
    }

    pub fn write_page(&mut self, page: &Page) {
        todo!()
    }

    pub fn alloc_page(&mut self) -> usize {
        todo!()
    }
}

pub struct Page {
    pub id: usize,
    pub pin_count: AtomicUsize, // todo: Replace pin_count and dirty with one atomic
    pub dirty: AtomicBool, // todo: Think about adding io_inprogress flag, if there is a posibility for multiple page writers
    pub data: RwLock<[u8; PAGE_SIZE]>,
}

pub trait PageCache {
    type Handle: PageHandle + Deref<Target = Page>;

    fn get(&self, id: &usize) -> Option<Self::Handle>;

    fn put(&self, page: Page);

    fn iter(&self) -> impl Iterator<Item = Self::Handle>;
}

pub trait PageHandle {
    fn page(&self) -> &Page;
}

struct PageHandleDashMap {
    inner: Arc<Page>,
}

impl PageHandle for PageHandleDashMap {
    fn page(&self) -> &Page {
        &self.inner
    }
}

impl Deref for PageHandleDashMap {
    type Target = Page;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref()
    }
}

struct PageCacheDashMap {
    data: DashMap<usize, Arc<Page>>,
}

impl PageCacheDashMap {
    pub fn new() -> Self {
        Self {
            data: DashMap::with_capacity(NUM_PAGES),
        }
    }
}

impl PageCache for PageCacheDashMap {
    type Handle = PageHandleDashMap;

    fn get(&self, id: &usize) -> Option<Self::Handle> {
        self.data
            .get(id)
            .map(|val| PageHandleDashMap { inner: val.clone() })
    }

    fn put(&self, page: Page) {
        self.data.insert(page.id, page.into());
    }

    fn iter(&self) -> impl Iterator<Item = Self::Handle> {
        self.data
            .iter()
            .map(|v| PageHandleDashMap { inner: v.clone() })
    }
}
