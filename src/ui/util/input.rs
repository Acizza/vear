use crate::ui::colors;

use super::{
    fill_area, pad_rect_left, text_fragments::Fragment, text_fragments::TextFragments, SimpleText,
};
use crossterm::event::KeyCode;
use tui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{StatefulWidget, Widget},
};
use unicode_segmentation::GraphemeCursor;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub struct Input {
    desc: &'static str,
    style: Style,
}

impl Input {
    pub const DEFAULT_BG_COLOR: Color = Color::Rgb(40, 40, 40);

    pub fn new(desc: &'static str) -> Self {
        Self {
            desc,
            style: Style::default()
                .bg(Self::DEFAULT_BG_COLOR)
                .fg(colors::WHITE),
        }
    }
}

impl StatefulWidget for Input {
    type State = InputState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        fill_area(area, buf, |cell| {
            cell.set_style(self.style);
        });

        let area = pad_rect_left(area, 1);

        let fragment_items = [(self.desc, self.style).into(), (" :> ", self.style).into()];

        let fragments = TextFragments::new(&fragment_items);
        fragments.render(area, buf);

        let offset = Fragment::total_len(&fragment_items);
        let input_area = pad_rect_left(area, offset);
        let input_text = SimpleText::new(state.visible_slice(input_area.width as usize));

        input_text.render(input_area, buf);
        state.update_cursor_pos(input_area);
    }
}

pub struct InputState {
    caret: Caret,
    pub cursor_pos: Option<(u16, u16)>,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            caret: Caret::new(),
            cursor_pos: None,
        }
    }

    pub fn process_key(&mut self, key: KeyCode) -> InputResult {
        match key {
            KeyCode::Char(ch) => {
                self.caret.push(ch);
                InputResult::Ok
            }
            KeyCode::Backspace => {
                self.caret.pop();
                InputResult::Ok
            }
            KeyCode::Enter => InputResult::ProcessInput(&self.caret.buffer),
            KeyCode::Left => {
                self.caret.move_left();
                InputResult::Ok
            }
            KeyCode::Right => {
                self.caret.move_right();
                InputResult::Ok
            }
            KeyCode::Home | KeyCode::Up => {
                self.caret.move_front();
                InputResult::Ok
            }
            KeyCode::End | KeyCode::Down => {
                self.caret.move_end();
                InputResult::Ok
            }
            KeyCode::Esc => InputResult::Return,
            _ => InputResult::Ok,
        }
    }

    fn visible_offset(&self, max_width: usize) -> usize {
        // Make room for the cursor
        let max_width = max_width.saturating_sub(1);

        if self.caret.display_offset < max_width as usize {
            return 0;
        }

        let desired_offset = self.caret.display_offset - max_width as usize;
        let mut cursor = GraphemeCursor::new(0, self.caret.buffer.len(), true);

        // TODO: this can probably be optimized
        for _ in 0..desired_offset {
            match cursor.next_boundary(&self.caret.buffer, 0) {
                Ok(Some(_)) => (),
                Ok(None) => break,
                Err(_) => return 0,
            }
        }

        cursor.cur_cursor()
    }

    fn update_cursor_pos(&mut self, area: Rect) {
        if area.width < 1 || area.height < 1 {
            return;
        }

        let offset = (self.caret.display_offset as u16).min(area.width);

        self.cursor_pos = Some((area.x + offset, area.y));
    }

    fn visible_slice(&self, width: usize) -> &str {
        let start = self.visible_offset(width);
        let end = (start + width).min(self.caret.buffer.len());
        &self.caret.buffer[start..end]
    }
}

struct Caret {
    buffer: String,
    cursor: GraphemeCursor,
    display_offset: usize,
}

impl Caret {
    fn new() -> Self {
        Self {
            buffer: String::new(),
            cursor: GraphemeCursor::new(0, 0, true),
            display_offset: 0,
        }
    }

    fn push(&mut self, ch: char) {
        let pos = self.pos();

        self.buffer.insert(pos, ch);
        self.cursor = GraphemeCursor::new(pos + ch.len_utf8(), self.buffer.len(), true);

        self.display_offset += UnicodeWidthChar::width(ch).unwrap_or(0);
    }

    fn pop(&mut self) {
        if self.pos() == 0 {
            return;
        }

        let pos = match self.cursor.prev_boundary(&self.buffer, 0).ok().flatten() {
            Some(pos) => pos,
            None => return,
        };

        let ch = self.buffer.remove(pos);
        let width = UnicodeWidthChar::width(ch).unwrap_or(0);

        self.display_offset = self.display_offset.saturating_sub(width);
        self.cursor = GraphemeCursor::new(pos, self.buffer.len(), true);
    }

    fn move_left(&mut self) {
        if self.pos() == 0 {
            return;
        }

        let old_pos = self.pos();

        if let Some(new_pos) = self.cursor.prev_boundary(&self.buffer, 0).ok().flatten() {
            let slice = &self.buffer[new_pos..old_pos];
            let width = UnicodeWidthStr::width(slice);

            self.display_offset = self.display_offset.saturating_sub(width);
        }
    }

    fn move_right(&mut self) {
        if self.pos() >= self.buffer.len() {
            return;
        }

        let old_pos = self.pos();

        if let Some(new_pos) = self.cursor.next_boundary(&self.buffer, 0).ok().flatten() {
            let slice = &self.buffer[old_pos..new_pos];
            let width = UnicodeWidthStr::width(slice);

            self.display_offset += width;
        }
    }

    fn move_front(&mut self) {
        self.cursor.set_cursor(0);
        self.display_offset = 0;
    }

    fn move_end(&mut self) {
        self.cursor.set_cursor(self.buffer.len());
        self.display_offset = UnicodeWidthStr::width(self.buffer.as_str());
    }

    #[inline(always)]
    fn pos(&self) -> usize {
        self.cursor.cur_cursor()
    }
}

pub enum InputResult<'a> {
    Ok,
    Return,
    ProcessInput(&'a str),
}
