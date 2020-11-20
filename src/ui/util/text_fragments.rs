use super::alignment_offset;
use tui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::Style,
    widgets::Widget,
};

/// Draw text in fragments without allocating.
///
/// This serves as a relative simple replacement for `tui::widget::Paragraph`.
/// It is meant to be used for relative simple text layouts that don't need multiple lines or scrolling.
///
/// On naive benchmarks, this solution is at least 4x as fast as `tui::widget::Paragraph`.
pub struct TextFragments<'a> {
    items: &'a [Fragment<'a>],
    alignment: Alignment,
}

impl<'a> TextFragments<'a> {
    pub fn new(items: &'a [Fragment<'a>]) -> Self {
        Self {
            items,
            alignment: Alignment::Left,
        }
    }

    #[inline(always)]
    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    fn can_draw_at_x(area: Rect, x: u16) -> bool {
        x < area.x + area.width && area.height > 0
    }
}

impl<'a> Widget for TextFragments<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let mut offset_x =
            alignment_offset(self.alignment, area.width, Fragment::total_len(&self.items));

        for item in self.items {
            let start_x = area.x + offset_x;

            match item {
                Fragment::Text(text, style) => {
                    let len = text.len() as u16;

                    if !Self::can_draw_at_x(area, start_x + len) {
                        return;
                    }

                    buf.set_string(start_x, area.y, text, *style);
                    offset_x += len;
                }
                Fragment::Char(ch, style) => {
                    if !Self::can_draw_at_x(area, start_x) {
                        return;
                    }

                    buf.get_mut(start_x, area.y).set_char(*ch).set_style(*style);
                    offset_x += 1;
                }
                Fragment::Widget(widget) => {
                    let fragments = widget.fragments();
                    let total_len = widget.total_fragments_len();

                    if !Self::can_draw_at_x(area, start_x + total_len) {
                        return;
                    }

                    let text = Self::new(fragments);

                    let widget_area = Rect {
                        x: start_x,
                        width: area.width.saturating_sub(offset_x),
                        ..area
                    };

                    text.render(widget_area, buf);
                    offset_x += total_len;
                }
            }
        }
    }
}

pub enum Fragment<'a> {
    Text(&'a str, Style),
    Char(char, Style),
    Widget(&'a dyn FragmentedWidget),
}

impl<'a> Fragment<'a> {
    /// Calculate the total length of each given item.
    pub fn total_len(items: &[Self]) -> u16 {
        items.iter().fold(0, |acc, item| match item {
            Self::Text(text, _) => acc + text.len() as u16,
            Self::Char(_, _) => acc + 1,
            Self::Widget(widget) => acc + widget.total_fragments_len(),
        })
    }
}

impl<'a> From<(&'a str, Style)> for Fragment<'a> {
    fn from((value, style): (&'a str, Style)) -> Self {
        Self::Text(value, style)
    }
}

impl<'a> From<(char, Style)> for Fragment<'a> {
    fn from((ch, style): (char, Style)) -> Self {
        Self::Char(ch, style)
    }
}

impl<'a, W> From<&'a W> for Fragment<'a>
where
    W: FragmentedWidget,
{
    fn from(widget: &'a W) -> Self {
        Self::Widget(widget)
    }
}

#[macro_export]
macro_rules! _impl_text_fragment {
    ($base_style:expr, $text:expr => $style:expr) => {
        ($text, $style).into()
    };

    ($base_style:expr, $widget:ident) => {
        (&$widget).into()
    };

    ($base_style:expr, $text:expr) => {
        ($text, $base_style).into()
    };
}

/// Construct an array of text fragments for use with [TextFragments].
///
/// The first argument should be the default style to apply to each fragment.
/// The rest should be a comma separated list of each fragment.
///
/// You can override the style of a text / char element by adding `=>` with the desired
/// style before the trailing comma.
#[macro_export]
macro_rules! text_fragments {
    ($base_style:expr, $($token:tt),+) => {
        [
            $(crate::_impl_text_fragment!($base_style, $token)),+
        ]
    };
}

/// Represents a widget that can be rendered as text fragments.
pub trait FragmentedWidget {
    /// Returns the combined length of each fragment.
    fn total_fragments_len(&self) -> u16 {
        Fragment::total_len(self.fragments())
    }

    /// Returns a reference to every text fragment.
    ///
    /// The [text_fragments] macro can be used in some cases to build the array.
    fn fragments(&self) -> &[Fragment];
}
