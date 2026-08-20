use std::{
    fs::{File, OpenOptions},
    os::unix::fs::FileExt,
    path::Path,
    sync::RwLock,
};

use crate::{
    storage::pages::{PAGE_SIZE, Page, PageType, get_init_page_bytes},
    write_read_impl::{read_exact_at_impl, write_at_impl},
};

struct IndexFileHeader {
    bucket_count: u32,
    page_count: u32,
    next_free: u64,
}

const INDEX_FILE_HEADER_SIZE: usize = std::mem::size_of::<u32>() + // bucket_count
    std::mem::size_of::<u32>() + // page_count
    std::mem::size_of::<u64>(); // next_free

pub struct IndexFile {
    file: File,
    header: RwLock<IndexFileHeader>,
}

impl IndexFile {
    pub fn open(path: &Path) -> std::io::Result<Self> {
        let (file, header) = match File::create_new(path) {
            Ok(f) => {
                let header = IndexFileHeader {
                    bucket_count: 0,
                    page_count: 0,
                    next_free: INDEX_FILE_HEADER_SIZE as u64,
                };
                write_header_to_file(&f, &header)?; // нужно реализовать симметрично read_header_from_file
                (f, header)
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                let f = OpenOptions::new().read(true).write(true).open(path)?;
                let header = read_header_from_file(&f)?;
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
        let page_id = header.page_count as usize;
        header.page_count += 1;
        write_header_to_file(&self.file, &header).expect("write header failed");
        let bytes = get_init_page_bytes(PageType::Bucket);
        self.file
            .write_at(&bytes, page_offset(page_id))
            .expect("write failed");
        Page {
            id: page_id,
            pin_count: 0.into(),
            dirty: false.into(),
            data: bytes.into(),
        }
    }

    pub fn write_page(&self, page_id: u32, data: &[u8; PAGE_SIZE]) -> std::io::Result<()> {
        write_at_impl(&self.file, data, page_offset(page_id as usize))
    }

    pub fn read_page(&self, page_id: u32) -> std::io::Result<[u8; PAGE_SIZE]> {
        let mut buf = [0; PAGE_SIZE];
        read_exact_at_impl(&self.file, &mut buf, page_offset(page_id as usize))?;
        Ok(buf)
    }
}

fn read_header_from_file(file: &File) -> std::io::Result<IndexFileHeader> {
    let mut buf = [0u8; INDEX_FILE_HEADER_SIZE];
    file.read_exact_at(&mut buf, 0)?;
    Ok(IndexFileHeader {
        bucket_count: u32::from_le_bytes(buf[0..4].try_into().unwrap()),
        page_count: u32::from_le_bytes(buf[4..8].try_into().unwrap()),
        next_free: u64::from_le_bytes(buf[8..16].try_into().unwrap()),
    })
}

fn write_header_to_file(file: &File, header: &IndexFileHeader) -> std::io::Result<()> {
    let mut buf = [0u8; INDEX_FILE_HEADER_SIZE];
    buf[0..4].copy_from_slice(&header.bucket_count.to_le_bytes());
    buf[4..8].copy_from_slice(&header.page_count.to_le_bytes());
    buf[8..16].copy_from_slice(&header.next_free.to_le_bytes());
    file.write_all_at(&buf, 0)
}

fn page_offset(page_id: usize) -> u64 {
    INDEX_FILE_HEADER_SIZE as u64 + page_id as u64 * PAGE_SIZE as u64
}
