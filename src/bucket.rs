use std::sync::Arc;
use transaction::Transaction;

// MaxKeySize is the maximum length of a key, in bytes
const MAXKEYSIZE: u32 = 32768;
// MaxValueSize is the maximum length of a value, in bytes
const MAXVALUESIZE: u32 = (1 << 31) - 2;

//const BUCKET_HEADER_SIZE: u32 =;
const MIN_FILL_PERCENTAGE: f32 = 0.1;
const MAX_FILL_PERCENTAGE: f32 = 1.0;

// DefaultFillPercent is the percentage that split pages are filled.
// This value can be changed by setting Bucket.FillPercent.
const DEF_FILL_PERCENTAGE: f32 = 0.5;

type PageId = i64;

// Bucket represents the on-file representation of a bucket.
// This is stored as the "value" of a bucket key. If the bucket is small enough,
// then its root page can be stored inline in the "value", after the bucket
// header. In the case of inline buckets, the "root" will be 0.
#[derive(Debug)]
pub struct Bucket {
    // Page id of the bucket's root-level page
    Root: PageId,
    // Monotonically incrementing, used by NextSequence()
    Sequence: u64,
    Transaction: Arc<Transaction>,
}
