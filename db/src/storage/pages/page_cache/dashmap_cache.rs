use std::sync::Arc;

use dashmap::DashMap;

use crate::storage::pages::{NUM_PAGES, Page, page_cache::PageCache};

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
    type Handle = Arc<Page>;

    fn get(&self, id: &usize) -> Option<Self::Handle> {
        self.data.get(id).map(|val| val.clone())
    }

    fn put(&self, page: Page) {
        self.data.insert(page.id, page.into());
    }

    fn iter(&self) -> impl Iterator<Item = Self::Handle> {
        self.data.iter().map(|v| v.clone())
    }
}
