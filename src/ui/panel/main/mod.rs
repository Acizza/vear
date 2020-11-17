mod entry_stats;

use std::rc::Rc;

use tui::layout::{Constraint, Direction, Layout};

use self::entry_stats::EntryStats;
use super::files::{PathViewer, PathViewerResult};
use super::{Backend, Draw, Frame, KeyCode, Panel, Rect};
use crate::archive::{ArchiveEntries, NodeID};

pub struct MainPanel<'a> {
    entries: Rc<ArchiveEntries>,
    path_viewer: PathViewer,
    entry_stats: EntryStats<'a>,
}

impl<'a> MainPanel<'a> {
    pub fn new(entries: ArchiveEntries) -> Self {
        let entries = Rc::new(entries);
        let path_viewer = PathViewer::new(Rc::clone(&entries), NodeID::first());

        let entry_stats =
            EntryStats::new(&entries, path_viewer.viewed_dir(), path_viewer.selected());

        Self {
            entries,
            path_viewer,
            entry_stats,
        }
    }
}

impl<'a> Panel for MainPanel<'a> {
    type KeyResult = ();

    fn process_key(&mut self, key: KeyCode) -> Self::KeyResult {
        match self.path_viewer.process_key(key) {
            PathViewerResult::Ok => (),
            PathViewerResult::PathSelected(id) => {
                self.entry_stats.update(
                    &self.entries,
                    self.path_viewer.viewed_dir(),
                    Some(&self.entries[id]),
                );
            }
        }
    }
}

impl<'a, B: Backend> Draw<B> for MainPanel<'a> {
    fn draw(&mut self, rect: Rect, frame: &mut Frame<B>) {
        let layout = Layout::default()
            .constraints([
                Constraint::Min(5),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .direction(Direction::Vertical)
            .split(rect);

        self.path_viewer.draw(layout[0], frame);

        frame.render_widget(self.entry_stats.clone(), layout[2]);
    }
}
