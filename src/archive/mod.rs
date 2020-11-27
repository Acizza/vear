pub mod extract;

use anyhow::{anyhow, Context, Result};
use chardetng::EncodingDetector;
use encoding_rs::Encoding;
use parking_lot::Mutex;
use std::fs::File;
use std::{
    borrow::Cow,
    ops::{Deref, Index},
    path::PathBuf,
};
use std::{io::Read, io::Seek, path::Path};
use zip::{read::ZipFile, ZipArchive};

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct NodeID(usize);

impl NodeID {
    #[inline(always)]
    pub const fn first() -> Self {
        Self(0)
    }
}

impl Deref for NodeID {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct Archive {
    inner: Mutex<ZipArchive<File>>,
    pub files: ArchiveEntries,
}

impl Archive {
    pub fn read<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let file = File::open(path).context("failed to open archive")?;
        let mut archive = ZipArchive::new(file).context("failed to parse archive")?;
        let files = ArchiveEntries::read(&mut archive)?;

        Ok(Self {
            inner: Mutex::new(archive),
            files,
        })
    }
}

impl Index<NodeID> for Archive {
    type Output = ArchiveEntry;

    fn index(&self, index: NodeID) -> &Self::Output {
        &self.files[index]
    }
}

pub struct ArchiveEntries(Vec<ArchiveEntry>);

impl ArchiveEntries {
    fn new(capacity: usize) -> Self {
        let mut entries = Vec::with_capacity(1 + capacity);
        entries.push(ArchiveEntry::root());

        Self(entries)
    }

    #[inline(always)]
    fn push_entry(&mut self, node: ArchiveEntry) -> NodeID {
        let next = NodeID(self.len());
        self.0.push(node);
        next
    }

    // TODO: make generic over archive type
    fn read<R>(archive: &mut ZipArchive<R>) -> Result<Self>
    where
        R: Read + Seek,
    {
        let mut entries = Self::new(archive.len());

        for i in 0..archive.len() {
            let file = archive
                .by_index(i)
                .with_context(|| anyhow!("failed to get archive file at index {}", i))?;

            let (path, encoding) = Self::decode_filename(file.name_raw());

            let mut cur_node = NodeID::first();

            for component in path.split_terminator('/') {
                let existing_pos = entries[cur_node]
                    .children
                    .iter()
                    .find(|&&id| entries[id].name == component)
                    .cloned();

                let next_node_pos = existing_pos.unwrap_or_else(|| {
                    let mut entry = ArchiveEntry::from_path(component, i, &path, encoding, &file);
                    entry.parent = Some(cur_node);

                    let id = entries.push_entry(entry);
                    entries.0[*cur_node].children.push(id);
                    id
                });

                cur_node = next_node_pos;
            }
        }

        Ok(entries)
    }

    fn decode_filename(bytes: &[u8]) -> (Cow<str>, &'static Encoding) {
        let mut detector = EncodingDetector::new();
        detector.feed(bytes, true);
        let encoding = detector.guess(None, true);

        let (name, encoding, _) = encoding.decode(bytes);
        (name, encoding)
    }

    #[inline(always)]
    pub fn children_iter<'a, I>(&self, nodes: I) -> ChildrenIter
    where
        I: IntoIterator<Item = &'a NodeID>,
    {
        ChildrenIter::new(nodes, &self)
    }
}

impl Deref for ArchiveEntries {
    type Target = Vec<ArchiveEntry>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Index<NodeID> for ArchiveEntries {
    type Output = ArchiveEntry;

    fn index(&self, index: NodeID) -> &Self::Output {
        &self.0[*index]
    }
}

pub struct ChildrenIter<'a> {
    queue: Vec<(NodeID, PathBuf)>,
    files: &'a ArchiveEntries,
}

impl<'a> ChildrenIter<'a> {
    fn new<'b, I>(base_nodes: I, files: &'a ArchiveEntries) -> Self
    where
        I: IntoIterator<Item = &'b NodeID>,
    {
        let mut queue = Vec::with_capacity(32);

        for node in base_nodes {
            queue.push((*node, PathBuf::new()));
        }

        Self { queue, files }
    }
}

impl<'a> Iterator for ChildrenIter<'a> {
    type Item = (NodeID, &'a ArchiveEntry, PathBuf);

    fn next(&mut self) -> Option<Self::Item> {
        let (id, mut path) = self.queue.pop()?;
        let node = &self.files[id];

        if is_valid_filename(&node.name) {
            path.push(&node.name);
        }

        for &child in &node.children {
            self.queue.push((child, path.clone()));
        }

        Some((id, node, path))
    }
}

fn is_valid_filename(name: &str) -> bool {
    !matches!(name, ".." | "." | "/")
}

#[derive(Clone)]
pub struct ArchiveEntry {
    pub name: String,
    pub props: EntryProperties,
    pub last_modified: Option<Date>,
    pub encoding: &'static Encoding,
    pub entry_num: usize,
    pub parent: Option<NodeID>,
    pub children: Vec<NodeID>,
}

impl ArchiveEntry {
    pub fn new<S>(
        name: S,
        entry_num: usize,
        props: EntryProperties,
        last_modified: Option<Date>,
        encoding: &'static Encoding,
    ) -> Self
    where
        S: Into<String>,
    {
        Self {
            name: name.into(),
            entry_num,
            props,
            last_modified,
            encoding,
            parent: None,
            children: Vec::new(),
        }
    }

    pub fn new_directory<S>(
        name: S,
        entry_num: usize,
        last_modified: Option<Date>,
        encoding: &'static Encoding,
    ) -> Self
    where
        S: Into<String>,
    {
        Self::new(
            name,
            entry_num,
            EntryProperties::Directory,
            last_modified,
            encoding,
        )
    }

    pub fn root() -> Self {
        Self::new_directory("/", 0, None, encoding_rs::UTF_8)
    }

    /// Create a new `ArchiveEntry` from a specific file path in an archive.
    ///
    /// The `path` should be the full path of the given `file`, and the
    /// `name` should be a slice from the given `path`. This ensures
    /// that directories and files are detected properly.
    fn from_path<S, P>(
        name: S,
        entry_num: usize,
        path: P,
        encoding: &'static Encoding,
        file: &ZipFile,
    ) -> Self
    where
        S: Into<String>,
        P: AsRef<str>,
    {
        let name = name.into();
        let path = path.as_ref();

        let props = if path.ends_with(&name) {
            EntryProperties::file(file)
        } else {
            EntryProperties::Directory
        };

        Self::new(
            name,
            entry_num,
            props,
            Some(file.last_modified().into()),
            encoding,
        )
    }
}

#[derive(Clone)]
pub enum EntryProperties {
    Directory,
    File(FileProperties),
}

impl EntryProperties {
    fn file(file: &ZipFile) -> Self {
        Self::File(file.into())
    }

    pub fn is_dir(&self) -> bool {
        match self {
            Self::Directory => true,
            Self::File(_) => false,
        }
    }
}

#[derive(Clone)]
pub struct FileProperties {
    pub raw_size_bytes: u64,
    pub compressed_size_bytes: u64,
}

impl<'a> From<&ZipFile<'a>> for FileProperties {
    fn from(file: &ZipFile<'a>) -> Self {
        Self {
            raw_size_bytes: file.size(),
            compressed_size_bytes: file.compressed_size(),
        }
    }
}

#[derive(Clone)]
pub struct Date {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
}

impl From<zip::DateTime> for Date {
    fn from(date: zip::DateTime) -> Self {
        Self {
            year: date.year(),
            month: date.month(),
            day: date.day(),
            hour: date.hour(),
            minute: date.minute(),
        }
    }
}
