use anyhow::{anyhow, Context, Result};
use std::ops::{Deref, Index};
use std::path::{Path, PathBuf};
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

#[derive(Debug)]
pub struct ArchiveEntries(Vec<Rc<ArchiveEntry>>);

impl ArchiveEntries {
    pub fn new_root() -> Self {
        let root = ArchiveEntry::new_directory("/");

        let mut entries = Vec::with_capacity(64);
        entries.push(Rc::new(root));

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

        let mut entries = Self::new_root();

        for i in 0..archive.len() {
            let file = archive
                .by_index(i)
                .with_context(|| anyhow!("failed to get archive file at index {}", i))?;

            // TODO: sanitize?
            let full_name = file.name();
            let path = PathBuf::from(full_name);

            let mut cur_node = NodeID::first();

            for component in path.iter() {
                let component = component.to_string_lossy();

                let existing_pos = entries[cur_node]
                    .children
                    .iter()
                    .find(|&&id| entries[id].name == component)
                    .cloned();

                let next_node_pos = match existing_pos {
                    Some(pos) => pos,
                    None => {
                        let mut entry = ArchiveEntry::from_path(component, full_name, &file);
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

        entries.0.shrink_to_fit();

        Ok(entries)
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
        // This is safe as NodeID's cannot be mutated publicly
        // Our use case for them in ArchiveEntries always points to a valid index.
        unsafe { &self.0.get_unchecked(*index) }
    }
}

#[derive(Debug, Clone)]
pub struct ArchiveEntry {
    pub name: String,
    pub props: EntryProperties,
    pub parent: Option<NodeID>,
    pub children: Vec<NodeID>,
}

impl ArchiveEntry {
    pub fn new<S>(name: S, props: EntryProperties) -> Self
    where
        S: Into<String>,
    {
        Self {
            name: name.into(),
            props,
            parent: None,
            children: Vec::new(),
        }
    }

    pub fn new_directory<S>(name: S) -> Self
    where
        S: Into<String>,
    {
        Self::new(name, EntryProperties::Directory)
    }

    /// Create a new `ArchiveEntry` from a specific file path in an archive.
    ///
    /// The `path` should be the full path of the given `file`, and the
    /// `name` should be a slice from the given `path`. This ensures
    /// that directories and files are detected properly.
    fn from_path<S, P>(name: S, path: P, file: &ZipFile) -> Self
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

        Self::new(name, props)
    }
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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
