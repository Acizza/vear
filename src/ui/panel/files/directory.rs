use super::{Backend, Draw, Frame, KeyCode, Panel};
use crate::util::size;
use crate::{
    archive::{Archive, ArchiveEntry, EntryProperties, NodeID},
    ui::util::fill_area,
};
use std::ops::Range;
use std::{ops::Deref, rc::Rc};
use tui::buffer::Buffer;
use tui::layout::Rect;
use tui::style::{Color, Modifier, Style};
use tui::widgets::Widget;
use unicode_width::UnicodeWidthStr;

/// Widget to browse a given directory.
pub struct DirectoryViewer {
    entries: WrappedSelection<DirectoryEntry>,
    directory: NodeID,
    highlighted: NodeID,
}

impl DirectoryViewer {
    /// Create a new `DirectoryViewer` to view the given `directory` in the given `archive`.
    ///
    /// Returns None if the given `directory` has no entries (children) to show.
    pub fn new(archive: &Archive, directory: NodeID) -> Option<Self> {
        let dir_entry = &archive[directory];

        if dir_entry.children.is_empty() {
            return None;
        }

        let mut children = dir_entry
            .children
            .iter()
            .map(|&id| {
                let entry = Rc::clone(&archive[id]);

                let size = match &entry.props {
                    EntryProperties::File(props) => size::formatted(props.raw_size_bytes),
                    EntryProperties::Directory => entry.children.len().to_string(),
                };

                DirectoryEntry {
                    id,
                    entry,
                    selected: false,
                    size,
                }
            })
            .collect::<Vec<_>>();

        children.sort_unstable_by(|x, y| {
            let by_kind_desc = y.entry.props.is_dir().cmp(&x.entry.props.is_dir());
            let by_name_desc = x.entry.name.cmp(&y.entry.name);
            by_kind_desc.then(by_name_desc)
        });

        // We're guaranteed to have at least one child, so this is safe
        let highlighted = children[0].id;

        Some(Self {
            entries: WrappedSelection::new(children),
            directory,
            highlighted,
        })
    }

    #[inline(always)]
    pub fn selected(&self) -> &DirectoryEntry {
        self.entries.selected()
    }

    #[inline(always)]
    pub fn selected_index(&self) -> usize {
        self.entries.index()
    }

    #[inline(always)]
    pub fn directory(&self) -> NodeID {
        self.directory
    }
}

impl Panel for DirectoryViewer {
    type KeyResult = DirectoryResult;

    fn process_key(&mut self, key: KeyCode) -> Self::KeyResult {
        match key {
            KeyCode::Up | KeyCode::Down => {
                let &DirectoryEntry { id, .. } = match key {
                    KeyCode::Up => self.entries.prev(),
                    KeyCode::Down => self.entries.next(),
                    _ => unreachable!(),
                };

                self.highlighted = id;
                DirectoryResult::EntryHighlight(id)
            }
            KeyCode::Char(' ') => {
                let entry = self.entries.selected_mut();
                entry.selected = !entry.selected;

                let next = self.entries.next();
                self.highlighted = next.id;

                DirectoryResult::Ok
            }
            KeyCode::Right | KeyCode::Enter => {
                DirectoryResult::ViewChild(self.entries.selected().id)
            }
            KeyCode::Left => DirectoryResult::ViewParent(self.entries.selected().id),
            _ => DirectoryResult::Ok,
        }
    }
}

impl<B: Backend> Draw<B> for DirectoryViewer {
    fn draw(&mut self, rect: Rect, frame: &mut Frame<B>) {
        if rect.width <= 1 || rect.height <= 1 {
            return;
        }

        let window = scroll_window(
            self.entries.index(),
            self.entries.len(),
            rect.height as usize,
        );

        let items = &self.entries[window.start..window.end];

        for (i, item) in items.iter().enumerate() {
            let rendered = RenderedItem::new(item, item.id == self.highlighted);

            let pos = Rect {
                y: rect.y + (i as u16),
                height: 1,
                ..rect
            };

            frame.render_widget(rendered, pos);
        }
    }
}

pub enum DirectoryResult {
    Ok,
    ViewChild(NodeID),
    ViewParent(NodeID),
    EntryHighlight(NodeID),
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
    pub fn next(&mut self) -> &T {
        self.index = (self.index + 1) % self.items.len().max(1);
        self.selected()
    }

    #[inline(always)]
    pub fn prev(&mut self) -> &T {
        self.index = if self.index == 0 {
            self.items.len().saturating_sub(1)
        } else {
            self.index - 1
        };

        self.selected()
    }

    #[inline(always)]
    pub fn selected(&self) -> &T {
        &self.items[self.index]
    }

    #[inline(always)]
    pub fn selected_mut(&mut self) -> &mut T {
        &mut self.items[self.index]
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
    pub id: NodeID,
    pub entry: Rc<ArchiveEntry>,
    pub selected: bool,
    pub size: String,
}

impl AsRef<ArchiveEntry> for DirectoryEntry {
    fn as_ref(&self) -> &ArchiveEntry {
        &self.entry
    }
}

struct RenderedItem<'a> {
    inner: &'a DirectoryEntry,
    highlighted: bool,
}

impl<'a> RenderedItem<'a> {
    fn new(inner: &'a DirectoryEntry, highlighted: bool) -> Self {
        Self { inner, highlighted }
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

impl<'a> Widget for RenderedItem<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        const BASE_NAME_OFFSET: u16 = 1;
        const BASE_SIZE_OFFSET: u16 = 1;
        const MIN_SPACING: u16 = 1;

        let name_offset = if self.inner.selected {
            BASE_NAME_OFFSET * 2
        } else {
            BASE_NAME_OFFSET
        };

        if area.width <= name_offset || area.height == 0 {
            return;
        }

        self.apply_line_color(area, buf);

        let style = if self.highlighted || self.inner.selected {
            Style::default().add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        buf.set_stringn(
            area.x + name_offset,
            area.y,
            &self.inner.entry.name,
            // This caps the maximum length to always show at least one free character at the end
            area.width.saturating_sub(name_offset + BASE_NAME_OFFSET) as usize,
            style,
        );

        let name_len = name_offset + UnicodeWidthStr::width(self.inner.entry.name.as_str()) as u16;
        let size_start = area
            .width
            .saturating_sub(self.inner.size.len() as u16)
            .saturating_sub(BASE_SIZE_OFFSET);
        let remaining_space = size_start.saturating_sub(MIN_SPACING);

        // Draw the description of the entry only if we have enough room for it
        if remaining_space >= name_len {
            buf.set_string(area.x + size_start, area.y, &self.inner.size, style);
        }
    }
}

/// Calculate how many items are visible based off a given cursor position.
///
/// Returns a range that represents the visible bounds.
fn scroll_window(cursor: usize, num_items: usize, height: usize) -> Range<usize> {
    // Scrolling will only happen if the cursor is beyond this threshold
    let base_threshold = height / 2;

    if cursor < base_threshold || num_items <= height {
        let range = Range {
            start: 0,
            end: num_items.min(height),
        };

        return range;
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

    Range { start, end }
}
