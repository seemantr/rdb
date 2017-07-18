use std::collections::hash_map::DefaultHasher;
use errors::Error;
use std::hash::{Hash, Hasher};
use memmap::{Mmap, Protection};
use std::fs::{OpenOptions, metadata};
use fs2::FileExt;
use std::{marker, mem, ptr, slice};
use std::ops::Deref;
use constants::*;
use std::path::Path;
use enc;

//--------------------------------------------------------------------
// Type aliases
//--------------------------------------------------------------------
// A page number in the database. u32 should be more than enough to define
// page numbers as u32 MAX value is 2147483647. Our page size is 4KB so u32
// gives us the capability to address upto 15.9 TB of data. This is more than
// enough as one should start thinking about sharding the data at anywhere near
// 1 TB mark.
type PageId = u32;

/// Pointer to the first byte of the Page.
type PagePtr = *const u8;

/// Mutable pointer to the first byte of the Page
type MutPagePtr = *mut u8;

/// Generic hash generator
fn hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}
//--------------------------------------------------------------------

/// Represents the metadata record for the database. This record
/// is appended at the beginning of each database file. It also contains
/// the header information about the database. There are two copies of
/// this at the beginning of the file.
#[derive(Debug, Copy, Clone, Hash)]
#[repr(C)]
pub struct Meta {
    /// Magic int value to idenfify if the file is for the database
    magic: u32,
    version: u32,
    flags: u32,
    page_size: u32,
    checksum: u64,
    transaction_id: u64,
    little_endian: bool,
    root: PageId,
    freelist: PageId,
    pgid: PageId,
}

impl Meta {
    fn default() -> Meta {
        Meta {
            magic: MAGIC_KEY,
            version: VERSION,
            flags: 0,
            page_size: OS_PAGE_SIZE as u32,
            checksum: 0,
            little_endian: cfg!(target_endian = "little"),
            root: 0,
            freelist: 0,
            transaction_id: 0,
            pgid: 0,
        }
    }

    // Validate that that given header is in the right format
    fn validate(&self) -> Result<(), Error> {
        if self.magic != MAGIC_KEY {
            return Err(Error::DatabaseInvalid);
        }
        if self.version != VERSION {
            return Err(Error::DatabaseVersionMismatch);
        }
        if self.checksum != 0 && self.checksum != hash(&self) {
            return Err(Error::ChecksumError);
        }
        Ok(())
    }
}

// Settings represents the options that can be set when opening a database.
#[derive(Debug)]
pub struct Settings {
    /// Create database if it doesn't exist
    auto_create: bool,

    // Open database in read-only mode. No file locks will be issued on the
    // database file.
    read_only: bool,

    // Initial Mmap Size is the initial mmap size of the database
    // in bytes. It is a helpful hint to preallocate the memmap. This
    // will avoid mmap resizing.
    // If <=0, the initial map size is the minimum required for the headers
    // and other metadata.
    // If size is smaller than the previous database size, it takes no effect.
    initial_mmap_size: u32,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            auto_create: true,
            read_only: false,
            initial_mmap_size: 0,
        }
    }
}

// DB represents the database persisted to a file on disk.
#[derive(Debug)]
pub struct Db {
    // The path to the folder which contains the database. If folder
    // doesn't exist then it will be created automatically.
    path: String,
    // The physical file containing the data related to keys
    data: Mmap,
    // Meta 0 page pointing to the root of the tree
    meta0: Meta,
    // Meta 1 page pointing to the root of the tree
    meta1: Meta,
    settings: Settings,
}

impl Db {
    /// Create a new database file
    fn create(path: &Path, settings: &Settings) -> Result<(), Error> {
        let data_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        // Set length for at least 4 pages
        data_file.set_len(OS_PAGE_SIZE as u64 * 4)?;
        let mut data_mmap = Mmap::open(&data_file, Protection::ReadWrite)?;

        // Create meta0 at page 1
        let meta0: &mut Meta = unsafe { mem::transmute(data_mmap.mut_ptr()) };
        *meta0 = Meta::default();

        // Create meta1 at page 2
        let meta1: &mut Meta =
            unsafe { mem::transmute(data_mmap.mut_ptr().offset(OS_PAGE_SIZE as isize)) };
        *meta1 = Meta::default();
        Ok(())
    }

    /// Initalize a database for use
    pub fn init(path: &Path, settings: &Settings) -> Result<(), Error> {

        let data_file = OpenOptions::new()
            .read(true)
            .write(!settings.read_only)
            .create(!settings.read_only)
            .open(path)?;

        let protection_mode = if settings.read_only {
            Protection::Read
        } else {
            Protection::ReadWrite
        };
        let data_mmap = Mmap::open(&data_file, protection_mode)?;

        // Lock file so that other processes using the database in read-write mode cannot
        // use the database  at the same time. This would cause corruption since
        // the two processes would write meta pages and free pages separately.
        // The database file is locked exclusively (only one process can grab the lock).
        if !settings.read_only {
            data_file.lock_exclusive()?;
        }

        let meta_data = metadata(path)?;
        if meta_data.len() == 0 {}

        Ok(())
    }

    /// Open a database
    ///
    /// A new database will be created if no database is found at the location.
    pub fn open(path: &str, settings: Option<Settings>) -> Result<(), Error> {
        let settings = match settings {
            Some(s) => s,
            None => Default::default(),
        };

        let data_file_path = Path::new(path);
        match (data_file_path.exists(), settings.auto_create) {
            (false, false) => Err(Error::DatabaseNotFound),
            (false, true) => Db::create(data_file_path, &settings),
            (true, _) => Db::init(data_file_path, &settings),
        }
    }
}

//--------------------------------------------------------------------
// Memory map Page management
//--------------------------------------------------------------------
bitflags! {
/// Flags used to represent the page type
    flags PageFlags : u32 {
        /// Metadata page which contains the location of the lookup page
        const PAGE_META      = 1,
        /// Page which contains the array lookup to the list pages
        const PAGE_JUMPLIST  = 2,
        /// Page containg the actual skip list
        const PAGE_KEYS      = 4,
        /// Page which contains the information about the free pages
        const PAGE_FREELIST  = 8,
        /// Page containing the value data
        const PAGE_DATA      = 16,
        /// Overflow page used in case the value is larger than the block
        const PAGE_OVERFLOW  = 32,
        /// Deleted page
        const PAGE_DELETED   = 64,
    }
}

trait PageWriter {}

/// Page array abstracts the memory map into pages of 4KB each. It can be
/// considered as the lowest unit to perform read and write operations. The idea
/// is that each page corresponds to the physical page of a disk.
#[derive(Debug)]
struct PageArray {
    data: Mmap,
}

impl PageArray {
    /// Checks if a given page is within the bounds of the memory map
    fn check_bounds(&self, id: PageId) {
        assert!(self.data.len() >= OS_PAGE_SIZE * id as usize);
    }

    /// Returns a pointer to the specific page of the mapped memory.
    fn page_ptr(&self, id: PageId) -> *const u8 {
        if id == 0 {
            return self.data.ptr();
        }
        self.check_bounds(id);
        let offset = OS_PAGE_SIZE as isize * id as isize;
        unsafe { self.data.ptr().offset(offset) }
    }

    /// Returns a mut pointer to the specific page of the mapped memory.
    unsafe fn page_mut_ptr(&mut self, id: PageId) -> *mut u8 {
        if id == 0 {
            return self.data.mut_ptr();
        }
        self.check_bounds(id);
        let offset = OS_PAGE_SIZE as isize * id as isize;
        self.data.mut_ptr().offset(offset)
    }

    /// Return the page info for a page with the given Id
    fn get_page_info(&self, id: PageId) -> PageInfo {
        PageInfo {
            ptr: self.page_ptr(id),
            id: id,
        }
    }
}


/// A page is usually 4096 bytes and maps to a block on the physical disk. Since
/// we are using mmap this will point to a location on the mmap.
///
/// Page layout in memory
///
///    --------------------------------------------------------------------------
///   | flags (32) | overflow page (32) | page specific data (4096 - 64 = 4032) |
///   --------------------------------------------------------------------------
#[derive(Debug, Clone, Copy)]
struct PageInfo {
    ptr: PagePtr,
    id: PageId,
}

const PI_OFFSET_OVERFLOW: u64 = 32;
const PI_OFFSET_PAGEDATA: u64 = 64;

impl PageInfo {
    /// Returns the overflow page if one exists
    fn overflow_page(&self) -> Option<PageId> {
        match enc::from_ptr_with_offset::<PageId>(self.ptr, PI_OFFSET_OVERFLOW) {
            x if x > 0 => Some(x),
            _ => None,
        }
    }

    /// Returns the type of the page from the pointer to the beginning of the page
    fn page_flags(&self) -> PageFlags {
        enc::from_ptr(self.ptr)
    }
}

/// PageIndex contains pointers to the first element of each data page. Page are
/// 4096 bytes and can contain upto 200 keys. So, PageIndex will have pointers
/// to each of the valid pages in the system.
/// A level can span across multiple pages. It offers functionality similar to
/// a skiplist level but does use probablity to determine which keys should be promoted
/// to higher level. In this sense it allows faster search using Binary search.
///
/// Due to the inherent array like nature these offer O(1) access to an element. The
/// elements are sorted
///
/// Page layout in memory
///
///    --------------------------------------------------------------------------
///   | header (64) | length (64) | value 0.....n (32 bytes each, n = 124)      |
///   --------------------------------------------------------------------------
/// So, we can store upto 124 pointers in a single array. This may not seem much
/// but this covers total 124 * 50 keys/kb * 4 page size = 24,800 (considering 20kb/key)
#[derive(Debug)]
struct PageIndex<'a> {
    /// Pointers to all the data pages
    page_ptrs: Vec<PageInfo>,
    /// Overall capactity across all the pages
    capacity: i64,
    /// Currently occupied elements
    length: i64,
    ///
    pa: &'a PageArray,
}

const PI_OFFSET_LENGTH: u64 = 64;
const PI_KEYS_PER_PAGE: usize = 124;

impl<'a> PageIndex<'a> {
    fn length(page: PageInfo) -> u32 {
        enc::from_ptr_with_offset::<PageId>(page.ptr, PI_OFFSET_LENGTH)
    }

    fn new(page: PageInfo, pa: &'a PageArray) -> PageIndex {
        let mut pages = vec![];
        let mut p = Some(page);
        let mut length = 0;
        loop {
            match p {
                Some(p1) => {
                    pages.push(p1);
                    length += PageIndex::length(p1);

                    // Grab the next page, if it is found then load it
                    match p1.overflow_page() {
                        Some(next_page) => p = Some(pa.get_page_info(next_page)),
                        None => break,
                    }
                }
                None => break,
            }
        }

        let capacity = (pages.len() * PI_KEYS_PER_PAGE) as i64;
        PageIndex {
            page_ptrs: pages,
            capacity: capacity,
            length: length as i64,
            pa: pa,
        }
    }
}
