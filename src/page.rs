use std::mem::size_of;
use db::Meta;

// A page number in the database
pub type PageId = u64;

const MIN_KEYS_PER_PAGE: u16 = 2;

lazy_static! {
    static ref BRANCH_PAGE_ELEMENT_SIZE : usize = size_of::<BranchPageElement>();
    static ref LEAF_PAGE_ELEMENT_SIZE : usize = size_of::<LeafPageElement>();
}

bitflags! {
// The below flags are used to represent the page type
    flags Page_Flags : u16 {
        const BRANCH        = 1,
        const LEAF          = 2,
        const META          = 4,
        const FREELIST      = 8,
        const BUCKET_LEAF   = 16,
    }
}

#[derive(Debug)]
pub struct Page {
    id: PageId,
    flags: u16,
    count: u16,
    overflow: u32,
    meta: *const Meta
}

// BranchPageElement represents a node on a branch page
#[derive(Debug)]
struct BranchPageElement {
    position: u32,
    key_size: u32,
    page_id: PageId,
}

// leafPageElement represents a node on a leaf page.
#[derive(Debug)]
struct LeafPageElement {
    flags: u32,
    position: u32,
    key_size: u32,
    vsize: u32,
}

