mod entry_stats;

use std::rc::Rc;

use tui::layout::{Constraint, Direction, Layout};

use self::entry_stats::EntryStats;
use super::files::PathViewer;
use super::{Backend, Draw, Frame, KeyCode, Panel, Rect};
use crate::archive::{ArchiveEntries, NodeID};

pub struct MainPanel {
    entries: Rc<ArchiveEntries>,
    path_viewer: PathViewer,
}

impl MainPanel {
    pub fn new(entries: ArchiveEntries) -> Self {
        let entries = Rc::new(entries);
        let path_viewer = PathViewer::new(Rc::clone(&entries), NodeID::first());

        Self {
            entries,
            path_viewer,
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
        let layout = Layout::default()
            .constraints([
                Constraint::Min(5),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .direction(Direction::Vertical)
            .split(rect);

        self.path_viewer.draw(layout[0], frame);

        let stats = EntryStats::new(
            &self.entries,
            &self.entries[self.path_viewer.viewed_dir()],
            self.path_viewer.selected(),
        );

        frame.render_widget(stats, layout[2]);
    }
}
