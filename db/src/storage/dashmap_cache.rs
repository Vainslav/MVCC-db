use std::{ops::Deref, sync::Arc};

use dashmap::DashMap;

use crate::storage::{
    page::{NUM_PAGES, Page},
    page_cache::{PageCache, PageHandle},
};

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
