use std::ops::Deref;

use crate::storage::page::Page;

pub trait PageCache {
    type Handle: PageHandle + Deref<Target = Page>;

    fn get(&self, id: &usize) -> Option<Self::Handle>;

    fn put(&self, page: Page);

    fn iter(&self) -> impl Iterator<Item = Self::Handle>;
}

pub trait PageHandle {
    fn page(&self) -> &Page;
}
