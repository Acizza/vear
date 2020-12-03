#![allow(clippy::cast_lossless)]
#![allow(clippy::cast_possible_wrap)]

use super::{Archive, ArchiveEntry, EntryProperties, NodeID};
use anyhow::Result;
use fuser::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyDirectoryPlus,
    ReplyEmpty, ReplyEntry, ReplyOpen, ReplyStatfs, Request, FUSE_ROOT_ID,
};
use libc::ENOENT;
use std::fs::File;
use std::str::FromStr;
use std::{
    collections::hash_map::Entry,
    collections::HashMap,
    ffi::OsStr,
    io::BufReader,
    io::{BufRead, Read},
    path::Path,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub struct MountedArchive {
    archive: Arc<Archive>,
    uid: u32,
    gid: u32,
    // TODO: use faster hashing algorithm
    cached_files: HashMap<NodeID, Vec<u8>>,
    cur_used_size_bytes: u64,
    avail_memory: AvailableMemory,
}

impl MountedArchive {
    const BLOCK_SIZE: u32 = 4096;
    // 8 GB
    const DEFAULT_TOTAL_MEM: u64 = 8 * 1024 * 1024;
    // Since our filesystem is read only, requests never need to expire
    const REQ_TTL: Duration = Duration::from_secs(u64::MAX);

    pub fn new(archive: Arc<Archive>) -> Self {
        let (uid, gid) = unsafe { (libc::getuid(), libc::getgid()) };

        Self {
            archive,
            uid,
            gid,
            cached_files: HashMap::new(),
            cur_used_size_bytes: 0,
            avail_memory: AvailableMemory::read()
                .unwrap_or_else(|| AvailableMemory::with_avail_kb(Self::DEFAULT_TOTAL_MEM)),
        }
    }

    pub fn mount<P>(self, path: P) -> Result<ArchiveMountSession>
    where
        P: AsRef<Path>,
    {
        let options = ["-o", "ro", "-o", "exec", "-o", "auto_unmount"]
            .iter()
            .map(|s| s.as_ref())
            .collect::<Vec<&OsStr>>();

        let handle = fuser::spawn_mount(self, path, &options)?;
        Ok(ArchiveMountSession(handle))
    }

    fn file_attr(
        &self,
        ino: u64,
        size: u64,
        kind: FileType,
        modified_time: Option<SystemTime>,
    ) -> FileAttr {
        let modified_time = modified_time.unwrap_or(UNIX_EPOCH);

        FileAttr {
            ino,
            size,
            blocks: size / Self::BLOCK_SIZE as u64,
            atime: SystemTime::now(),
            mtime: modified_time,
            ctime: modified_time,
            crtime: modified_time,
            kind,
            // User can execute, everyone can read
            perm: 0o544,
            nlink: 0,
            uid: self.uid,
            gid: self.gid,
            rdev: 0,
            blksize: Self::BLOCK_SIZE as u32,
            padding: 0,
            flags: 0,
        }
    }

    fn get_node(&self, inode: u64) -> Option<(NodeID, &ArchiveEntry)> {
        let id = self
            .archive
            .files
            .create_node_id((inode - FUSE_ROOT_ID) as u32)?;

        let node = &self.archive.files[id];

        Some((id, node))
    }

    fn attr_from_node(&self, node_id: NodeID, node: &ArchiveEntry) -> FileAttr {
        let (size, kind) = match &node.props {
            EntryProperties::File(props) => (props.raw_size_bytes, FileType::RegularFile),
            EntryProperties::Directory => (0, FileType::Directory),
        };

        self.file_attr(
            *node_id as u64 + FUSE_ROOT_ID,
            size,
            kind,
            node.last_modified.as_ref().map(Into::into),
        )
    }
}

// TODO: use fh variable available in many operations to reduce the number of node lookups
impl Filesystem for MountedArchive {
    fn lookup(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let name = name.to_string_lossy();

        let node = if let Some((_, node)) = self.get_node(parent) {
            node
        } else {
            reply.error(ENOENT);
            return;
        };

        let child_id = node
            .children
            .iter()
            .find(|&&id| self.archive.files[id].name == name)
            .cloned();

        let (child_id, child) = if let Some(child) = child_id {
            (child, &self.archive.files[child])
        } else {
            reply.error(ENOENT);
            return;
        };

        let attr = self.attr_from_node(child_id, child);
        reply.entry(&Self::REQ_TTL, &attr, 0);
    }

    fn getattr(&mut self, _req: &Request<'_>, inode: u64, reply: ReplyAttr) {
        let (node_id, node) = if let Some((id, node)) = self.get_node(inode) {
            (id, node)
        } else {
            reply.error(ENOENT);
            return;
        };

        let attr = self.attr_from_node(node_id, node);
        reply.attr(&Self::REQ_TTL, &attr);
    }

    fn open(&mut self, _req: &Request<'_>, _ino: u64, _flags: i32, reply: ReplyOpen) {
        reply.opened(0, 0);
    }

    fn release(
        &mut self,
        _req: &Request<'_>,
        inode: u64,
        _fh: u64,
        _flags: i32,
        _lock_owner: Option<u64>,
        _flush: bool,
        reply: ReplyEmpty,
    ) {
        // Only release a file if we've used over half of the available system memory.
        // We need to keep files cached for as long as possible as reading is very expensive.
        let cur_used_kb = self.cur_used_size_bytes / 1024;
        let remaining_threshold = self.avail_memory.cur_available_kb() / 2;

        if cur_used_kb > remaining_threshold {
            let (node_id, node) = if let Some((id, node)) = self.get_node(inode) {
                (id, node)
            } else {
                reply.error(ENOENT);
                return;
            };

            let size = match &node.props {
                EntryProperties::File(props) => props.raw_size_bytes,
                EntryProperties::Directory => 0,
            };

            self.cached_files.remove(&node_id);
            self.cur_used_size_bytes -= size;
        }

        reply.ok();
    }

    fn read(
        &mut self,
        _req: &Request<'_>,
        inode: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        let (node_id, entry_num, node_size) = if let Some((id, node)) = self.get_node(inode) {
            let node_size = match &node.props {
                EntryProperties::File(props) => props.raw_size_bytes,
                EntryProperties::Directory => 0,
            };

            (id, node.entry_num, node_size)
        } else {
            reply.error(ENOENT);
            return;
        };

        let entry = self.cached_files.entry(node_id);

        let file_data = match entry {
            Entry::Occupied(ref entry) => entry.get(),
            Entry::Vacant(entry) => {
                let mut archive = self.archive.inner.lock();

                let mut file = if let Ok(file) = archive.by_index(entry_num) {
                    file
                } else {
                    reply.error(ENOENT);
                    return;
                };

                let mut bytes = Vec::with_capacity(node_size as usize);
                file.read_to_end(&mut bytes).unwrap();

                self.cur_used_size_bytes += file.size();

                entry.insert(bytes)
            }
        };

        let offset = offset as usize;
        let end = (offset + size as usize).min(file_data.len());

        reply.data(&file_data[offset..end]);
    }

    fn readdir(
        &mut self,
        _req: &Request<'_>,
        inode: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        let node = if let Some((_, node)) = self.get_node(inode) {
            node
        } else {
            reply.error(ENOENT);
            return;
        };

        for (i, &child_id) in node.children.iter().enumerate().skip(offset as usize) {
            let child = &self.archive.files[child_id];

            let kind = match &child.props {
                EntryProperties::File(_) => FileType::RegularFile,
                EntryProperties::Directory => FileType::Directory,
            };

            if reply.add(
                *child_id as u64 + FUSE_ROOT_ID,
                FUSE_ROOT_ID as i64 + i as i64,
                kind,
                &child.name,
            ) {
                break;
            }
        }

        reply.ok();
    }

    fn readdirplus(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _offset: i64,
        reply: ReplyDirectoryPlus,
    ) {
        reply.ok();
    }

    fn flush(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _lock_owner: u64,
        reply: ReplyEmpty,
    ) {
        reply.ok();
    }

    fn fsync(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _datasync: bool,
        reply: ReplyEmpty,
    ) {
        reply.ok();
    }

    fn opendir(&mut self, _req: &Request<'_>, _ino: u64, _flags: i32, reply: ReplyOpen) {
        reply.opened(0, 0);
    }

    fn releasedir(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _flags: i32,
        reply: ReplyEmpty,
    ) {
        reply.ok();
    }

    fn fsyncdir(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _datasync: bool,
        reply: ReplyEmpty,
    ) {
        reply.ok();
    }

    fn statfs(&mut self, _req: &Request<'_>, _ino: u64, reply: ReplyStatfs) {
        let blocks = self.archive.total_size_bytes / Self::BLOCK_SIZE as u64;

        reply.statfs(
            blocks,
            0,
            0,
            self.archive.files.len() as u64,
            0,
            Self::BLOCK_SIZE,
            255,
            0,
        );
    }

    fn access(&mut self, _req: &Request<'_>, _ino: u64, _mask: i32, reply: ReplyEmpty) {
        reply.ok();
    }
}

fn read_meminfo_field(field: &str) -> Option<u64> {
    let file = File::open("/proc/meminfo").ok()?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line.ok()?;

        let cur_field = match line.split(':').next() {
            Some(cur) => cur,
            None => continue,
        };

        if cur_field != field {
            continue;
        }

        let value = line
            .rsplit(' ')
            .nth(1)
            .and_then(|num| u64::from_str(num).ok())?;

        return Some(value);
    }

    None
}

struct AvailableMemory {
    current_kb: u64,
    last_check: SystemTime,
}

impl AvailableMemory {
    const REFRESH_SECS: u64 = 5;

    fn read() -> Option<Self> {
        let current_kb = read_meminfo_field("MemAvailable")?;

        Some(Self {
            current_kb,
            last_check: SystemTime::now(),
        })
    }

    fn with_avail_kb(available_kb: u64) -> Self {
        Self {
            current_kb: available_kb,
            last_check: SystemTime::now(),
        }
    }

    fn cur_available_kb(&mut self) -> u64 {
        if let Ok(elapsed) = self.last_check.elapsed() {
            if elapsed.as_secs() < Self::REFRESH_SECS {
                return self.current_kb;
            }
        }

        if let Some(avail) = read_meminfo_field("MemAvailable") {
            self.current_kb = avail;
            self.last_check = SystemTime::now();
        }

        self.current_kb
    }
}

/// A wrapper type around `fuser::BackgroundSession` that's safe to send across threads.
///
/// This uses unsafe impl's for `Send` and `Sync`. These should in fact be safe as the wrapper type prevents any kind of
/// modification to the underlying `fuser::BackgroundSession`.
pub struct ArchiveMountSession(fuser::BackgroundSession);

unsafe impl Send for ArchiveMountSession {}
unsafe impl Sync for ArchiveMountSession {}
