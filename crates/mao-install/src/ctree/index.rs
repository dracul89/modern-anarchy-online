//! The index file's B-tree.
//!
//! A node (node-size bytes, at its position in the index file) starts with
//! a 0x20-byte header:
//!
//! | offset | field |
//! |---|---|
//! | 0x00 | the next node on the same level (0: none) |
//! | 0x04 | the previous node on the same level |
//! | 0x08 | the number of entries (u16) |
//! | 0x0a | the bytes the entries take (u16) |
//! | 0x11 | 1 in a leaf |
//! | 0x1c | the highest position high word in the node |
//!
//! The index header keeps the root (0x1c), the leftmost leaf (0x48), the
//! number of levels (0x114: the engine's searches need it) and how many
//! nodes have split (0x10c).
//!
//! and its last 12 bytes are its high key (the key its parent's entry for
//! it has: all 0xff at a level's right end) and a zero word, which the
//! engine's searches go by: the entries, 16 bytes each, stop short of it.
//! The entries are a position's low word, the 8-byte key,
//! the position's high word. In a leaf the position is a record's; above
//! the leaves it is a child node's, and the key the highest key under that
//! child (the last entry of a level holds all 0xff bytes). Keys are in
//! ascending order, compared bytewise. Deleting keys can leave a leaf
//! empty: it stays in its level's chain and under its parent's entry.

use super::file::CtFile;
use super::{Error, Position, Result};

/// The size of a node's header.
pub const NODE_HEADER: usize = 0x20;
/// The size of an entry: the key and the position around it.
pub const ENTRY: usize = 16;
/// The size of a node's trailer: its high key and a zero word.
pub const TRAILER: usize = 12;
/// Deeper than any tree the files can hold (a guard against loops).
const MAX_DEPTH: usize = 32;

/// One node, decoded.
pub struct Node {
    pub next: u32,
    pub previous: u32,
    pub leaf: bool,
    /// The highest key the node can hold (from its last 12 bytes).
    pub high: [u8; 8],
    bytes: Vec<u8>,
    count: usize,
}

impl Node {
    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn key(&self, i: usize) -> &[u8; 8] {
        let at = NODE_HEADER + i * ENTRY + 4;
        self.bytes[at..at + 8].try_into().unwrap()
    }

    pub fn position(&self, i: usize) -> Position {
        let at = NODE_HEADER + i * ENTRY;
        let word = |o: usize| u32::from_le_bytes(self.bytes[o..o + 4].try_into().unwrap());
        Position { low: word(at), high: word(at + 12) }
    }
}

/// An index file.
pub struct Index {
    file: CtFile,
}

impl Index {
    pub fn new(file: CtFile) -> Index {
        Index { file }
    }

    pub fn file(&self) -> &CtFile {
        &self.file
    }

    /// The number of keys, from the header.
    pub fn keys(&self) -> u32 {
        self.file.header().keys
    }

    pub fn node(&self, at: u32) -> Result<Node> {
        let size = self.file.header().node_size as usize;
        if size < NODE_HEADER + ENTRY + TRAILER {
            return Err(Error::BAD_NODE);
        }
        let mut bytes = vec![0; size];
        self.file.read(at, &mut bytes)?;
        let count = u16::from_le_bytes([bytes[8], bytes[9]]) as usize;
        if NODE_HEADER + count * ENTRY > size - TRAILER {
            return Err(Error::BAD_NODE);
        }
        let word = |o: usize| u32::from_le_bytes(bytes[o..o + 4].try_into().unwrap());
        let high = bytes[size - TRAILER..size - TRAILER + 8].try_into().unwrap();
        Ok(Node { next: word(0), previous: word(4), leaf: bytes[0x11] == 1, high, count, bytes })
    }

    /// The leaf that would hold `key`, and the first entry in it not below
    /// `key` (its length when there is none); None in an empty tree.
    pub fn seek(&self, key: &[u8; 8]) -> Result<Option<(Node, usize)>> {
        let mut at = self.file.header().root;
        if at == 0 {
            return Ok(None);
        }
        for _ in 0..MAX_DEPTH {
            let node = self.node(at)?;
            let i = (0..node.len()).find(|&i| node.key(i) >= key).unwrap_or(node.len());
            if node.leaf {
                return Ok(Some((node, i)));
            }
            if i == node.len() {
                return Ok(None);
            }
            at = node.position(i).low;
        }
        Err(Error::BAD_NODE)
    }

    /// The first entry at or after entry `i` of `node`, following the leaf
    /// chain past the node's end (and past empty leaves).
    fn forward(&self, mut node: Node, mut i: usize) -> Result<Option<([u8; 8], Position)>> {
        for _ in 0..=u16::MAX {
            if i < node.len() {
                return Ok(Some((*node.key(i), node.position(i))));
            }
            if node.next == 0 {
                return Ok(None);
            }
            node = self.node(node.next)?;
            i = 0;
        }
        Err(Error::BAD_NODE)
    }

    /// The last entry before entry `i` of `node`, following the leaf chain
    /// backwards.
    fn backward(&self, mut node: Node, mut i: usize) -> Result<Option<([u8; 8], Position)>> {
        for _ in 0..=u16::MAX {
            if i > 0 {
                return Ok(Some((*node.key(i - 1), node.position(i - 1))));
            }
            if node.previous == 0 {
                return Ok(None);
            }
            node = self.node(node.previous)?;
            i = node.len();
        }
        Err(Error::BAD_NODE)
    }

    /// The leaf at the left (or right) edge of the tree.
    fn edge(&self, right: bool) -> Result<Option<Node>> {
        let mut at = self.file.header().root;
        if at == 0 {
            return Ok(None);
        }
        for _ in 0..MAX_DEPTH {
            let node = self.node(at)?;
            if node.leaf {
                return Ok(Some(node));
            }
            if node.is_empty() {
                return Err(Error::BAD_NODE);
            }
            at = node.position(if right { node.len() - 1 } else { 0 }).low;
        }
        Err(Error::BAD_NODE)
    }

    /// The smallest key.
    pub fn first(&self) -> Result<Option<([u8; 8], Position)>> {
        match self.edge(false)? {
            Some(leaf) => self.forward(leaf, 0),
            None => Ok(None),
        }
    }

    /// The largest key.
    pub fn last(&self) -> Result<Option<([u8; 8], Position)>> {
        match self.edge(true)? {
            Some(leaf) => {
                let n = leaf.len();
                self.backward(leaf, n)
            }
            None => Ok(None),
        }
    }

    /// The smallest key not below `key`.
    pub fn at_or_after(&self, key: &[u8; 8]) -> Result<Option<([u8; 8], Position)>> {
        match self.seek(key)? {
            Some((leaf, i)) => self.forward(leaf, i),
            None => Ok(None),
        }
    }

    /// The smallest key above `key`.
    pub fn after(&self, key: &[u8; 8]) -> Result<Option<([u8; 8], Position)>> {
        match self.seek(key)? {
            Some((leaf, i)) => {
                let i = if i < leaf.len() && leaf.key(i) == key { i + 1 } else { i };
                self.forward(leaf, i)
            }
            None => Ok(None),
        }
    }

    /// The largest key below `key`.
    pub fn before(&self, key: &[u8; 8]) -> Result<Option<([u8; 8], Position)>> {
        match self.seek(key)? {
            Some((leaf, i)) => self.backward(leaf, i),
            None => self.last(),
        }
    }

    /// The position the index keeps for `key`.
    pub fn find(&self, key: &[u8; 8]) -> Result<Option<Position>> {
        Ok(match self.seek(key)? {
            Some((node, i)) if i < node.len() && node.key(i) == key => Some(node.position(i)),
            _ => None,
        })
    }
}

/// The key the last entry of a level's last node holds.
pub const HIGH_KEY: [u8; 8] = [0xff; 8];

impl Node {
    /// An empty node of `size` bytes.
    pub fn empty(size: usize, leaf: bool) -> Node {
        Node { next: 0, previous: 0, leaf, high: HIGH_KEY, bytes: vec![0; size], count: 0 }
    }

    /// How many entries fit in a node of `size` bytes.
    pub fn capacity(size: usize) -> usize {
        (size - NODE_HEADER - TRAILER) / ENTRY
    }

    fn entry(&self, i: usize) -> [u8; ENTRY] {
        let at = NODE_HEADER + i * ENTRY;
        self.bytes[at..at + ENTRY].try_into().unwrap()
    }

    fn put_entry(&mut self, i: usize, entry: &[u8; ENTRY]) {
        let at = NODE_HEADER + i * ENTRY;
        self.bytes[at..at + ENTRY].copy_from_slice(entry);
    }

    fn make_entry(key: &[u8; 8], position: Position) -> [u8; ENTRY] {
        let mut e = [0; ENTRY];
        e[..4].copy_from_slice(&position.low.to_le_bytes());
        e[4..12].copy_from_slice(key);
        e[12..].copy_from_slice(&position.high.to_le_bytes());
        e
    }

    /// Puts an entry in at `i`, moving the later ones up (there may be one
    /// more than fit until the node is split).
    pub fn insert(&mut self, i: usize, key: &[u8; 8], position: Position) {
        if NODE_HEADER + (self.count + 1) * ENTRY > self.bytes.len() {
            self.bytes.resize(NODE_HEADER + (self.count + 1) * ENTRY, 0);
        }
        for j in (i..self.count).rev() {
            let e = self.entry(j);
            self.put_entry(j + 1, &e);
        }
        self.put_entry(i, &Node::make_entry(key, position));
        self.count += 1;
    }

    pub fn remove(&mut self, i: usize) {
        for j in i + 1..self.count {
            let e = self.entry(j);
            self.put_entry(j - 1, &e);
        }
        self.count -= 1;
        self.put_entry(self.count, &[0; ENTRY]);
    }

    pub fn set_position(&mut self, i: usize, position: Position) {
        let key = *self.key(i);
        self.put_entry(i, &Node::make_entry(&key, position));
    }

    /// The node's bytes as they go in the file (`size` of them).
    fn to_bytes(&self, size: usize) -> Vec<u8> {
        let mut b = self.bytes.clone();
        b.resize(size, 0);
        b[0..4].copy_from_slice(&self.next.to_le_bytes());
        b[4..8].copy_from_slice(&self.previous.to_le_bytes());
        b[8..10].copy_from_slice(&(self.count as u16).to_le_bytes());
        b[10..12].copy_from_slice(&((self.count * ENTRY) as u16).to_le_bytes());
        b[0x11] = self.leaf as u8;
        b[size - TRAILER..size - TRAILER + 8].copy_from_slice(&self.high);
        b[size - 4..].copy_from_slice(&[0; 4]);
        b
    }

    /// Moves the entries from `from` on into a new node.
    fn split_off(&mut self, from: usize) -> Node {
        let mut right = Node::empty(self.bytes.len().max(NODE_HEADER), self.leaf);
        right.high = self.high;
        for j in from..self.count {
            let e = self.entry(j);
            right.put_entry(j - from, &e);
            self.put_entry(j, &[0; ENTRY]);
        }
        right.count = self.count - from;
        self.count = from;
        right
    }
}

impl Index {
    pub fn file_mut(&mut self) -> &mut CtFile {
        &mut self.file
    }

    fn write_node(&mut self, at: u32, node: &Node) -> Result<()> {
        let size = self.file.header().node_size as usize;
        let bytes = node.to_bytes(size);
        self.file.write(at, &bytes)
    }

    /// A new node at the end of the file.
    fn new_node(&mut self) -> Result<u32> {
        let size = self.file.header().node_size;
        let at = self.file.header().in_use.checked_add(1).ok_or(Error::FULL)?;
        let last = at.checked_add(size - 1).ok_or(Error::FULL)?;
        self.file.grow(last)?;
        self.file.set_word(0x08, last);
        self.file.count_change();
        Ok(at)
    }

    /// Adds `key` (which must not be there yet) for `position`. A full
    /// node splits in two halves, the new one to its right in its level's
    /// chain, and its parent gets an entry for the left half's highest key;
    /// a root that splits gets a new root above it.
    pub fn insert(&mut self, key: &[u8; 8], position: Position) -> Result<()> {
        let size = self.file.header().node_size as usize;
        let root = self.file.header().root;
        if root == 0 {
            let at = self.new_node()?;
            let mut leaf = Node::empty(size, true);
            leaf.insert(0, key, position);
            self.write_node(at, &leaf)?;
            self.file.set_word(0x1c, at);
            self.file.set_word(0x48, at);
            self.file.set_word(0x114, 1);
        } else {
            // Down to the leaf, remembering the way.
            let mut path: Vec<(u32, Node, usize)> = Vec::new();
            let mut raised = Vec::new();
            let mut at = root;
            let mut node = self.node(at)?;
            while !node.leaf {
                if path.len() >= MAX_DEPTH || node.is_empty() {
                    return Err(Error::BAD_NODE);
                }
                let mut i = (0..node.len()).find(|&i| node.key(i) >= key).unwrap_or(node.len());
                if i == node.len() {
                    // Past the level's highest key (a tree without the 0xff
                    // entry): the last child's key goes up to this one.
                    i -= 1;
                    let child = node.position(i);
                    node.remove(i);
                    node.insert(i, key, child);
                    raised.push(at);
                }
                let child = node.position(i).low;
                let raise = raised.last() == Some(&at);
                path.push((at, node, i));
                at = child;
                node = self.node(at)?;
                if raise && node.high.as_slice() < key.as_slice() {
                    node.high = *key;
                    if !node.leaf {
                        raised.push(at);
                    }
                }
            }
            let i = (0..node.len()).find(|&i| node.key(i) >= key).unwrap_or(node.len());
            if i < node.len() && node.key(i) == key {
                return Err(Error::DUPLICATE);
            }
            node.insert(i, key, position);
            loop {
                if node.len() <= Node::capacity(size) {
                    self.write_node(at, &node)?;
                    break;
                }
                let right_at = self.new_node()?;
                let mut right = node.split_off(node.len() / 2);
                right.next = node.next;
                right.previous = at;
                if node.next != 0 {
                    let mut after = self.node(node.next)?;
                    after.previous = right_at;
                    self.write_node(node.next, &after)?;
                }
                node.next = right_at;
                let left_high = *node.key(node.len() - 1);
                node.high = left_high;
                self.write_node(at, &node)?;
                self.write_node(right_at, &right)?;
                let splits = self.file.word(0x10c).wrapping_add(1);
                self.file.set_word(0x10c, splits);
                match path.pop() {
                    Some((parent_at, mut parent, slot)) => {
                        parent.set_position(slot, Position { low: right_at, high: 0 });
                        parent.insert(slot, &left_high, Position { low: at, high: 0 });
                        at = parent_at;
                        node = parent;
                    }
                    None => {
                        raised.clear();
                        let new_root = self.new_node()?;
                        let mut top = Node::empty(size, false);
                        top.insert(0, &left_high, Position { low: at, high: 0 });
                        top.insert(1, &HIGH_KEY, Position { low: right_at, high: 0 });
                        self.write_node(new_root, &top)?;
                        self.file.set_word(0x1c, new_root);
                        let depth = self.file.word(0x114).wrapping_add(1);
                        self.file.set_word(0x114, depth);
                        break;
                    }
                }
            }
            // Parents whose highest key went up, above the splits.
            for (parent_at, parent, _) in path {
                if raised.contains(&parent_at) {
                    self.write_node(parent_at, &parent)?;
                }
            }
        }
        let keys = self.file.header().keys.wrapping_add(1);
        self.file.set_word(0x18, keys);
        self.file.count_change();
        Ok(())
    }

    /// Takes `key` out (its leaf may be left empty). Whether it was there.
    pub fn remove(&mut self, key: &[u8; 8]) -> Result<bool> {
        let Some((leaf, i)) = self.seek_at(key)? else { return Ok(false) };
        let (at, mut node) = leaf;
        if i >= node.len() || node.key(i) != key {
            return Ok(false);
        }
        node.remove(i);
        self.write_node(at, &node)?;
        let keys = self.file.header().keys.wrapping_sub(1);
        self.file.set_word(0x18, keys);
        self.file.count_change();
        Ok(true)
    }

    /// Points `key` at `position` instead.
    pub fn set_position(&mut self, key: &[u8; 8], position: Position) -> Result<bool> {
        let Some(((at, mut node), i)) = self.seek_at(key)? else { return Ok(false) };
        if i >= node.len() || node.key(i) != key {
            return Ok(false);
        }
        node.set_position(i, position);
        self.write_node(at, &node)?;
        Ok(true)
    }

    /// As `seek`, with the leaf's position.
    fn seek_at(&self, key: &[u8; 8]) -> Result<Option<((u32, Node), usize)>> {
        let mut at = self.file.header().root;
        if at == 0 {
            return Ok(None);
        }
        for _ in 0..MAX_DEPTH {
            let node = self.node(at)?;
            let i = (0..node.len()).find(|&i| node.key(i) >= key).unwrap_or(node.len());
            if node.leaf {
                return Ok(Some(((at, node), i)));
            }
            if i == node.len() {
                return Ok(None);
            }
            at = node.position(i).low;
        }
        Err(Error::BAD_NODE)
    }
}
