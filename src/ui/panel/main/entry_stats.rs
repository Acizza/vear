use std::borrow::Cow;

use crate::{
    archive::ArchiveEntry,
    archive::{Archive, EntryProperties},
};
use crate::{archive::NodeID, util::size};
use tui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::Widget,
};

#[derive(Clone)]
pub struct EntryStats<'a> {
    date: Option<String>,
    encoding: Option<&'static str>,
    compressed_size: Option<String>,
    total_size: Cow<'a, str>,
    selection: String,
}

impl<'a> EntryStats<'a> {
    pub fn new(
        archive: &Archive,
        viewed_dir: NodeID,
        selected: Option<&ArchiveEntry>,
        selected_idx: usize,
    ) -> Self {
        let dir_entry = &archive[viewed_dir];

        Self {
            date: selected.and_then(Self::date_text),
            encoding: selected.map(Self::encoding_text),
            compressed_size: selected.and_then(Self::compressed_size_text),
            total_size: Self::total_size_text(archive, dir_entry),
            selection: Self::selection_text(dir_entry, selected_idx),
        }
    }

    pub fn update(
        &mut self,
        archive: &Archive,
        viewed_dir: NodeID,
        selected: Option<&ArchiveEntry>,
        selected_idx: usize,
    ) {
        *self = Self::new(archive, viewed_dir, selected, selected_idx);
    }

    fn date_text(entry: &ArchiveEntry) -> Option<String> {
        let date = match &entry.last_modified {
            Some(last_modified) => last_modified,
            None => return None,
        };

        format!(
            "{}-{:02}-{:02} {:02}:{:02}",
            date.year, date.month, date.day, date.hour, date.minute,
        )
        .into()
    }

    fn encoding_text(entry: &ArchiveEntry) -> &'static str {
        entry.encoding.name()
    }

    fn compressed_size_text(entry: &ArchiveEntry) -> Option<String> {
        let (compressed, raw) = match &entry.props {
            EntryProperties::File(props) => (props.compressed_size_bytes, props.raw_size_bytes),
            EntryProperties::Directory => return None,
        };

        if raw == 0 {
            return None;
        }

        let pcnt = ((compressed as f64 / raw as f64) * 100.0).round();

        format!("{} [{}%]", size::formatted_compact(compressed), pcnt).into()
    }

    fn total_size_text(archive: &Archive, dir: &ArchiveEntry) -> Cow<'a, str> {
        let (raw_size, compressed_size) = dir.children.iter().map(|&id| &archive[id]).fold(
            (0, 0),
            |(acc_raw, acc_com), entry| match &entry.props {
                EntryProperties::File(props) => (
                    acc_raw + props.raw_size_bytes,
                    acc_com + props.compressed_size_bytes,
                ),
                EntryProperties::Directory => (acc_raw, acc_com),
            },
        );

        if raw_size == 0 {
            Cow::Borrowed("empty")
        } else {
            let ratio = ((compressed_size as f64 / raw_size as f64) * 100.0).round();

            format!(
                "{}:{} [{}%]",
                size::formatted_extra_compact(compressed_size),
                size::formatted_extra_compact(raw_size),
                ratio
            )
            .into()
        }
    }

    fn selection_text(dir_entry: &ArchiveEntry, selected: usize) -> String {
        format!("{}/{}", 1 + selected, dir_entry.children.len())
    }
}

impl<'a> Widget for EntryStats<'a> {
    fn render(self, rect: Rect, buf: &mut Buffer) {
        const MARGIN: u16 = 1;
        const PADDING: Constraint = Constraint::Length(2);

        if rect.width <= MARGIN || rect.height == 0 {
            return;
        }

        let layout = Layout::default()
            .constraints([
                Constraint::Ratio(2, 5),
                PADDING,
                Constraint::Ratio(1, 5),
                PADDING,
                Constraint::Ratio(2, 5),
            ])
            .direction(Direction::Horizontal)
            .horizontal_margin(MARGIN)
            .split(rect);

        let left_layout = Layout::default()
            .constraints([
                Constraint::Length(self.date.as_ref().map(|date| date.len()).unwrap_or(0) as u16),
                Constraint::Length(2),
                Constraint::Length(self.encoding.as_ref().map(|e| e.len()).unwrap_or(0) as u16),
            ])
            .direction(Direction::Horizontal)
            .split(layout[0]);

        if let Some(date) = &self.date {
            let text = SimpleText::new(date).alignment(Alignment::Left);
            text.render(left_layout[0], buf);
        }

        if let Some(encoding) = self.encoding {
            let text = SimpleText::new(encoding).alignment(Alignment::Left);
            text.render(left_layout[2], buf);
        }

        if let Some(compressed_size) = &self.compressed_size {
            let text = SimpleText::new(compressed_size).alignment(Alignment::Center);
            text.render(layout[2], buf);
        }

        let right_layout = Layout::default()
            .constraints([
                Constraint::Min(self.total_size.len() as u16),
                PADDING,
                Constraint::Length(self.selection.len() as u16),
            ])
            .direction(Direction::Horizontal)
            .split(layout[4]);

        let total_size = SimpleText::new(self.total_size).alignment(Alignment::Right);
        total_size.render(right_layout[0], buf);

        let selection = SimpleText::new(&self.selection).alignment(Alignment::Right);
        selection.render(right_layout[2], buf);
    }
}

/// This is a mimic of the tui crate's Span type that can be rendered without allocating.
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
