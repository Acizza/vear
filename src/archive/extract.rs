use super::{Archive, ArchiveEntry, EntryProperties, NodeID};
use anyhow::{anyhow, Context, Result};
use parking_lot::Mutex;
use smallvec::SmallVec;
use std::{
    fs::{self, File},
    io,
    sync::atomic::Ordering,
    sync::Arc,
};
use std::{
    path::{Path, PathBuf},
    sync::atomic::AtomicU32,
};
use zip::ZipArchive;

pub struct Extractor {
    archive: Arc<Archive>,
    base_nodes: SmallVec<[NodeID; 4]>,
    pub extracted: AtomicU32,
    pub total_to_extract: u32,
}

impl Extractor {
    pub fn prepare(archive: Arc<Archive>, base_nodes: SmallVec<[NodeID; 4]>) -> Self {
        let total_to_extract = if base_nodes.contains(&NodeID::first()) {
            archive.files.len() as u32
        } else {
            archive
                .files
                .children_iter(&base_nodes)
                .fold(0, |acc, _| acc + 1)
                .min(archive.files.len() as u32)
        };

        Self {
            archive,
            base_nodes,
            extracted: AtomicU32::new(0),
            total_to_extract,
        }
    }

    pub fn extract<P>(&self, out_path: P) -> Result<()>
    where
        P: AsRef<Path> + Into<PathBuf>,
    {
        fs::create_dir_all(&out_path).context("failed to create base output path")?;

        let out_path = out_path.into();

        let valid_files = self
            .archive
            .files
            .children_iter(&self.base_nodes)
            .filter(|(id, _, _)| *id != NodeID::first());

        for (_, node, path) in valid_files {
            Self::extract_file(&self.archive.inner, node, &out_path.join(path))?;
            self.extracted.fetch_add(1, Ordering::Relaxed);
        }

        Ok(())
    }

    fn extract_file(
        archive: &Mutex<ZipArchive<File>>,
        entry: &ArchiveEntry,
        out_path: &Path,
    ) -> Result<()> {
        match &entry.props {
            EntryProperties::Directory => fs::create_dir(&out_path)
                .with_context(|| anyhow!("failed to create directory: {}", out_path.display()))?,
            EntryProperties::File(_) => {
                let mut file = File::create(&out_path)
                    .with_context(|| anyhow!("failed to create file: {}", out_path.display()))?;

                let mut archive = archive.lock();

                let mut archive_file = archive.by_index(entry.entry_num).with_context(|| {
                    anyhow!("failed to get {} from archive", out_path.display())
                })?;

                io::copy(&mut archive_file, &mut file)
                    .with_context(|| anyhow!("failed to extract file: {}", out_path.display()))?;
            }
        }

        Ok(())
    }
}
