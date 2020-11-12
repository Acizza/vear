mod directory;

use super::{Backend, Draw, Frame, KeyCode, Panel, Rect};
use directory::{DirectoryEntry, DirectoryResult, DirectoryViewer, EntryKind};
use tui::layout::{Constraint, Direction, Layout};

pub struct PathViewer<'a> {
    parent_dir: Option<DirectoryViewer<'a>>,
    cur_dir: DirectoryViewer<'a>,
    child_dir: Option<DirectoryViewer<'a>>,
}

impl<'a> PathViewer<'a> {
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
            parent_dir: None,
            cur_dir: DirectoryViewer::new(items),
            child_dir: None,
        }
    }
}

impl<'a> Panel for PathViewer<'a> {
    type KeyResult = ();

    fn process_key(&mut self, key: KeyCode) -> Self::KeyResult {
        match self.cur_dir.process_key(key) {
            DirectoryResult::Ok => (),
            DirectoryResult::EntrySelected(entry) => {
                // TODO
                self.parent_dir = Some(DirectoryViewer::new(vec![entry.clone()]));
                self.child_dir = Some(DirectoryViewer::new(vec![entry]));
            }
        }
    }
}

impl<'a, B: Backend> Draw<B> for PathViewer<'a> {
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
