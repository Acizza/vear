mod directory;

use self::directory::DirectoryEntry;

use super::{Backend, Draw, Frame, KeyCode, Panel, Rect};
use crate::archive::{Archive, NodeID};
use directory::{DirectoryResult, DirectoryViewer};
use smallvec::SmallVec;
use std::mem;
use tui::layout::{Constraint, Direction, Layout};

/// Widget to navigate and browse a given directory with its parent and child to ease navigation.
pub struct PathViewer {
    parent_dir: Option<DirectoryViewer>,
    cur_dir: DirectoryViewer,
    child_dir: Option<DirectoryViewer>,
}

impl PathViewer {
    /// Create a new `PathViewer` to view the given `directory` in the given `archive`.
    ///
    /// Returns None if the given `directory` has no entries (children) to show.
    pub fn new(archive: &Archive, directory: NodeID) -> Option<Self> {
        let cur_dir = DirectoryViewer::new(&archive, directory)?;
        let child_dir = DirectoryViewer::new(&archive, cur_dir.highlighted().id);

        Some(Self {
            parent_dir: None,
            cur_dir,
            child_dir,
        })
    }

    pub fn process_key(&mut self, key: KeyCode, archive: &Archive) -> PathViewerResult {
        match self.cur_dir.process_key(key) {
            DirectoryResult::Ok => PathViewerResult::Ok,
            DirectoryResult::EntryHighlight(id) => {
                self.child_dir = if archive[id].props.is_dir() {
                    DirectoryViewer::new(archive, id)
                } else {
                    None
                };

                PathViewerResult::PathSelected(id)
            }
            DirectoryResult::ViewChild(id) => {
                let new_cur = match DirectoryViewer::new(archive, id) {
                    Some(new_cur) => new_cur,
                    None => return PathViewerResult::Ok,
                };

                let old_cur = mem::replace(&mut self.cur_dir, new_cur);
                let highlighted_node = self.highlighted().id;

                self.parent_dir = Some(old_cur);
                self.child_dir = DirectoryViewer::new(archive, highlighted_node);

                PathViewerResult::PathSelected(highlighted_node)
            }
            DirectoryResult::ViewParent(id) => {
                let new_cur = match mem::take(&mut self.parent_dir) {
                    Some(new_cur) => new_cur,
                    None => return PathViewerResult::Ok,
                };

                self.child_dir = Some(mem::replace(&mut self.cur_dir, new_cur));

                let parent = archive[id]
                    .parent
                    .and_then(|parent| archive[parent].parent)
                    .and_then(|parent| archive[parent].parent);

                if let Some(parent) = parent {
                    self.parent_dir = DirectoryViewer::new(archive, parent);
                }

                PathViewerResult::PathSelected(self.highlighted().id)
            }
        }
    }

    #[inline(always)]
    pub fn directory(&self) -> NodeID {
        self.cur_dir.directory()
    }

    /// Returns a reference to the currently highlighted [`DirectoryEntry`].
    #[inline(always)]
    pub fn highlighted(&self) -> &DirectoryEntry {
        self.cur_dir.highlighted()
    }

    pub fn selected_ids(&self) -> SmallVec<[NodeID; 4]> {
        self.cur_dir.selected_ids()
    }

    /// Returns the index of the selected entry in the currently viewed directory.
    #[inline(always)]
    pub fn highlighted_index(&self) -> usize {
        self.cur_dir.highlighted_index()
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
