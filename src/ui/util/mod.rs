pub mod text_fragments;

use std::borrow::Cow;
use tui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::Style,
    widgets::Widget,
};

/// This is a mimic of the `tui::text::Span` type that can be rendered without allocating.
pub struct SimpleText<'a> {
    text: Cow<'a, str>,
    alignment: Alignment,
    style: Style,
}

impl<'a> SimpleText<'a> {
    pub fn new<S>(text: S) -> Self
    where
        S: Into<Cow<'a, str>>,
    {
        Self {
            text: text.into(),
            alignment: Alignment::Left,
            style: Style::default(),
        }
    }

    #[inline(always)]
    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }
}

impl<'a> Widget for SimpleText<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let len = self.text.len() as u16;

        if area.width < len {
            return;
        }

        let offset = alignment_offset(self.alignment, area.width, len);

        buf.set_string(area.x + offset, area.y, self.text, self.style);
    }
}

fn alignment_offset(alignment: Alignment, total_len: u16, item_len: u16) -> u16 {
    match alignment {
        Alignment::Left => 0,
        Alignment::Center => (total_len / 2).saturating_sub(item_len / 2),
        Alignment::Right => total_len.saturating_sub(item_len),
    }
}

pub fn pad_rect_horiz(rect: Rect, padding: u16) -> Rect {
    Rect {
        x: rect.x + padding,
        width: rect.width.saturating_sub(padding * 2),
        ..rect
    }
}
