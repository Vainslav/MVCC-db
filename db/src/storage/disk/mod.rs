mod data;
mod index;

use std::{
    fs::{File, OpenOptions},
    io,
    path::Path,
    sync::RwLock,
};

use crate::{
    storage::{
        disk::{data::DataFileHeader, index::IndexFileHeader},
        pages::{PAGE_SIZE, Page, PageType, get_init_page_bytes},
    },
    write_read_impl::{read_exact_at_impl, write_at_impl},
};

pub struct DiskManager {
    index_files: Vec<DiskFile<IndexFileHeader>>,
    data_files: Vec<DiskFile<DataFileHeader>>,
}

impl DiskManager {
    pub fn init(index_file_paths: Vec<&str>, data_file_paths: Vec<&str>) -> DiskManager {
        todo!()
    }

    pub fn read_page(&self, page_id: usize) -> Page {
        todo!()
    }

    pub fn write_page(&mut self, page: &Page) {
        match page.get_page_type() {
            PageType::Bucket => todo!(),
            PageType::Data => todo!(),
        }
    }

    pub fn alloc_page(&mut self, page_type: PageType) -> Page {
        match page_type {
            PageType::Bucket => todo!(),
            PageType::Data => todo!(),
        }
    }
}

struct DiskFile<T: FileHeader> {
    file: File,
    header: RwLock<T>,
}

impl<T: FileHeader> DiskFile<T> {
    pub fn open(path: &Path) -> io::Result<Self> {
        let (file, header) = match File::create_new(path) {
            Ok(f) => {
                let header = T::new();
                T::write_header_to_file(&f, &header)?;
                (f, header)
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                let f = OpenOptions::new().read(true).write(true).open(path)?;
                let header = T::read_header_from_file(&f)?;
                (f, header)
            }
            Err(e) => return Err(e),
        };
        Ok(Self {
            file,
            header: header.into(),
        })
    }

    pub fn alloc_bucket_page(&self) -> Page {
        let mut header = self.header.write().unwrap();
        let page_id = header.inc_page_count();
        T::write_header_to_file(&self.file, &header).expect("write header failed");
        let bytes = get_init_page_bytes(PageType::Bucket);
        write_at_impl(&self.file, &bytes, Self::page_offset(page_id)).expect("write failed");
        Page {
            id: page_id,
            pin_count: 0.into(),
            dirty: false.into(),
            data: bytes.into(),
        }
    }

    pub fn write_page(&self, page_id: u32, data: &[u8; PAGE_SIZE]) -> io::Result<()> {
        write_at_impl(&self.file, data, Self::page_offset(page_id))
    }

    pub fn read_page(&self, page_id: u32) -> io::Result<[u8; PAGE_SIZE]> {
        let mut buf = [0; PAGE_SIZE];
        read_exact_at_impl(&self.file, &mut buf, Self::page_offset(page_id))?;
        Ok(buf)
    }

    fn page_offset(page_id: u32) -> u64 {
        T::header_size() as u64 + page_id as u64 * PAGE_SIZE as u64
    }
}

trait FileHeader {
    fn new() -> Self;

    fn write_header_to_file(file: &File, header: &Self) -> io::Result<()>;

    fn read_header_from_file(file: &File) -> io::Result<Self>
    where
        Self: Sized;

    fn header_size() -> usize;

    fn page_count(&self) -> u32;
    fn inc_page_count(&mut self) -> u32;
}
