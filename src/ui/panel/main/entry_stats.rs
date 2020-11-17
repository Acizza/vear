use std::borrow::Cow;

use crate::{archive::ArchiveEntries, util::size};
use crate::{archive::ArchiveEntry, archive::EntryProperties};
use tui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::Widget,
};

pub struct EntryStats<'a> {
    entries: &'a ArchiveEntries,
    dir_entry: &'a ArchiveEntry,
    selected: Option<&'a ArchiveEntry>,
}

impl<'a> EntryStats<'a> {
    pub fn new(
        entries: &'a ArchiveEntries,
        dir_entry: &'a ArchiveEntry,
        selected: Option<&'a ArchiveEntry>,
    ) -> Self {
        Self {
            entries,
            dir_entry,
            selected,
        }
    }

    fn draw_date(
        &self,
        selected: &ArchiveEntry,
        alignment: Alignment,
        area: Rect,
        buf: &mut Buffer,
    ) {
        let date = match &selected.last_modified {
            Some(last_modified) => last_modified,
            None => return,
        };

        let text = SimpleText::new(format!(
            "{}-{:02}-{:02} {:02}:{:02}",
            date.year, date.month, date.day, date.hour, date.minute,
        ))
        .alignment(alignment);

        text.render(area, buf);
    }

    fn draw_compressed_size(
        &self,
        selected: &ArchiveEntry,
        alignment: Alignment,
        area: Rect,
        buf: &mut Buffer,
    ) {
        let (compressed, raw) = match &selected.props {
            EntryProperties::File(props) => (props.compressed_size_bytes, props.raw_size_bytes),
            EntryProperties::Directory => return,
        };

        let pcnt = ((compressed as f64 / raw as f64) * 100.0).round();

        let text = SimpleText::new(format!(
            "{} [{}%]",
            size::formatted_compact(compressed),
            pcnt
        ))
        .alignment(alignment);

        text.render(area, buf);
    }

    fn draw_total_size(&self, alignment: Alignment, area: Rect, buf: &mut Buffer) {
        // TODO: only calculate once
        let (raw_size, compressed_size) = self
            .dir_entry
            .children
            .iter()
            .map(|&id| &self.entries[id])
            .fold((0, 0), |(acc_raw, acc_com), entry| match &entry.props {
                EntryProperties::File(props) => (
                    acc_raw + props.raw_size_bytes,
                    acc_com + props.compressed_size_bytes,
                ),
                EntryProperties::Directory => (acc_raw, acc_com),
            });

        let size_str = if raw_size == 0 {
            Cow::Borrowed("empty")
        } else {
            let ratio = ((compressed_size as f64 / raw_size as f64) * 100.0).round();

            format!(
                "{} / {} [{}%]",
                size::formatted_extra_compact(compressed_size),
                size::formatted_extra_compact(raw_size),
                ratio
            )
            .into()
        };

        let text = SimpleText::new(size_str).alignment(alignment);
        text.render(area, buf);
    }
}

impl<'a> Widget for EntryStats<'a> {
    fn render(self, rect: Rect, buf: &mut Buffer) {
        const MARGIN: u16 = 1;

        if rect.width <= MARGIN || rect.height == 0 {
            return;
        }

        let layout = Layout::default()
            .constraints([
                Constraint::Ratio(1, 3),
                Constraint::Ratio(1, 3),
                Constraint::Ratio(1, 3),
            ])
            .direction(Direction::Horizontal)
            .horizontal_margin(MARGIN)
            .split(rect);

        if let Some(selected) = &self.selected {
            self.draw_date(selected, Alignment::Left, layout[0], buf);
            self.draw_compressed_size(selected, Alignment::Center, layout[1], buf);
        }

        self.draw_total_size(Alignment::Right, layout[2], buf);
    }
}

struct SimpleText<'a> {
    text: Cow<'a, str>,
    alignment: Alignment,
    style: Style,
}

impl<'a> SimpleText<'a> {
    fn new<S>(text: S) -> Self
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
    fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    fn alignment_offset(&self, total_len: u16, item_len: u16) -> u16 {
        match self.alignment {
            Alignment::Left => 0,
            Alignment::Center => (total_len / 2).saturating_sub(item_len / 2),
            Alignment::Right => total_len - item_len,
        }
    }
}

impl<'a> Widget for SimpleText<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let len = self.text.len() as u16;

        if area.width < len {
            return;
        }

        let offset = self.alignment_offset(area.width, len);

        buf.set_string(area.x + offset, area.y, self.text, self.style);
    }
}
