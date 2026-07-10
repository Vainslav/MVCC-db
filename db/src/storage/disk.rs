use std::fs::File;

use crate::storage::page::Page;

pub struct DiskManager {
    file: File,
}

pub fn read_page(page: usize) -> Page {
    todo!()
}

pub fn write_page(page: &Page) {
    todo!()
}

pub fn alloc_page() -> usize {
    todo!()
}
