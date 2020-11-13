use super::{Backend, Draw, Frame, KeyCode, Panel};
use crate::archive::{ArchiveEntry, EntryProperties};
use std::ops::Deref;
use std::ops::Range;
use std::rc::Rc;
use tui::buffer::{Buffer, Cell};
use tui::layout::Rect;
use tui::style::{Color, Modifier, Style};
use tui::widgets::Widget;

pub struct DirectoryViewer {
    pub items: WrappedSelection<DirectoryEntry>,
}

impl DirectoryViewer {
    pub fn new(items: Vec<DirectoryEntry>) -> Self {
        Self {
            items: WrappedSelection::new(items),
        }
    }

    /// Calculate how many items are visible based off a given cursor position.
    ///
    /// Returns a range that represents the visible bounds, and a new cursor relative to the visible range.
    fn scroll_window(
        &self,
        cursor: usize,
        num_items: usize,
        height: usize,
    ) -> (Range<usize>, usize) {
        // Scrolling will only happen if the cursor is beyond this threshold
        let base_threshold = height / 2;

        if cursor < base_threshold || num_items <= height {
            let range = Range {
                start: 0,
                end: num_items.min(height),
            };

            return (range, cursor);
        }

        // We can now assume there needs to be at least one item that needs to
        // be scrolled and factor that into our offset
        let offset = 1 + (cursor - base_threshold);
        let end = (offset + height).min(num_items);

        let start = if end == num_items {
            // The remaining items will now fit
            num_items.saturating_sub(height)
        } else {
            offset
        };

        (Range { start, end }, cursor.saturating_sub(start))
    }
}

impl Panel for DirectoryViewer {
    type KeyResult = DirectoryResult;

    fn process_key(&mut self, key: KeyCode) -> Self::KeyResult {
        match key {
            KeyCode::Up | KeyCode::Down => {
                let new_item = match key {
                    KeyCode::Up => self.items.prev(),
                    KeyCode::Down => self.items.next(),
                    _ => unreachable!(),
                };

                new_item
                    .map(DirectoryResult::EntryHighlight)
                    .unwrap_or(DirectoryResult::Ok)
            }
            KeyCode::Char(' ') => {
                let entry = match self.items.selected_mut() {
                    Some(entry) => entry,
                    None => return DirectoryResult::Ok,
                };

                entry.selected = !entry.selected;
                self.items.next();

                DirectoryResult::Ok
            }
            KeyCode::Right | KeyCode::Enter => match self.items.selected() {
                Some(entry) => DirectoryResult::ChildEntry(entry.clone()),
                None => DirectoryResult::Ok,
            },
            KeyCode::Left => match self.items.selected() {
                Some(entry) => DirectoryResult::ParentEntry(entry.clone()),
                None => DirectoryResult::Ok,
            },
            _ => DirectoryResult::Ok,
        }
    }
}

impl<B: Backend> Draw<B> for DirectoryViewer {
    fn draw(&mut self, rect: Rect, frame: &mut Frame<B>) {
        let (window, relative_index) =
            self.scroll_window(self.items.index(), self.items.len(), rect.height as usize);

        let items = &self.items[window.start..window.end];

        for (i, item) in items.iter().enumerate() {
            let highlighted = relative_index == i;
            let rendered_item = RenderedEntry::new(item, highlighted);

            let pos = Rect {
                y: rect.y + (i as u16),
                height: 1,
                ..rect
            };

            frame.render_widget(rendered_item, pos);
        }
    }
}

pub enum DirectoryResult {
    Ok,
    ChildEntry(DirectoryEntry),
    ParentEntry(DirectoryEntry),
    EntryHighlight(DirectoryEntry),
}

pub struct WrappedSelection<T> {
    items: Vec<T>,
    index: usize,
}

impl<T> WrappedSelection<T>
where
    T: Clone,
{
    pub fn new(items: Vec<T>) -> Self {
        Self { items, index: 0 }
    }

    #[inline(always)]
    pub fn next(&mut self) -> Option<T> {
        self.index = (self.index + 1) % self.items.len();
        self.items.get(self.index).cloned()
    }

    #[inline(always)]
    pub fn prev(&mut self) -> Option<T> {
        self.index = if self.index == 0 {
            self.items.len() - 1
        } else {
            self.index - 1
        };

        self.items.get(self.index).cloned()
    }

    #[inline(always)]
    pub fn selected(&self) -> Option<T> {
        self.items.get(self.index).cloned()
    }

    #[inline(always)]
    pub fn selected_mut(&mut self) -> Option<&mut T> {
        self.items.get_mut(self.index)
    }

    #[inline(always)]
    pub fn index(&self) -> usize {
        self.index
    }
}

impl<T> Deref for WrappedSelection<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

#[derive(Clone)]
pub struct DirectoryEntry {
    pub entry: Rc<ArchiveEntry>,
    pub selected: bool,
}

struct RenderedEntry<'a> {
    inner: &'a DirectoryEntry,
    highlighted: bool,
}

impl<'a> RenderedEntry<'a> {
    fn new(entry: &'a DirectoryEntry, highlighted: bool) -> Self {
        Self {
            inner: entry,
            highlighted,
        }
    }

    fn apply_line_color(&self, area: Rect, buf: &mut Buffer) {
        const WHITE: Color = Color::Rgb(225, 225, 225);
        const BLACK: Color = Color::Rgb(10, 10, 10);

        let primary_color = match &self.inner.entry.props {
            EntryProperties::File(_) => WHITE,
            EntryProperties::Directory => Color::LightBlue,
        };

        match (self.highlighted, self.inner.selected) {
            (true, true) => fill_area(area, buf, |cell| {
                cell.fg = BLACK;
                cell.bg = Color::Yellow;
            }),
            (true, false) => fill_area(area, buf, |cell| {
                cell.fg = BLACK;
                cell.bg = primary_color;
            }),
            (false, true) => fill_area(area, buf, |cell| {
                cell.fg = Color::Yellow;
            }),
            (false, false) => fill_area(area, buf, |cell| {
                cell.fg = primary_color;
            }),
        }
    }
}

impl<'a> Widget for RenderedEntry<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        const BASE_NAME_OFFSET: u16 = 1;
        const BASE_SIZE_OFFSET: u16 = 1;
        const MIN_SPACING: u16 = 1;

        self.apply_line_color(area, buf);

        let style = if self.highlighted || self.inner.selected {
            Style::default().add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let name_offset = if self.inner.selected {
            BASE_NAME_OFFSET * 2
        } else {
            BASE_NAME_OFFSET
        };

        buf.set_stringn(
            area.x + name_offset,
            area.y,
            &self.inner.entry.name,
            // This caps the maximum length to always show at least one free character at the end
            area.width.saturating_sub(name_offset + BASE_NAME_OFFSET) as usize,
            style,
        );

        let desc_text = match &self.inner.entry.props {
            EntryProperties::File(props) => formatted_size(props.raw_size_bytes),
            EntryProperties::Directory => self.inner.entry.children.len().to_string(),
        };

        let name_len = name_offset + self.inner.entry.name.len() as u16;
        let size_start = area
            .width
            .saturating_sub(desc_text.len() as u16)
            .saturating_sub(BASE_SIZE_OFFSET);
        let remaining_space = size_start.saturating_sub(MIN_SPACING);

        // Draw the description of the entry only if we have enough room for it
        if remaining_space >= name_len {
            buf.set_string(area.x + size_start, area.y, desc_text, style);
        }
    }
}

fn fill_area<F>(area: Rect, buf: &mut Buffer, func: F)
where
    F: Fn(&mut Cell),
{
    for x in 0..area.width {
        for y in 0..area.height {
            func(buf.get_mut(area.x + x, area.y + y))
        }
    }
}

fn formatted_size(bytes: u64) -> String {
    const BASE_UNIT: u64 = 1024;

    macro_rules! match_units {
        ($($pow:expr => $unit_name:expr => $formatter:expr),+) => {{
            $(
            let threshold = BASE_UNIT.pow($pow);

            if bytes >= threshold {
                let raw_value = bytes as f64 / threshold as f64;

                return if raw_value >= 10.0 {
                    format!("{} {}", raw_value.round(), $unit_name)
                } else {
                    format!(concat!($formatter, " {}"), raw_value, $unit_name)
                };
            }
            )+

            #[cold]
            unreachable!()
        }};
    }

    match_units!(
        // Terabytes
        4 => "T" => "{:.02}",
        // Gigabytes
        3 => "G" => "{:.02}",
        // Megabytes
        2 => "M" => "{:.02}",
        // Kilobytes
        1 => "K" => "{:.02}",
        // Bytes
        0 => "B" => "{}"
    )
}
