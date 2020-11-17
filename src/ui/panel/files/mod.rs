mod directory;

use super::{Backend, Draw, Frame, KeyCode, Panel, Rect};
use crate::archive::{ArchiveEntries, ArchiveEntry, NodeID};
use directory::{DirectoryResult, DirectoryViewer};
use std::{mem, rc::Rc};
use tui::layout::{Constraint, Direction, Layout};

pub struct PathViewer {
    entries: Rc<ArchiveEntries>,
    parent_dir: Option<DirectoryViewer>,
    cur_dir: DirectoryViewer,
    child_dir: Option<DirectoryViewer>,
}

impl PathViewer {
    pub fn new(entries: Rc<ArchiveEntries>, viewed: NodeID) -> Self {
        let cur_dir = DirectoryViewer::new(&entries, viewed);

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

    #[inline(always)]
    pub fn viewed_dir(&self) -> NodeID {
        self.cur_dir.viewed
    }

    pub fn selected(&self) -> Option<&ArchiveEntry> {
        self.cur_dir
            .entries
            .selected()
            .map(|selected| selected.entry.as_ref())
    }

    pub fn selected_id(&self) -> Option<NodeID> {
        self.cur_dir.entries.selected().map(|selected| selected.id)
    }

    fn new_dir_viewer(&self, node: NodeID) -> DirectoryViewer {
        DirectoryViewer::new(&self.entries, node)
    }
}

impl Panel for PathViewer {
    type KeyResult = PathViewerResult;

    fn process_key(&mut self, key: KeyCode) -> Self::KeyResult {
        match self.cur_dir.process_key(key) {
            DirectoryResult::Ok => PathViewerResult::Ok,
            DirectoryResult::EntryHighlight(id) => {
                self.child_dir = if self.entries[id].props.is_dir() {
                    Some(self.new_dir_viewer(id))
                } else {
                    None
                };

                PathViewerResult::PathSelected(id)
            }
            DirectoryResult::ViewChild(id) => {
                let node = &self.entries[id];

                if !node.props.is_dir() || node.children.is_empty() {
                    return PathViewerResult::Ok;
                }

                let old_cur = {
                    let replacement = self.new_dir_viewer(id);
                    mem::replace(&mut self.cur_dir, replacement)
                };

                self.parent_dir = Some(old_cur);

                self.child_dir = self
                    .cur_dir
                    .entries
                    .selected()
                    .map(|selected| self.new_dir_viewer(selected.id));

                self.selected_id()
                    .map(PathViewerResult::PathSelected)
                    .unwrap_or(PathViewerResult::Ok)
            }
            DirectoryResult::ViewParent(id) => {
                let new_cur = match mem::take(&mut self.parent_dir) {
                    Some(new_cur) => new_cur,
                    None => return PathViewerResult::Ok,
                };

                self.child_dir = Some(mem::replace(&mut self.cur_dir, new_cur));

                let parent = self.entries[id]
                    .parent
                    .and_then(|parent| self.entries[parent].parent)
                    .and_then(|parent| self.entries[parent].parent);

                if let Some(parent) = parent {
                    self.parent_dir = Some(self.new_dir_viewer(parent));
                }

                self.selected_id()
                    .map(PathViewerResult::PathSelected)
                    .unwrap_or(PathViewerResult::Ok)
            }
        }
    }
}

impl<B: Backend> Draw<B> for PathViewer {
    fn draw(&mut self, rect: Rect, frame: &mut Frame<B>) {
        let layout = Layout::default()
            .constraints([
                Constraint::Percentage(25),
                Constraint::Length(1),
                Constraint::Percentage(50),
                Constraint::Length(1),
                Constraint::Percentage(25),
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

pub enum PathViewerResult {
    Ok,
    PathSelected(NodeID),
}
