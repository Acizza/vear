mod files;
pub mod main;

pub use main::MainPanel;

use anyhow::Result;
use crossterm::event::KeyCode;
use tui::backend::Backend;
use tui::layout::Rect;
use tui::Frame;

pub trait Panel {
    type KeyResult;

    fn tick(&mut self) -> Result<()> {
        Ok(())
    }

    fn process_key(&mut self, key: KeyCode) -> Self::KeyResult;
}

pub trait Draw<B>
where
    B: Backend,
{
    fn draw(&mut self, rect: Rect, frame: &mut Frame<B>);
}
