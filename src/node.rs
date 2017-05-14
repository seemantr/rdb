
/*
// Inode represents an internal node inside of a node.
// It can be used to point to elements in a page or point
// to an element which hasn't been added to a page yet.
#[derive(Debug)]
struct InNode {
    Flags: i32,
    PageId: PageId,
    Key: &[u8],
    Value: &[u8],
}

struct Node {
    Bucket: *mut Node,
    IsLeaf: bool,
    Unbalanced: bool,
    Spilled: bool,
    Key: [u8],
    PageId: PageId,
    Parent: *mut Node,
    Children: &[Node],
    Inodes: &[InNode],
}
*/
