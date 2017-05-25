use std::collections::hash_map::DefaultHasher;
use errors::Error;
use std::hash::{Hash, Hasher};
use memmap::{Mmap, Protection};
use std::fs::{OpenOptions, metadata};
use fs2::FileExt;
use std::{mem, ptr, slice};
use constants::*;
use std::path::Path;

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

//--------------------------------------------------------------------
// Helper methods
//--------------------------------------------------------------------
/// Get the raw byte representation of a struct
unsafe fn to_slice<'a, T>(p: *const u8) -> &'a [u8] {
    slice::from_raw_parts(p, mem::size_of::<T>())
}

/// Create an struct from the pointer
fn from_ptr<T>(p: *const u8) -> T {
    unsafe { ptr::read(p as *const T) }
}

/// Create an struct from the pointer
fn from_ptr_with_offset<T>(p: *const u8, offset: isize) -> T {
    unsafe { ptr::read(p.offset(offset) as *const T) }
}


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

    /// Checks if a given page is within the bounds of the memory map
    fn check_bounds(&self, id: PageId) {
        assert!(self.data.len() >= OS_PAGE_SIZE as usize * id as usize);
    }

    /// Returns a pointer to the specific page of the mapped memory.
    unsafe fn page_ptr(&self, id: PageId) -> *const u8 {
        if id == 0 {
            return self.data.ptr();
        }
        self.check_bounds(id);
        let offset = OS_PAGE_SIZE as isize * id as isize;
        self.data.ptr().offset(offset)
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
        const PAGE_LOOKUP    = 2,
        /// Page containg the actual skip list
        const PAGE_LIST      = 4,
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

/// A page is usually 4096 bytes and maps to a block on the physical disk. Since
/// we are using mmap this will point to a location on the mmap.
///
/// Page layout in memory
///
///    --------------------------------------------------------------------------
///   | flags (32) | overflow page (32) | page specific data (4096 - 64 = 4032) |
///   --------------------------------------------------------------------------

const OFFSET_OVERFLOW: isize = 32;
const OFFSET_PAGEDATA: isize = 64;

struct Page {
    ptr: PagePtr,
    id: PageId,
}

impl Page {
    /// Create a new page from the pointer
    fn new(id: PageId, ptr: MutPagePtr) -> Page {
        Page { ptr: ptr, id: id }
    }

    /// Returns the overflow page if one exists
    fn overflow_page(&self) -> Option<PageId> {
        match from_ptr_with_offset::<PageId>(self.ptr, OFFSET_OVERFLOW) {
            x if x > 0 => Some(x),
            _ => None,
        }
    }

    /// Returns the type of the page from the pointer to the beginning of the page
    fn page_flags(&self) -> PageFlags {
        from_ptr(self.ptr)
    }
}

/// PageArray contains pointers to the first element of each data page. Page are
/// 4096 bytes and can contain upto 200 keys. So, PageArray will have pointers
/// to each of the valid pages in the system.
/// A level can span across multiple pages. It offers functionality similar to
/// a skiplist level but does use probablity to determine which keys should be promoted
/// to higher level. In this sense it allows faster search using Binary search.
///
/// Due to the inherent array like nature these offer O(1) access to an element.
#[derive(Debug)]
struct PageArray {
    /// Pointers to all the data pages
    page_ptrs: Vec<*const u8>,
    /// Overall capactity across all the pages
    capacity: i64,
    /// Currently occupied elements
    length: i64,
}

impl PageArray {
    fn new(ptr: PagePtr) -> PageArray {
        //assert!(Db::page_type_from_ptr(ptr) == PAGE_LOOKUP);
        // Lookup { page: vec![page], capacity: 0, length: 0, element: vec![] }
        unimplemented!()
    }
}

