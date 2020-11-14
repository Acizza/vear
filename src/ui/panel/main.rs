use super::files::PathViewer;
use super::{Backend, Draw, Frame, KeyCode, Panel, Rect};
use crate::archive::ArchiveEntries;

pub struct MainPanel {
    path_viewer: PathViewer,
}

impl MainPanel {
    pub fn new(archive_entries: ArchiveEntries) -> Self {
        Self {
            path_viewer: PathViewer::new(archive_entries),
        }
    }
}

impl Panel for MainPanel {
    type KeyResult = ();

    fn process_key(&mut self, key: KeyCode) -> Self::KeyResult {
        self.path_viewer.process_key(key)
    }
}

impl<B: Backend> Draw<B> for MainPanel {
    fn draw(&mut self, rect: Rect, frame: &mut Frame<B>) {
        self.path_viewer.draw(rect, frame);
    }
}
