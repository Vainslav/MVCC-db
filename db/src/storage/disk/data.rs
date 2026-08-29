use std::{fs::File, io, ops::Range};

use crate::{
    storage::disk::FileHeader,
    write_read_impl::{read_exact_at_impl, write_at_impl},
};

const DATA_FILE_HEADER_SIZE: usize = std::mem::size_of::<u32>(); // page_count

const PAGE_COUNT_RANGE: Range<usize> = 0..4;

pub struct DataFileHeader {
    page_count: u16,
}

impl FileHeader for DataFileHeader {
    fn new() -> Self {
        DataFileHeader { page_count: 0 }
    }

    fn write_header_to_file(file: &File, header: &Self) -> io::Result<()> {
        let mut buf = [0u8; DATA_FILE_HEADER_SIZE];
        buf[PAGE_COUNT_RANGE].copy_from_slice(&header.page_count.to_le_bytes());
        write_at_impl(&file, &buf, 0)
    }

    fn read_header_from_file(file: &File) -> io::Result<Self>
    where
        Self: Sized,
    {
        let mut buf = [0u8; DATA_FILE_HEADER_SIZE];
        read_exact_at_impl(&file, &mut buf, 0)?;
        Ok(DataFileHeader {
            page_count: u16::from_le_bytes(buf[PAGE_COUNT_RANGE].try_into().unwrap()),
        })
    }

    fn header_size() -> usize {
        DATA_FILE_HEADER_SIZE
    }

    fn page_count(&self) -> u16 {
        self.page_count
    }

    fn inc_page_count(&mut self) -> u16 {
        let prev = self.page_count;
        self.page_count += 1;
        prev
    }
}
