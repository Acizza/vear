use super::{Backend, Draw, Frame, KeyCode, Panel};
use std::borrow::Cow;
use std::ops::Deref;
use tui::buffer::{Buffer, Cell};
use tui::layout::Rect;
use tui::style::{Color, Modifier, Style};
use tui::widgets::Widget;

pub struct DirectoryViewer<'a> {
    items: WrappedSelection<DirectoryEntry<'a>>,
}

impl<'a> DirectoryViewer<'a> {
    pub fn new(items: Vec<DirectoryEntry<'a>>) -> Self {
        Self {
            items: WrappedSelection::new(items),
        }
    }
}

impl<'a> Panel for DirectoryViewer<'a> {
    type KeyResult = ();

    fn process_key(&mut self, key: KeyCode) -> Self::KeyResult {
        match key {
            KeyCode::Up => {
                self.items.prev();
            }
            KeyCode::Down => {
                self.items.next();
            }
            KeyCode::Char(' ') => {
                let entry = match self.items.selected_mut() {
                    Some(entry) => entry,
                    None => return,
                };

                entry.selected = !entry.selected;
                self.items.next();
            }
            _ => (),
        }
    }
}

impl<'a, B: Backend> Draw<B> for DirectoryViewer<'a> {
    fn draw(&mut self, rect: Rect, frame: &mut Frame<B>) {
        for (i, item) in self.items.iter().enumerate() {
            let highlighted = self.items.index() == i;
            let rendered_item = RenderedEntry::new(item, highlighted);

            let cur_height = i as u16;

            if cur_height >= rect.height {
                break;
            }

            let pos = Rect {
                y: rect.y + cur_height,
                height: 1,
                ..rect
            };

            frame.render_widget(rendered_item, pos);
        }
    }
}

struct WrappedSelection<T> {
    items: Vec<T>,
    index: usize,
}

impl<T> WrappedSelection<T> {
    pub fn new(items: Vec<T>) -> Self {
        Self { items, index: 0 }
    }

    #[inline(always)]
    pub fn next(&mut self) -> Option<&T> {
        self.index = (self.index + 1) % self.items.len();
        self.items.get(self.index)
    }

    #[inline(always)]
    pub fn prev(&mut self) -> Option<&T> {
        self.index = if self.index == 0 {
            self.items.len() - 1
        } else {
            self.index - 1
        };

        self.items.get(self.index)
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

pub struct DirectoryEntry<'a> {
    pub name: Cow<'a, str>,
    pub size_bytes: u64,
    pub kind: EntryKind,
    pub selected: bool,
}

#[derive(Copy, Clone)]
pub enum EntryKind {
    File,
    Directory,
}

impl<'a> DirectoryEntry<'a> {
    fn size_display(&self) -> String {
        const BASE_UNIT: u64 = 1024;

        macro_rules! match_units {
            ($($pow:expr => $unit_name:expr => $formatter:expr),+) => {{
                $(
                let total_bytes = BASE_UNIT.pow($pow);

                if self.size_bytes >= total_bytes {
                    return format!(concat!($formatter, " {}"), self.size_bytes as f64 / total_bytes as f64, $unit_name);
                }
                )+

                #[cold]
                format!("0 B")
            }};
        }

        match_units!(
            4 => "TB" => "{:.02}",
            3 => "GB" => "{:.02}",
            2 => "MB" => "{:.02}",
            1 => "KB" => "{:.02}",
            0 => "B" => "{}"
        )
    }
}

struct RenderedEntry<'a> {
    entry: &'a DirectoryEntry<'a>,
    highlighted: bool,
}

impl<'a> RenderedEntry<'a> {
    fn new(entry: &'a DirectoryEntry<'a>, highlighted: bool) -> Self {
        Self { entry, highlighted }
    }

    fn apply_line_color(&self, area: Rect, buf: &mut Buffer) {
        const WHITE: Color = Color::Rgb(225, 225, 225);
        const BLACK: Color = Color::Rgb(10, 10, 10);

        let primary_color = match self.entry.kind {
            EntryKind::File => WHITE,
            EntryKind::Directory => Color::LightBlue,
        };

        match (self.highlighted, self.entry.selected) {
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

        let style = if self.highlighted || self.entry.selected {
            Style::default().add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let name_offset = if self.entry.selected {
            BASE_NAME_OFFSET * 2
        } else {
            BASE_NAME_OFFSET
        };

        buf.set_stringn(
            area.x + name_offset,
            area.y,
            self.entry.name.as_ref(),
            // This caps the maximum length to always show at least one free character at the end
            area.width.saturating_sub(name_offset + BASE_NAME_OFFSET) as usize,
            style,
        );

        let size = self.entry.size_display();

        let name_len = name_offset + self.entry.name.len() as u16;
        let size_start = area
            .width
            .saturating_sub(size.len() as u16)
            .saturating_sub(BASE_SIZE_OFFSET);
        let remaining_space = size_start.saturating_sub(MIN_SPACING);

        // Draw the size of the entry only if we have enough room for it
        if remaining_space >= name_len {
            buf.set_string(area.x + size_start, area.y, size, style);
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
