use tui::{
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders},
};

use super::files::PathViewer;
use super::{Backend, Draw, Frame, KeyCode, Panel, Rect};

pub struct MainPanel<'a> {
    path_viewer: PathViewer<'a>,
}

impl<'a> MainPanel<'a> {
    pub fn new() -> Self {
        Self {
            path_viewer: PathViewer::new(),
        }
    }
}

impl<'a> Panel for MainPanel<'a> {
    type KeyResult = ();

    fn process_key(&mut self, key: KeyCode) -> Self::KeyResult {
        self.path_viewer.process_key(key)
    }
}

impl<'a, B: Backend> Draw<B> for MainPanel<'a> {
    fn draw(&mut self, rect: Rect, frame: &mut Frame<B>) {
        let horiz_layout = Layout::default()
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(50),
                Constraint::Percentage(25),
            ])
            .direction(Direction::Horizontal)
            .split(rect);

        let layout = Layout::default()
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(50),
                Constraint::Percentage(25),
            ])
            .direction(Direction::Vertical)
            .split(horiz_layout[1]);

        let block = Block::default().borders(Borders::ALL).title("Test");
        let viewer_pos = block.inner(layout[1]);

        frame.render_widget(block, layout[1]);
        self.path_viewer.draw(viewer_pos, frame);
    }
}
