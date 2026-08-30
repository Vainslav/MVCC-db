use std::{fs::File, io, ops::Range};

use crate::{
    storage::{disk::FileHeader, pages::PageType},
    write_read_impl::{read_exact_at_impl, write_at_impl},
};

const DATA_FILE_HEADER_SIZE: usize = std::mem::size_of::<u16>(); // page_count

const PAGE_COUNT_RANGE: Range<usize> = 0..2;

#[derive(Debug, PartialEq)]
pub struct DataFileHeader {
    page_count: u16,
}

impl FileHeader for DataFileHeader {
    const PAGE_TYPE: PageType = PageType::Data;

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

#[cfg(test)]
mod tests {

    use std::io::{Read, Write};

    use tempfile::NamedTempFile;

    use super::*;

    #[test]
    fn test_read_header_from_file() {
        let mut file = tempfile::tempfile().unwrap();

        file.write(&[0u8; DATA_FILE_HEADER_SIZE]);

        let header = DataFileHeader::read_header_from_file(&file).unwrap();

        assert_eq!(header, DataFileHeader::new())
    }

    #[test]
    fn test_write_header_to_file() {
        let mut file = tempfile::tempfile().unwrap();

        let header = DataFileHeader::new();

        DataFileHeader::write_header_to_file(&file, &header).unwrap();

        let mut buf = [0u8; DATA_FILE_HEADER_SIZE];

        file.read(&mut buf).unwrap();

        assert_eq!([0u8; DATA_FILE_HEADER_SIZE], buf);
    }
}
