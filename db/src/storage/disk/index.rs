use std::{fs::File, io, ops::Range};

use crate::{
    storage::disk::FileHeader,
    write_read_impl::{read_exact_at_impl, write_at_impl},
};

const NEXT_FREE_RANGE: Range<usize> = 0..8;
const BUCKET_COUNT_RANGE: Range<usize> = 8..12;
const PAGE_COUNT_RANGE: Range<usize> = 12..16;

const INDEX_FILE_HEADER_SIZE: usize = std::mem::size_of::<u32>() + // bucket_count
    std::mem::size_of::<u32>() + // page_count
    std::mem::size_of::<u64>(); // next_free

#[derive(Debug, PartialEq, Eq)]
#[repr(C)]
pub struct IndexFileHeader {
    next_free: u64,
    bucket_count: u32,
    page_count: u32,
}

impl FileHeader for IndexFileHeader {
    fn new() -> Self {
        IndexFileHeader {
            next_free: INDEX_FILE_HEADER_SIZE as u64,
            bucket_count: 0,
            page_count: 0,
        }
    }

    fn write_header_to_file(file: &File, header: &Self) -> io::Result<()> {
        let mut buf = [0u8; INDEX_FILE_HEADER_SIZE];
        buf[NEXT_FREE_RANGE].copy_from_slice(&header.next_free.to_le_bytes());
        buf[BUCKET_COUNT_RANGE].copy_from_slice(&header.bucket_count.to_le_bytes());
        buf[PAGE_COUNT_RANGE].copy_from_slice(&header.page_count.to_le_bytes());
        write_at_impl(&file, &buf, 0)
    }

    fn read_header_from_file(file: &File) -> io::Result<Self>
    where
        Self: Sized,
    {
        let mut buf = [0u8; INDEX_FILE_HEADER_SIZE];
        read_exact_at_impl(&file, &mut buf, 0)?;
        Ok(IndexFileHeader {
            next_free: u64::from_le_bytes(buf[NEXT_FREE_RANGE].try_into().unwrap()),
            bucket_count: u32::from_le_bytes(buf[BUCKET_COUNT_RANGE].try_into().unwrap()),
            page_count: u32::from_le_bytes(buf[PAGE_COUNT_RANGE].try_into().unwrap()),
        })
    }

    fn header_size() -> usize {
        INDEX_FILE_HEADER_SIZE
    }

    fn page_count(&self) -> u32 {
        self.page_count
    }

    fn inc_page_count(&mut self) -> u32 {
        let prev = self.page_count;
        self.page_count += 1;
        prev
    }
}

#[cfg(test)]
mod tests {

    use std::io::{Read, Write};

    use tempfile::NamedTempFile;

    use super::*;

    fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
        unsafe { ::core::slice::from_raw_parts(
            (p as *const T) as *const u8,
            ::core::mem::size_of::<T>(),
        ) }
    }

    #[test]
    fn test_read_header_from_file() {
        let mut file = tempfile::tempfile().unwrap();

        file.write(any_as_u8_slice(&IndexFileHeader::new()));

        let header = IndexFileHeader::read_header_from_file(&file).unwrap();

        assert_eq!(header, IndexFileHeader::new())
    }

    #[test]
    fn test_write_header_to_file() {
        let mut file = tempfile::tempfile().unwrap();

        let header = IndexFileHeader::new();

        IndexFileHeader::write_header_to_file(&file, &header).unwrap();

        let mut buf = [0u8; INDEX_FILE_HEADER_SIZE];

        file.read(&mut buf).unwrap();    

        assert_eq!(any_as_u8_slice(&header), buf);
    }
}