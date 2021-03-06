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
    pub mount_state: MountState,
}

impl KeyHints {
    const COLOR: Color = Color::DarkGray;
    const MOUNTED_COLOR: Color = Color::Cyan;

    fn draw_extract_hint(&self, area: Rect, buf: &mut Buffer) {
        let style = Style::default().fg(Self::COLOR);

        let extract_all = KeyHint::with_char(self.extract_to_dir_key, "to dir", style);
        let extract_to_cwd = KeyHint::with_char(self.extract_to_cwd_key, "to cwd", style);

        let extract_items =
            text_fragments![style, "Extract [", extract_all, ", ", extract_to_cwd, ']'];

        let extract_keys = TextFragments::new(&extract_items);
        extract_keys.render(area, buf);
    }

    fn draw_mount_hint(&self, area: Rect, buf: &mut Buffer) {
        match self.mount_state {
            MountState::Mounted { unmount } => {
                let style = Style::default().fg(Self::MOUNTED_COLOR);

                let unmount_hint = KeyHint::with_str(unmount, "unmount", style);

                let mount_items = text_fragments![style, "Mount [", unmount_hint, ']'];

                let mount_keys = TextFragments::new(&mount_items).alignment(Alignment::Right);
                mount_keys.render(area, buf);
            }
            MountState::Unmounted {
                mount_at_tmp,
                mount_at_dir,
            } => {
                let style = Style::default().fg(Self::COLOR);

                let mount_at_tmp = KeyHint::with_char(mount_at_tmp, "at tmp", style);
                let mount_at_dir = KeyHint::with_char(mount_at_dir, "at dir", style);

                let mount_items =
                    text_fragments![style, "Mount [", mount_at_tmp, ", ", mount_at_dir, ']'];

                let mount_keys = TextFragments::new(&mount_items).alignment(Alignment::Right);
                mount_keys.render(area, buf);
            }
        }
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
    const SEPARATOR: &'static str = " -> ";

    fn with_char(key: char, desc: &'static str, style: Style) -> Self {
        let items = [
            (key, style).into(),
            (Self::SEPARATOR, style).into(),
            (desc, style).into(),
        ];

        Self { items }
    }

    fn with_str(key: &'static str, desc: &'static str, style: Style) -> Self {
        let items = [
            (key, style).into(),
            (Self::SEPARATOR, style).into(),
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

pub enum MountState {
    Mounted {
        unmount: &'static str,
    },
    Unmounted {
        mount_at_tmp: char,
        mount_at_dir: char,
    },
}
