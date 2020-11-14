mod directory;

use super::{Backend, Draw, Frame, KeyCode, Panel, Rect};
use crate::archive::{ArchiveEntries, NodeID};
use directory::{DirectoryResult, DirectoryViewer};
use std::mem;
use tui::layout::{Constraint, Direction, Layout};

pub struct PathViewer {
    entries: ArchiveEntries,
    parent_dir: Option<DirectoryViewer>,
    cur_dir: DirectoryViewer,
    child_dir: Option<DirectoryViewer>,
}

impl PathViewer {
    pub fn new(entries: ArchiveEntries) -> Self {
        let cur_dir = DirectoryViewer::new(&entries, NodeID::first());

        let child_dir = cur_dir
            .entries
            .selected()
            .map(|selected| DirectoryViewer::new(&entries, selected.id));

        Self {
            entries,
            parent_dir: None,
            cur_dir,
            child_dir,
        }
    }

    fn new_dir_viewer(&self, node: NodeID) -> DirectoryViewer {
        DirectoryViewer::new(&self.entries, node)
    }
}

impl Panel for PathViewer {
    type KeyResult = ();

    fn process_key(&mut self, key: KeyCode) -> Self::KeyResult {
        match self.cur_dir.process_key(key) {
            DirectoryResult::Ok => (),
            DirectoryResult::EntryHighlight(node) => {
                self.child_dir = if self.entries[node].props.is_dir() {
                    Some(self.new_dir_viewer(node))
                } else {
                    None
                };
            }
            DirectoryResult::ChildEntry(node) => {
                if !self.entries[node].props.is_dir() {
                    return;
                }

                let old_cur = {
                    let replacement = self.new_dir_viewer(node);
                    mem::replace(&mut self.cur_dir, replacement)
                };

                self.parent_dir = Some(old_cur);

                self.child_dir = self
                    .cur_dir
                    .entries
                    .selected()
                    .map(|selected| self.new_dir_viewer(selected.id));
            }
            DirectoryResult::ParentEntry(node) => {
                let new_cur = match mem::take(&mut self.parent_dir) {
                    Some(new_cur) => new_cur,
                    None => return,
                };

                self.child_dir = Some(mem::replace(&mut self.cur_dir, new_cur));

                let parent = self.entries[node]
                    .parent
                    .and_then(|parent| self.entries[parent].parent)
                    .and_then(|parent| self.entries[parent].parent);

                if let Some(parent) = parent {
                    self.parent_dir = Some(self.new_dir_viewer(parent));
                }
            }
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
