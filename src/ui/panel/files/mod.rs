mod directory;

use super::{Backend, Draw, Frame, KeyCode, Panel, Rect};
use directory::{DirectoryEntry, DirectoryViewer, EntryKind};

pub struct FileViewer<'a> {
    cur_files: DirectoryViewer<'a>,
}

impl<'a> FileViewer<'a> {
    pub fn new() -> Self {
        let items = (0..20)
            .map(|i| DirectoryEntry {
                name: format!("test {}.mp4", i).into(),
                size_bytes: 512 * (i as u64).pow(8),
                kind: if i % 3 == 0 {
                    EntryKind::Directory
                } else {
                    EntryKind::File
                },
                selected: false,
            })
            .collect::<Vec<_>>();

        Self {
            cur_files: DirectoryViewer::new(items),
        }
    }
}

impl<'a> Panel for FileViewer<'a> {
    type KeyResult = ();

    fn process_key(&mut self, key: KeyCode) -> Self::KeyResult {
        self.cur_files.process_key(key)
    }
}

impl<'a, B: Backend> Draw<B> for FileViewer<'a> {
    fn draw(&mut self, rect: Rect, frame: &mut Frame<B>) {
        self.cur_files.draw(rect, frame);
    }
}
