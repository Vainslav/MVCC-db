use std::{fs::File};

mod index;
mod data;

use crate::storage::pages::{Page, PageType};

pub fn read_page(page_id: usize) -> Page {
    todo!()
}

pub fn write_page(page: &Page) {
    todo!()
}

pub fn alloc_page(page_type: PageType) -> usize {
    todo!()
}