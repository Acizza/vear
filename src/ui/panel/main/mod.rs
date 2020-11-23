mod entry_stats;
mod key_hints;

use self::{entry_stats::EntryStats, key_hints::KeyHints};
use super::files::{PathViewer, PathViewerResult};
use super::{Backend, Draw, Frame, KeyCode, Panel, Rect};
use crate::{
    archive::{Archive, NodeID},
    ui::{
        util::{
            input::{Input, InputResult, InputState},
            pad_rect_horiz,
        },
        InputLock,
    },
};
use anyhow::{Context, Result};
use std::rc::Rc;
use tui::layout::{Constraint, Direction, Layout};

pub struct MainPanel<'a> {
    archive: Rc<Archive>,
    path_viewer: PathViewer,
    entry_stats: EntryStats<'a>,
    state: PanelState,
}

impl<'a> MainPanel<'a> {
    const EXTRACT_TO_DIR_KEY: char = 's';
    const EXTRACT_TO_CWD_KEY: char = 'e';
    const MOUNT_AT_DIR_KEY: char = 'l';
    const MOUNT_AT_TMP_KEY: char = 'm';

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
            state: PanelState::Navigating,
        })
    }

    fn process_path_viewer_key(&mut self, key: KeyCode) {
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

impl<'a> Panel for MainPanel<'a> {
    type KeyResult = InputLock;

    fn process_key(&mut self, key: KeyCode) -> Self::KeyResult {
        match &mut self.state {
            PanelState::Navigating => match key {
                KeyCode::Char(Self::EXTRACT_TO_DIR_KEY) | KeyCode::Char(Self::MOUNT_AT_DIR_KEY) => {
                    let desc = match key {
                        KeyCode::Char(Self::EXTRACT_TO_DIR_KEY) => "extract to",
                        KeyCode::Char(Self::MOUNT_AT_DIR_KEY) => "mount at",
                        _ => unreachable!(),
                    };

                    self.state = PanelState::Input(InputState::new(), desc);
                    InputLock::Locked
                }
                key => {
                    self.process_path_viewer_key(key);
                    InputLock::Unlocked
                }
            },
            PanelState::Input(input, _) => {
                match input.process_key(key) {
                    InputResult::Ok => (),
                    InputResult::Return => self.state = PanelState::Navigating,
                    InputResult::ProcessInput(_) => {
                        input.reset();
                    }
                }

                InputLock::Locked
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

        match &mut self.state {
            PanelState::Navigating => {
                let key_hints = KeyHints {
                    extract_to_dir_key: alpha_upper(Self::EXTRACT_TO_DIR_KEY),
                    extract_to_cwd_key: alpha_upper(Self::EXTRACT_TO_CWD_KEY),
                    mount_at_dir_key: alpha_upper(Self::MOUNT_AT_DIR_KEY),
                    mount_at_tmp_key: alpha_upper(Self::MOUNT_AT_TMP_KEY),
                };

                frame.render_widget(key_hints, pad_rect_horiz(layout[3], 1));
            }
            PanelState::Input(state, desc) => {
                let input = Input::new(desc);
                frame.render_stateful_widget(input, layout[3], state);

                if let Some((x, y)) = state.cursor_pos {
                    frame.set_cursor(x, y);
                }
            }
        }
    }
}

type Description = &'static str;

enum PanelState {
    Navigating,
    Input(InputState, Description),
}

// TODO: use char::to_ascii_uppercase if/when it's made a const fn
const fn alpha_upper(ch: char) -> char {
    (ch as u8 - 32) as char
}
