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
        pages::{PAGE_SIZE, Page, PageId, PageType, get_init_page_bytes},
    },
    write_read_impl::{read_exact_at_impl, write_at_impl},
};

pub struct DiskManager {
    index_files: Vec<DiskFile<IndexFileHeader>>,
    data_files: Vec<DiskFile<DataFileHeader>>,
}

impl DiskManager {
    pub fn init(index_file_paths: Vec<&str>, data_file_paths: Vec<&str>) -> DiskManager {
        let index_files: Vec<_> = index_file_paths
            .iter()
            .map(|path| DiskFile::<IndexFileHeader>::open(Path::new(path)).unwrap())
            .collect();
        let data_files: Vec<_> = data_file_paths
            .iter()
            .map(|path| DiskFile::<DataFileHeader>::open(Path::new(path)).unwrap())
            .collect();
        Self {
            index_files,
            data_files,
        }
    }

    pub fn read_page(&self, page_id: &PageId, page_type: PageType) -> io::Result<Page> {
        let PageId { page_num, file_id } = *page_id;
        let data = match page_type {
            PageType::Bucket => self
                .index_files
                .get(file_id as usize)
                .expect(&format!("invalid file_id: {}", file_id))
                .read_page(page_num),
            PageType::Data => self
                .data_files
                .get(file_id as usize)
                .expect(&format!("invalid file_id: {}", file_id))
                .read_page(page_num),
        }?;

        Ok(Page {
            id: *page_id,
            pin_count: 0.into(),
            dirty: false.into(),
            data: data.into(),
        })
    }

    pub fn next_writable(&self, page_type: PageType) -> io::Result<PageId> {
        let data = match page_type {
            PageType::Bucket => self.index_files.last().unwrap().last_page()?,
            PageType::Data => self.data_files.last().unwrap().last_page()?,
        };

        let file_id = (match page_type {
            PageType::Bucket => self.index_files.len(),
            PageType::Data => self.data_files.len(),
        } - 1) as u16;

        Ok(PageId {
            page_num: data.0,
            file_id,
        })
    }

    pub fn write_page(&self, page: &Page) -> io::Result<()> {
        let PageId { page_num, file_id } = page.id;
        let page_data_lock = page.data.read().unwrap();

        match page.get_page_type() {
            PageType::Bucket => self
                .index_files
                .get(file_id as usize)
                .expect(&format!("invalid file_id: {}", file_id))
                .write_page(page_num, &page_data_lock),
            PageType::Data => self
                .data_files
                .get(file_id as usize)
                .expect(&format!("invalid file_id: {}", file_id))
                .write_page(page_num, &page_data_lock),
        }
    }

    pub fn alloc_page(&mut self, page_type: PageType) -> Page {
        let (page_num, data) = match page_type {
            PageType::Bucket => self.index_files.last().unwrap().alloc_page(),
            PageType::Data => self.data_files.last().unwrap().alloc_page(),
        };

        let file_id = (match page_type {
            PageType::Bucket => self.index_files.len(),
            PageType::Data => self.data_files.len(),
        } - 1) as u16;

        let id = PageId { page_num, file_id };

        Page {
            id,
            pin_count: 0.into(),
            dirty: false.into(),
            data: data.into(),
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

        let s = Self {
            file,
            header: header.into(),
        };

        if s.header.read().unwrap().page_count() == 0 {
            match T::PAGE_TYPE {
                PageType::Bucket => {
                    for _ in 0..32 {
                        s.alloc_page();
                    }
                }
                PageType::Data => {
                    s.alloc_page();
                }
            }
        }

        Ok(s)
    }

    pub fn alloc_page(&self) -> (u16, [u8; PAGE_SIZE]) {
        let mut header = self.header.write().unwrap();
        let page_id = header.inc_page_count();
        T::write_header_to_file(&self.file, &header).expect("write header failed");
        let bytes = get_init_page_bytes(T::PAGE_TYPE);
        write_at_impl(&self.file, &bytes, Self::page_offset(page_id)).expect("write failed");
        (page_id, bytes)
    }

    pub fn write_page(&self, page_id: u16, data: &[u8; PAGE_SIZE]) -> io::Result<()> {
        write_at_impl(&self.file, data, Self::page_offset(page_id))
    }

    pub fn read_page(&self, page_id: u16) -> io::Result<[u8; PAGE_SIZE]> {
        let mut buf = [0; PAGE_SIZE];
        read_exact_at_impl(&self.file, &mut buf, Self::page_offset(page_id))?;
        Ok(buf)
    }

    pub fn last_page(&self) -> io::Result<(u16, [u8; PAGE_SIZE])> {
        let mut buf = [0; PAGE_SIZE];
        let page_id = self.header.read().unwrap().page_count() - 1;
        read_exact_at_impl(&self.file, &mut buf, Self::page_offset(page_id))?;
        Ok((page_id, buf))
    }

    fn page_offset(page_id: u16) -> u64 {
        T::header_size() as u64 + page_id as u64 * PAGE_SIZE as u64
    }
}

trait FileHeader {
    const PAGE_TYPE: PageType;

    fn new() -> Self;

    fn write_header_to_file(file: &File, header: &Self) -> io::Result<()>;

    fn read_header_from_file(file: &File) -> io::Result<Self>
    where
        Self: Sized;

    fn header_size() -> usize;

    fn page_count(&self) -> u16;
    fn inc_page_count(&mut self) -> u16;
}

#[cfg(test)]
mod tests {
    use crate::storage::pages::bucket_page::get_bucket_page_init_bytes;

    use super::*;

    use tempfile::tempdir;

    #[test]
    fn test_open() {
        let dir = tempdir().unwrap();

        let index_path = dir.path().join("index.db");
        let data_path = dir.path().join("data.db");

        let index_file = DiskFile::<IndexFileHeader>::open(&index_path).unwrap();
        let data_file = DiskFile::<DataFileHeader>::open(&data_path).unwrap();

        assert_eq!(index_file.header.read().unwrap().page_count(), 32);
        assert_eq!(data_file.header.read().unwrap().page_count(), 1);

        let mut buf = [0u8; PAGE_SIZE];

        read_exact_at_impl(
            &index_file.file,
            &mut buf,
            IndexFileHeader::header_size() as u64,
        )
        .unwrap();

        assert_eq!(buf, get_init_page_bytes(PageType::Bucket));

        read_exact_at_impl(
            &data_file.file,
            &mut buf,
            DataFileHeader::header_size() as u64,
        )
        .unwrap();

        assert_eq!(buf, get_init_page_bytes(PageType::Data));
    }

    #[test]
    fn test_alloc_page() {
        let dir = tempdir().unwrap();

        let index_path = dir.path().join("index.db");
        let index_file = DiskFile::<IndexFileHeader>::open(&index_path).unwrap();

        assert_eq!(index_file.header.read().unwrap().page_count(), 32);

        let (new_id, new_data) = index_file.alloc_page();

        assert_eq!(new_id, 32);
        assert_eq!(index_file.header.read().unwrap().page_count(), 33);

        let mut buf = [0u8; PAGE_SIZE];
        read_exact_at_impl(
            &index_file.file,
            &mut buf,
            IndexFileHeader::header_size() as u64,
        )
        .unwrap();
        assert_eq!(buf, new_data);

        let data_path = dir.path().join("data.db");
        let data_file = DiskFile::<DataFileHeader>::open(&data_path).unwrap();

        assert_eq!(data_file.header.read().unwrap().page_count(), 1);

        let (data_id, new) = data_file.alloc_page();

        assert_eq!(data_id, 1);
        assert_eq!(data_file.header.read().unwrap().page_count(), 2);

        let mut data_buf = [0u8; PAGE_SIZE];
        read_exact_at_impl(
            &data_file.file,
            &mut data_buf,
            DataFileHeader::header_size() as u64 + PAGE_SIZE as u64,
        )
        .unwrap();
        assert_eq!(data_buf, new);
    }

    #[test]
    fn test_last_page() {
        let dir = tempdir().unwrap();

        let index_path = dir.path().join("index.db");
        let data_path = dir.path().join("data.db");

        let index_file = DiskFile::<IndexFileHeader>::open(&index_path).unwrap();
        let data_file = DiskFile::<DataFileHeader>::open(&data_path).unwrap();

        assert_eq!(index_file.header.read().unwrap().page_count(), 32);
        assert_eq!(data_file.header.read().unwrap().page_count(), 1);

        let (i_id, i_data) = index_file.last_page().unwrap();
        let (d_id, d_data) = data_file.last_page().unwrap();

        assert_eq!(i_id, 31);
        assert_eq!(d_id, 0);

        assert_eq!(i_data, get_init_page_bytes(PageType::Bucket));
        assert_eq!(d_data, get_init_page_bytes(PageType::Data));
    }
    
    #[test]
    fn test_index_first_page() {
        let dir = tempdir().unwrap();

        let index_path = dir.path().join("index.db");
        
        let index_file = DiskFile::<IndexFileHeader>::open(&index_path).unwrap();

        assert_eq!(index_file.header.read().unwrap().page_count(), 32);

        let mut buf = [0; 4];

        read_exact_at_impl(
            &index_file.file,
            &mut buf,
            IndexFileHeader::header_size() as u64 + 1,
        )
        .unwrap();

        assert_eq!([0; 4], buf)
    }
}
