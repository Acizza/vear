mod entry_stats;
mod key_hints;
mod progress_bar;

use self::{entry_stats::EntryStats, key_hints::KeyHints};
use super::files::{PathViewer, PathViewerResult};
use super::{Backend, Draw, Frame, KeyCode, Panel, Rect};
use crate::{
    archive::{extract::Extractor, Archive, NodeID},
    ui::{
        util::{
            input::{Input, InputResult, InputState},
            pad_rect_horiz,
        },
        InputLock,
    },
};
use anyhow::{Context, Result};
use async_std::task;
use parking_lot::Mutex;
use progress_bar::ProgressBar;
use std::sync::{atomic::Ordering, Arc};
use tui::layout::{Constraint, Direction, Layout};

pub struct MainPanel<'a> {
    archive: Arc<Archive>,
    path_viewer: PathViewer,
    entry_stats: EntryStats<'a>,
    state: Arc<Mutex<PanelState>>,
}

impl<'a> MainPanel<'a> {
    const EXTRACT_TO_DIR_KEY: char = 's';
    const EXTRACT_TO_CWD_KEY: char = 'e';
    const MOUNT_AT_DIR_KEY: char = 'l';
    const MOUNT_AT_TMP_KEY: char = 'm';

    pub fn new(archive: Archive) -> Result<Self> {
        let archive = Arc::new(archive);
        let path_viewer =
            PathViewer::new(Arc::clone(&archive), NodeID::first()).context("archive is empty")?;

        let entry_stats = EntryStats::new(
            &archive,
            path_viewer.directory(),
            path_viewer.highlighted().id,
            path_viewer.highlighted_index(),
        );

        Ok(Self {
            archive,
            path_viewer,
            entry_stats,
            state: Arc::new(Mutex::new(PanelState::Navigating)),
        })
    }
}

impl<'a> Panel for MainPanel<'a> {
    type KeyResult = InputLock;

    fn process_key(&mut self, key: KeyCode) -> Self::KeyResult {
        let mut state = self.state.lock();

        match &mut *state {
            PanelState::Navigating | PanelState::Extracting(_) => match (&*state, key) {
                (PanelState::Navigating, KeyCode::Char(Self::EXTRACT_TO_DIR_KEY))
                | (PanelState::Navigating, KeyCode::Char(Self::MOUNT_AT_DIR_KEY)) => {
                    let action = match key {
                        KeyCode::Char(Self::EXTRACT_TO_DIR_KEY) => InputAction::Extract,
                        KeyCode::Char(Self::MOUNT_AT_DIR_KEY) => InputAction::Mount,
                        _ => unreachable!(),
                    };

                    *state = PanelState::Input(InputState::new(), action);
                    InputLock::Locked
                }
                (_, key) => {
                    match self.path_viewer.process_key(key) {
                        PathViewerResult::Ok => (),
                        PathViewerResult::PathSelected(id) => {
                            self.entry_stats.update(
                                &self.archive,
                                self.path_viewer.directory(),
                                id,
                                self.path_viewer.highlighted_index(),
                            );
                        }
                    }
                    InputLock::Unlocked
                }
            },
            PanelState::Input(input, action) => {
                match input.process_key(key) {
                    InputResult::Ok => (),
                    InputResult::Return => *state = PanelState::Navigating,
                    InputResult::ProcessInput(path) => {
                        match action {
                            InputAction::Extract => {
                                let selected = self.path_viewer.selected_ids();
                                let path = path.to_string();

                                let archive = Arc::clone(&self.archive);
                                let panel_state = Arc::clone(&self.state);
                                let extractor = Arc::new(Extractor::prepare(archive, selected));

                                *state = PanelState::Extracting(Arc::clone(&extractor));

                                // TODO: handle unwrap
                                task::spawn(async move {
                                    extractor.extract(path).unwrap();
                                    *panel_state.lock() = PanelState::Navigating;
                                });
                            }
                            InputAction::Mount => unimplemented!(),
                        }
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

        let mut state = self.state.lock();

        match &mut *state {
            PanelState::Navigating => {
                let key_hints = KeyHints {
                    extract_to_dir_key: alpha_upper(Self::EXTRACT_TO_DIR_KEY),
                    extract_to_cwd_key: alpha_upper(Self::EXTRACT_TO_CWD_KEY),
                    mount_at_dir_key: alpha_upper(Self::MOUNT_AT_DIR_KEY),
                    mount_at_tmp_key: alpha_upper(Self::MOUNT_AT_TMP_KEY),
                };

                frame.render_widget(key_hints, pad_rect_horiz(layout[3], 1));
            }
            PanelState::Extracting(extractor) => {
                let extracted = extractor.extracted.load(Ordering::Relaxed) as f32;
                let total_ext = extractor.total_to_extract as f32;
                let pcnt = ((extracted / total_ext) * 100.0).round() as u8;

                let progress = ProgressBar::new(pcnt);
                frame.render_widget(progress, layout[3]);
            }
            PanelState::Input(state, action) => {
                let input = Input::new(action.desc());
                frame.render_stateful_widget(input, layout[3], state);

                if let Some((x, y)) = state.cursor_pos {
                    frame.set_cursor(x, y);
                }
            }
        }
    }
}

enum PanelState {
    Navigating,
    Input(InputState, InputAction),
    Extracting(Arc<Extractor>),
}

#[derive(Copy, Clone)]
enum InputAction {
    Extract,
    Mount,
}

impl InputAction {
    fn desc(self) -> &'static str {
        match self {
            Self::Extract => "extract to",
            Self::Mount => "mount at",
        }
    }
}

// TODO: use char::to_ascii_uppercase if/when it's made a const fn
const fn alpha_upper(ch: char) -> char {
    (ch as u8 - 32) as char
}
