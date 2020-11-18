mod entry_stats;

use std::rc::Rc;

use tui::layout::{Constraint, Direction, Layout};

use self::entry_stats::EntryStats;
use super::files::{PathViewer, PathViewerResult};
use super::{Backend, Draw, Frame, KeyCode, Panel, Rect};
use crate::archive::{Archive, NodeID};

pub struct MainPanel<'a> {
    archive: Rc<Archive>,
    path_viewer: PathViewer,
    entry_stats: EntryStats<'a>,
}

impl<'a> MainPanel<'a> {
    pub fn new(archive: Archive) -> Self {
        let archive = Rc::new(archive);
        let path_viewer = PathViewer::new(Rc::clone(&archive), NodeID::first());

        let entry_stats = EntryStats::new(
            &archive,
            path_viewer.viewed_dir(),
            path_viewer.selected(),
            path_viewer.selected_idx(),
        );

        Self {
            archive,
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
                    &self.archive,
                    self.path_viewer.viewed_dir(),
                    Some(&self.archive[id]),
                    self.path_viewer.selected_idx(),
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
