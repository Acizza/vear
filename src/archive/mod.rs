use anyhow::{anyhow, Context, Result};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use zip::{read::ZipFile, ZipArchive};

#[derive(Debug, Clone)]
pub struct ArchiveEntry {
    pub name: String,
    pub props: EntryProperties,
    pub children: Vec<Rc<ArchiveEntry>>,
}

impl ArchiveEntry {
    pub fn new<S>(name: S, props: EntryProperties) -> Self
    where
        S: Into<String>,
    {
        Self {
            name: name.into(),
            props,
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

// TODO: make generic over archive type
pub fn read_files<P>(path: P) -> Result<ArchiveEntry>
where
    P: AsRef<Path>,
{
    let file = File::open(path).context("failed to open archive")?;
    let mut archive = ZipArchive::new(file).context("failed to parse archive")?;

    let mut root = ArchiveEntry::new_directory("/");

    for i in 0..archive.len() {
        let file = archive
            .by_index(i)
            .with_context(|| anyhow!("failed to get archive file at index {}", i))?;

        // TODO: sanitize?
        let full_name = file.name();
        let path = PathBuf::from(full_name);

        let mut cur_node = &mut root;

        for component in path.iter() {
            let component = component.to_string_lossy();
            let existing_pos = cur_node.children.iter().position(|e| e.name == component);

            let next_node_pos = match existing_pos {
                Some(pos) => pos,
                None => {
                    let entry = ArchiveEntry::from_path(component, full_name, &file);
                    cur_node.children.push(Rc::new(entry));
                    cur_node.children.len() - 1
                }
            };

            // Unwrapping here is safe as we aren't creating any other Rc's or weak pointers to any children
            // TODO: this can be optimized to use get_mut_unchecked if/when it's made stable: https://github.com/rust-lang/rust/issues/63292
            cur_node = Rc::get_mut(&mut cur_node.children[next_node_pos]).unwrap();
        }
    }

    Ok(root)
}
