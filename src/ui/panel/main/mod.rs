mod entry_stats;
mod key_hints;

use self::{entry_stats::EntryStats, key_hints::KeyHints};
use super::files::{PathViewer, PathViewerResult};
use super::{Backend, Draw, Frame, KeyCode, Panel, Rect};
use crate::{
    archive::{Archive, NodeID},
    ui::util::pad_rect_horiz,
};
use anyhow::{Context, Result};
use std::rc::Rc;
use tui::layout::{Constraint, Direction, Layout};

pub struct MainPanel<'a> {
    archive: Rc<Archive>,
    path_viewer: PathViewer,
    entry_stats: EntryStats<'a>,
}

impl<'a> MainPanel<'a> {
    pub fn new(archive: Archive) -> Result<Self> {
        let archive = Rc::new(archive);
        let path_viewer =
            PathViewer::new(Rc::clone(&archive), NodeID::first()).context("archive is empty")?;

        let entry_stats = EntryStats::new(
            &archive,
            path_viewer.directory(),
            path_viewer.selected(),
            path_viewer.selected_index(),
        );

        Ok(Self {
            archive,
            path_viewer,
            entry_stats,
        })
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
                    self.path_viewer.directory(),
                    &self.archive[id],
                    self.path_viewer.selected_index(),
                );
            }
        }
    }
}

impl<'a, B: Backend> Draw<B> for MainPanel<'a> {
    fn draw(&mut self, rect: Rect, frame: &mut Frame<B>) {
        let layout = Layout::default()
            .constraints([
                // Path viewer
                Constraint::Min(5),
                // Padding
                Constraint::Length(1),
                // Entry stats
                Constraint::Length(1),
                // Key hints
                Constraint::Length(1),
            ])
            .direction(Direction::Vertical)
            .split(rect);

        self.path_viewer.draw(layout[0], frame);

        frame.render_widget(self.entry_stats.clone(), layout[2]);

        let key_hints = KeyHints {
            extract_to_dir_key: 'S',
            extract_to_cwd_key: 'E',
            mount_at_dir_key: 'L',
            mount_at_tmp_key: 'M',
        };

        frame.render_widget(key_hints, pad_rect_horiz(layout[3], 1));
    }
}
