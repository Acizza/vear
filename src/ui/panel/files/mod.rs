mod directory;

use super::{Backend, Draw, Frame, KeyCode, Panel, Rect};
use crate::archive::ArchiveEntry;
use directory::{DirectoryEntry, DirectoryResult, DirectoryViewer};
use tui::layout::{Constraint, Direction, Layout};

pub struct PathViewer {
    base_entry: ArchiveEntry,
    parent_dir: Option<DirectoryViewer>,
    cur_dir: DirectoryViewer,
    child_dir: Option<DirectoryViewer>,
}

impl PathViewer {
    pub fn new(base_entry: ArchiveEntry) -> Self {
        let dir_files = Self::mapped_entries(&base_entry);

        Self {
            base_entry,
            parent_dir: None,
            cur_dir: DirectoryViewer::new(dir_files),
            child_dir: None,
        }
    }

    fn mapped_entries(entry: &ArchiveEntry) -> Vec<DirectoryEntry> {
        entry
            .children
            .iter()
            .map(|entry| DirectoryEntry {
                entry: entry.clone(),
                selected: false,
            })
            .collect()
    }
}

impl Panel for PathViewer {
    type KeyResult = ();

    fn process_key(&mut self, key: KeyCode) -> Self::KeyResult {
        match self.cur_dir.process_key(key) {
            DirectoryResult::Ok => (),
            _ => (),
        }
    }
}

impl<B: Backend> Draw<B> for PathViewer {
    fn draw(&mut self, rect: Rect, frame: &mut Frame<B>) {
        let layout = Layout::default()
            .constraints([
                Constraint::Percentage(20),
                Constraint::Length(1),
                Constraint::Percentage(50),
                Constraint::Length(1),
                Constraint::Percentage(30),
            ])
            .direction(Direction::Horizontal)
            .split(rect);

        if let Some(parent_dir) = &mut self.parent_dir {
            parent_dir.draw(layout[0], frame);
        }

        self.cur_dir.draw(layout[2], frame);

        if let Some(child_dir) = &mut self.child_dir {
            child_dir.draw(layout[4], frame);
        }
    }
}
