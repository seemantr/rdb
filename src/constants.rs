// Operating system page size. Ideally this should be populated
// dyanimically.
pub const OS_PAGE_SIZE: usize = 4096;

// The minimum number of keys required in a database page.
// Setting this to a larger value will place a smaller bound on the
// maximum size of a data item. Data items larger than this size will
// be pushed into overflow pages instead of being stored directly in
// the B-tree node. This value used to default to 4. With a page size
// of 4096 bytes that meant that any item larger than 1024 bytes would
// go into an overflow page. That also meant that on average 2-3KB of
// each overflow page was wasted space. The value cannot be lower than
// 2 because then there would no longer be a tree structure. With this
// value, items larger than 2KB will go into overflow pages, and on
// average only 1KB will be wasted.
pub const MIN_KEYS_PER_PAGE: u16 = 2;

// A stamp that identifies a file as an Ozone DB file.
// There's nothing special about this value other than that it is easily
// recognizable, and it will reflect any byte order mismatches.
pub const MAGIC_KEY: u32 = 0xBADC0DE;
pub const MAGIC_KEY_SIZE: u64 = 32;

// The data file format version.
pub const VERSION: u32 = 1;
pub const VERSION_SIZE: u64 = 32;

// MaxKeySize is the maximum length of a key, in bytes. The database is
// idelly suited for smaller keys as we will be able to cahce more keys in
// memory. With utf8 encoding characters can be 1 to 4 bytes long, so worst
// case scenario we can have 255/4 ~= 63 character keys.
pub const MAX_KEY_SIZE: u32 = 255;

// MaxValueSize is the maximum length of a value, in bytes
pub const MAX_VALUE_SIZE: u32 = (1 << 31) - 2;

// Default size of memory map
pub const DEFAULT_MAPSIZE: u32 = 1048576;

//const BUCKET_HEADER_SIZE: u32 =;
pub const MIN_FILL_PERCENTAGE: f32 = 0.1;
pub const MAX_FILL_PERCENTAGE: f32 = 1.0;

// DefaultFillPercent is the percentage that split pages are filled.
// This value can be changed by setting Bucket.FillPercent.
pub const DEF_FILL_PERCENTAGE: f32 = 0.5;

// Number of slots in the reader table.
// This value was chosen somewhat arbitrarily. 126 readers plus a
// couple mutexes fit exactly into 8KB on my development machine.
pub const DEFAULT_READERS: u8 = 126;
