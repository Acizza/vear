use crate::ui::{
    colors,
    util::{fill_area, text_fragments::TextFragments},
};
use smallvec::SmallVec;
use std::char;
use tui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::Widget,
};

pub struct ProgressBar {
    progress: u8,
}

impl ProgressBar {
    /// Create a new [`ProgressBar`] with the given `progress`.
    ///
    /// Valid values for `progress` are from 0 to 100.
    /// It will be automatically clamped if it goes beyond that.
    pub fn new(progress: u8) -> Self {
        Self {
            progress: progress.min(100),
        }
    }
}

impl Widget for ProgressBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let prog_area = Rect {
            width: ((f32::from(self.progress) / 100.0) * f32::from(area.width)).floor() as u16,
            ..area
        };

        fill_area(prog_area, buf, |cell| {
            cell.bg = Color::Cyan;
            cell.fg = colors::BLACK;
        });

        let style = Style::default();

        // This section renders the current progress without allocating
        let mut fragments: SmallVec<[_; 4]> = SmallVec::new();

        if self.progress > 0 {
            let mut remaining = self.progress;

            while remaining != 0 {
                let digit = b'0' + (remaining % 10);
                remaining /= 10;
                fragments.push((digit as char, style).into());
            }

            fragments.reverse();
        } else {
            fragments.push(('0', style).into());
        }

        fragments.push(('%', style).into());

        let text = TextFragments::new(&fragments).alignment(Alignment::Center);
        text.render(area, buf);
    }
}
