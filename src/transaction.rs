use bucket::Bucket;
use db::Db;
use db::Meta;
use page::{PageId, Page};
use std::sync::Arc;
use std::iter::Map;

// TransactionId represents the internal transaction identifier.
pub type TransactionId = u64;

// Tx represents a read-only or read/write transaction on the database.
// Read-only transactions can be used for retrieving values for keys and creating cursors.
// Read/write transactions can create and remove buckets and create and remove keys.
//
// IMPORTANT: You must commit or rollback transactions when you are done with
// them. Pages can not be reclaimed by the writer until no more transactions
// are using them. A long running read transaction can cause the database to
// quickly grow.
#[derive(Debug)]
pub struct Transaction {
    writable: bool,
    managed: bool,
    db: Arc<Db>,
    meta: Arc<Meta>,
    root: Bucket,
    pages: Map<PageId, Arc<Page>>,
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

// TxStats represents statistics about the actions performed by the transaction.
#[derive(Debug)]
struct TxStats {
    page_count: u32,
}
