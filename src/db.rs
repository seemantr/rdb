use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use errors::DbError;
use std::io::Error;
use std::hash::{Hash, Hasher};
use std::time::Duration;
use memmap::{Mmap, Protection};
use std::fs::{OpenOptions, metadata};
use std::io::prelude::*;
use fs2::FileExt;
use std::os;
use std::mem;
use std::ptr;
use constants::*;
use page::PageId;
use page;

// Bucket represents the on-file representation of a bucket.
// This is stored as the "value" of a bucket key. If the bucket is small enough,
// then its root page can be stored inline in the "value", after the bucket
// header. In the case of inline buckets, the "root" will be 0.
#[derive(Debug, Copy, Clone, Hash)]
pub struct Bucket {
    // Page id of the bucket's root-level page
    root: PageId,
    // Monotonically incrementing, used by NextSequence()
    sequence: u64,
    transaction: *const Transaction,
}

// TransactionId represents the internal transaction identifier.
type TransactionId = u64;

// Tx represents a read-only or read/write transaction on the database.
// Read-only transactions can be used for retrieving values for keys and creating cursors.
// Read/write transactions can create and remove buckets and create and remove keys.
//
// IMPORTANT: You must commit or rollback transactions when you are done with
// them. Pages can not be reclaimed by the writer until no more transactions
// are using them. A long running read transaction can cause the database to
// quickly grow.
#[derive(Debug, Copy, Clone)]
pub struct Transaction {
    writable: bool,
    managed: bool,
    db: *const Db,
    meta: *const Meta,
    root: Bucket,
    //pages: *const HashMap<PageId, *const page::Page>,
    stats: TxStats,
    //CommitHandlers: [],

	// WriteFlag specifies the flag for write-related methods like WriteTo().
	// Tx opens the database file with the specified flag to copy the data.
	//
	// By default, the flag is unset, which works well for mostly in-memory
	// workloads. For databases that are much larger than available RAM,
	// set the flag to syscall.O_DIRECT to avoid trashing the page cache.
    write_flag: u32,
}

impl Transaction {
    fn init(mut self, db: *const Db) {
        self.db = db;
        //self.pages = &HashMap::new();
        //self.meta =
    }
}
// TxStats represents statistics about the actions performed by the transaction.
#[derive(Debug, Copy, Clone)]
struct TxStats {
    page_count: u32,
}

// Settings represents the options that can be set when opening a database.
pub struct Settings {
    // Timeout is the amount of time to wait to obtain a file lock.
    // When set to zero it will wait indefinitely. This option is only
    // available on Darwin and Linux.
    timeout: Duration,

    // Sets the DB.NoGrowSync flag before memory mapping the file.
    no_grow_sync: bool,

    // Open database in read-only mode. Uses flock(..., LOCK_SH |LOCK_NB) to
    // grab a shared lock (UNIX).
    read_only: bool,

    // Sets the DB.MmapFlags flag before memory mapping the file.
    mmap_flags: u32,

    // InitialMmapSize is the initial mmap size of the database
    // in bytes. Read transactions won't block write transaction
    // if the InitialMmapSize is large enough to hold database mmap
    // size. (See DB.Begin for more information)
    //
    // If <=0, the initial map size is 0.
    // If initialMmapSize is smaller than the previous database size,
    // it takes no effect.
    initial_mmap_size: u32,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            timeout: Duration::new(0, 0),
            no_grow_sync: false,
            read_only: true,
            mmap_flags: 0,
            initial_mmap_size: 0,
        }
    }
}

// DB represents a collection of buckets persisted to a file on disk.
// All data access is performed through transactions which can be obtained through the DB.
// All the functions on DB will return a ErrDatabaseNotOpen if accessed before Open() is called.
#[derive(Debug)]
pub struct Db {
    // When enabled, the database will perform a Check() after every commit.
    // A panic is issued if the database is in an inconsistent state. This
    // flag has a large performance impact so it should only be used for
    // debugging purposes.
    strict_mode: bool,

    // Setting the NoSync flag will cause the database to skip fsync()
    // calls after each commit. This can be useful when bulk loading data
    // into a database and you can restart the bulk load in the event of
    // a system failure or database corruption. Do not set this flag for
    // normal use.
    //
    // If the package global IgnoreNoSync constant is true, this value is
    // ignored.  See the comment on that constant for more details.
    //
    // THIS IS UNSAFE. PLEASE USE WITH CAUTION.
    no_sync: bool,

    // When true, skips the truncate call when growing the database.
    // Setting this to true is only safe on non-ext3/ext4 systems.
    // Skipping truncation avoids preallocation of hard drive space and
    // bypasses a truncate() and fsync() syscall on remapping.
    no_grow_sync: bool,

    // If you want to read the entire database fast, you can set MmapFlag to
    // syscall.MAP_POPULATE on Linux 2.6.23+ for sequential read-ahead.
    mmap_flags: u32,

    // MaxBatchSize is the maximum size of a batch. Default value is
    // copied from DefaultMaxBatchSize in Open.
    //
    // If <=0, disables batching.
    //
    // Do not change concurrently with calls to Batch.
    max_batch_size: u32,

    // MaxBatchDelay is the maximum delay before a batch starts.
    // Default value is copied from DefaultMaxBatchDelay in Open.
    //
    // If <=0, effectively disables batching.
    //
    // Do not change concurrently with calls to Batch.
    max_batch_delay: u32,

    // AllocSize is the amount of space allocated when the database
    // needs to create new pages. This is done to amortize the cost
    // of truncate() and fsync() when growing the data file.
    alloc_size: u32,

    path: String,
    mmap: Mmap,
    meta0: *mut Meta,
    meta1: *mut Meta,
    page_size: usize,
    opened: bool,
}

impl Db {
    fn open(path: &str, settings: Option<Settings>) -> Result<(), Error> {
        let settings = match settings {
            Some(s) => s,
            None => Default::default(),
        };

        let file = OpenOptions::new()
            .read(true)
            .write(!settings.read_only)
            .create(!settings.read_only)
            .open(path)?;

        // Lock file so that other processes using Rdb in read-write mode cannot
        // use the database  at the same time. This would cause corruption since
        // the two processes would write meta pages and free pages separately.
        // The database file is locked exclusively (only one process can grab the lock)
        // if !options.ReadOnly.
        if !settings.read_only {
            file.lock_exclusive()?;
        }

        let meta_data = metadata(path)?;
        if meta_data.len() == 0 {}

        // Set default values for later DB operations.
        //db.alloc_size = DefaultAl
        Ok(())
    }

    // init creates a new database file and initializes its meta pages.
    fn init(&mut self) -> Result<(), Error> {
        self.page_size = OS_PAGE_SIZE;
        let buffer: [u8; OS_PAGE_SIZE as usize] = [0u8; OS_PAGE_SIZE as usize];
        for n in 0..2 {}
        Ok(())
    }

    // page retrieves a page reference from the mmap based on the current page size.
    unsafe fn page(&mut self, id: PageId) -> page::Page {
        let offset = OS_PAGE_SIZE as isize * id as isize;
        let header_pointer = self.mmap.ptr().offset(offset);
        let data_pointer = self.mmap
            .ptr()
            .offset(offset + mem::size_of::<page::PageHeader>() as isize);
        page::Page {
            header: ptr::read(header_pointer as *const _),
            data: ptr::read(data_pointer as *const _),
        }
    }
}
/*

    fn meta(mut self) -> *mut Meta {
        // We have to return the meta with the highest txid which doesn't fail
	    // validation. Otherwise, we can cause errors when in fact the database is
	    // in a consistent state. metaA is the one with the higher txid.
        if *self.meta1.txid > *self.meta0.txid {
            return self.meta1.Clone();
        }
        self.meta0.Clone();
    }*/

#[derive(Debug, Copy, Clone, Hash)]
pub struct Meta {
    magic: u32,
    version: u32,
    page_size: u32,
    flags: u32,
    root: Bucket,
    freelist: PageId,
    pgid: PageId,
    txid: TransactionId,
    checksum: u64,
}

// Generic hash generator
fn hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

impl Meta {
    fn validate(&self) -> Result<(), DbError> {
        if self.magic != MAGIC_KEY {
            return Err(DbError::Invalid);
        }
        if self.version != VERSION {
            return Err(DbError::VersionMismatch);
        }
        if self.checksum != 0 && self.checksum != hash(&self) {
            return Err(DbError::Checksum);
        }
        Ok(())
    }
}
