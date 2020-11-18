use anyhow::{anyhow, Context, Result};
use chardetng::EncodingDetector;
use encoding_rs::Encoding;
use std::path::Path;
use std::{
    borrow::Cow,
    ops::{Deref, Index},
};
use std::{fs::File, rc::Rc};
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
    pub files: ArchiveEntries,
}

impl Archive {
    pub fn read<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let files = ArchiveEntries::read(path)?;
        Ok(Self { files })
    }
}

impl Index<NodeID> for Archive {
    type Output = Rc<ArchiveEntry>;

    fn index(&self, index: NodeID) -> &Self::Output {
        &self.files[index]
    }
}

pub struct ArchiveEntries(Vec<Rc<ArchiveEntry>>);

impl ArchiveEntries {
    fn new(capacity: usize) -> Self {
        let mut entries = Vec::with_capacity(1 + capacity);
        entries.push(Rc::new(ArchiveEntry::root()));

        Self(entries)
    }

    #[inline(always)]
    fn push_entry(&mut self, node: ArchiveEntry) -> NodeID {
        let next = NodeID(self.len());
        self.0.push(Rc::new(node));
        next
    }

    // TODO: make generic over archive type
    pub fn read<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let file = File::open(path).context("failed to open archive")?;
        let mut archive = ZipArchive::new(file).context("failed to parse archive")?;

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

                let next_node_pos = match existing_pos {
                    Some(pos) => pos,
                    None => {
                        let mut entry = ArchiveEntry::from_path(component, &path, encoding, &file);
                        entry.parent = Some(cur_node);

                        let id = entries.push_entry(entry);

                        Rc::get_mut(&mut entries.0[*cur_node])
                            .unwrap()
                            .children
                            .push(id);

                        id
                    }
                };

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
}

impl Deref for ArchiveEntries {
    type Target = Vec<Rc<ArchiveEntry>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Index<NodeID> for ArchiveEntries {
    type Output = Rc<ArchiveEntry>;

    fn index(&self, index: NodeID) -> &Self::Output {
        &self.0[*index]
    }
}

#[derive(Clone)]
pub struct ArchiveEntry {
    pub name: String,
    pub props: EntryProperties,
    pub last_modified: Option<Date>,
    pub encoding: &'static Encoding,
    pub parent: Option<NodeID>,
    pub children: Vec<NodeID>,
}

impl ArchiveEntry {
    pub fn new<S>(
        name: S,
        props: EntryProperties,
        last_modified: Option<Date>,
        encoding: &'static Encoding,
    ) -> Self
    where
        S: Into<String>,
    {
        Self {
            name: name.into(),
            props,
            last_modified,
            encoding,
            parent: None,
            children: Vec::new(),
        }
    }

    pub fn new_directory<S>(
        name: S,
        last_modified: Option<Date>,
        encoding: &'static Encoding,
    ) -> Self
    where
        S: Into<String>,
    {
        Self::new(name, EntryProperties::Directory, last_modified, encoding)
    }

    pub fn root() -> Self {
        Self::new_directory("/'", None, encoding_rs::UTF_8)
    }

    /// Create a new `ArchiveEntry` from a specific file path in an archive.
    ///
    /// The `path` should be the full path of the given `file`, and the
    /// `name` should be a slice from the given `path`. This ensures
    /// that directories and files are detected properly.
    fn from_path<S, P>(name: S, path: P, encoding: &'static Encoding, file: &ZipFile) -> Self
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

        Self::new(name, props, Some(file.last_modified().into()), encoding)
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
