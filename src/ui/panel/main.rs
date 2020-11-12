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
        self.path_viewer.draw(rect, frame);
    }
}
