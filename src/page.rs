use std::mem::size_of;
use db::Meta;
use constants::*;
use std::ptr;

// A page number in the database
pub type PageId = u64;

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
#[repr(C, packed)]
pub struct PageHeader {
    id: PageId,
    flags: u16,
    count: u16,
    overflow: u32,
}

pub struct Page<'a> {
    pub header: PageHeader,
    pub data: &'a [u8],
}

// BranchPageElement represents a node on a branch page
#[derive(Debug)]
struct BranchPageElement {
    position: u32,
    key_size: u32,
    page_id: PageId,
}

impl BranchPageElement {
    fn from_page(page: &Page) -> BranchPageElement {
        unsafe { ptr::read(page.data.as_ptr() as *const _) }
    }
}

// leafPageElement represents a node on a leaf page.
#[derive(Debug)]
struct LeafPageElement {
    flags: u32,
    position: u32,
    key_size: u32,
    vsize: u32,
}

impl LeafPageElement {
    fn from_page(page: &Page) -> LeafPageElement {
        unsafe { ptr::read(page.data.as_ptr() as *const _) }
    }
}
