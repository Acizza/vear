use crate::{
    text_fragments,
    ui::util::text_fragments::{Fragment, FragmentedWidget, TextFragments},
};
use tui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::Widget,
};

pub struct KeyHints {
    pub extract_to_dir_key: char,
    pub extract_to_cwd_key: char,
    pub mount_at_tmp_key: char,
    pub mount_at_dir_key: char,
}

impl KeyHints {
    const COLOR: Color = Color::DarkGray;

    fn draw_extract_hint(&self, area: Rect, buf: &mut Buffer) {
        let style = Style::default().fg(Self::COLOR);

        let extract_all = KeyHint::new(self.extract_to_dir_key, "to dir", style);
        let extract_to_cwd = KeyHint::new(self.extract_to_cwd_key, "to cwd", style);

        let extract_items =
            text_fragments![style, "Extract [", extract_all, ", ", extract_to_cwd, ']'];

        let extract_keys = TextFragments::new(&extract_items);
        extract_keys.render(area, buf);
    }

    fn draw_mount_hint(&self, area: Rect, buf: &mut Buffer) {
        let style = Style::default().fg(Self::COLOR);

        let mount_at_tmp = KeyHint::new(self.mount_at_tmp_key, "at tmp", style);
        let mount_at_dir = KeyHint::new(self.mount_at_dir_key, "at dir", style);

        let mount_items = text_fragments![style, "Mount [", mount_at_tmp, ", ", mount_at_dir, ']'];

        let mount_keys = TextFragments::new(&mount_items).alignment(Alignment::Right);
        mount_keys.render(area, buf);
    }
}

impl Widget for KeyHints {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let layout = Layout::default()
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .direction(Direction::Horizontal)
            .split(area);

        self.draw_extract_hint(layout[0], buf);
        self.draw_mount_hint(layout[1], buf);
    }
}

struct KeyHint<'a> {
    items: [Fragment<'a>; 3],
}

impl<'a> KeyHint<'a> {
    fn new(key: char, desc: &'static str, style: Style) -> Self {
        let items = [
            (key, style).into(),
            (" -> ", style).into(),
            (desc, style).into(),
        ];

        Self { items }
    }
}

impl<'a> FragmentedWidget for KeyHint<'a> {
    fn fragments(&self) -> &[Fragment] {
        &self.items
    }
}
