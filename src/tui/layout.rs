//! Screen geometry shared by rendering and mouse hit-testing, so both always agree.

use ratatui::layout::Rect;
use unicode_width::UnicodeWidthStr;

pub const MIN_WIDTH: u16 = 44;
pub const MIN_HEIGHT: u16 = 12;
/// Below this width the sidebar and the request workspace take turns filling the screen.
pub const NARROW_WIDTH: u16 = 72;
const URL_BAR_HEIGHT: u16 = 3;
const MIN_EDITOR_HEIGHT: u16 = 5;
const MIN_RESPONSE_HEIGHT: u16 = 5;
const AUTH_PANEL_HEIGHT: u16 = 4;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Shell {
    pub header: Rect,
    pub body: Rect,
    pub footer: Rect,
}

pub fn shell(area: Rect) -> Shell {
    let header_height = area.height.min(1);
    let footer_height = area.height.saturating_sub(header_height).min(1);
    Shell {
        header: Rect::new(area.x, area.y, area.width, header_height),
        body: Rect::new(
            area.x,
            area.y + header_height,
            area.width,
            area.height
                .saturating_sub(header_height)
                .saturating_sub(footer_height),
        ),
        footer: Rect::new(
            area.x,
            area.y + area.height.saturating_sub(footer_height),
            area.width,
            footer_height,
        ),
    }
}

pub fn too_small(area: Rect) -> bool {
    area.width < MIN_WIDTH || area.height < MIN_HEIGHT
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RequestsParams {
    pub sidebar_visible: bool,
    pub sidebar_focused: bool,
    pub sidebar_width: u16,
    pub response_percent: u16,
    pub response_zoomed: bool,
    pub editor_focused: bool,
}

/// Panels of the requests screen; hidden panels have an empty rect.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RequestsLayout {
    pub sidebar: Rect,
    pub url: Rect,
    pub editor: Rect,
    pub response: Rect,
}

pub fn requests(area: Rect, params: RequestsParams) -> RequestsLayout {
    if params.response_zoomed {
        return RequestsLayout {
            response: area,
            ..RequestsLayout::default()
        };
    }

    let (sidebar, main) = if area.width < NARROW_WIDTH {
        if params.sidebar_focused {
            (area, Rect::default())
        } else {
            (Rect::default(), area)
        }
    } else if params.sidebar_visible || params.sidebar_focused {
        let width = clamp_sidebar_width(params.sidebar_width, area.width);
        (
            Rect::new(area.x, area.y, width, area.height),
            Rect::new(area.x + width, area.y, area.width - width, area.height),
        )
    } else {
        (Rect::default(), area)
    };

    if main.is_empty() {
        return RequestsLayout {
            sidebar,
            ..RequestsLayout::default()
        };
    }

    let url_height = URL_BAR_HEIGHT.min(main.height);
    let rest = main.height - url_height;
    let response_height = response_height(rest, params.response_percent, params.editor_focused);
    let editor_height = rest - response_height;

    RequestsLayout {
        sidebar,
        url: Rect::new(main.x, main.y, main.width, url_height),
        editor: Rect::new(main.x, main.y + url_height, main.width, editor_height),
        response: Rect::new(
            main.x,
            main.y + url_height + editor_height,
            main.width,
            response_height,
        ),
    }
}

fn response_height(available: u16, percent: u16, editor_focused: bool) -> u16 {
    if available == 0 {
        return 0;
    }
    // Without room for both, the focused panel gets the space and the other shrinks to a
    // one-line strip, so whatever is being edited stays visible.
    if available < MIN_EDITOR_HEIGHT + MIN_RESPONSE_HEIGHT {
        let strip = 3.min(available / 2);
        return if editor_focused { strip } else { available - strip };
    }
    let target = (u32::from(available) * u32::from(percent.clamp(10, 90)) / 100) as u16;
    let min = MIN_RESPONSE_HEIGHT.min(available);
    let max = available.saturating_sub(MIN_EDITOR_HEIGHT).max(min);
    target.clamp(min, max)
}

pub fn clamp_sidebar_width(width: u16, total: u16) -> u16 {
    let max = (total * 2 / 5).max(20);
    width.clamp(20, max).min(total)
}

/// Converts a row inside `area` into the response share of the editor/response split.
pub fn response_percent_at(row: u16, area: Rect) -> u16 {
    let top = area.y.saturating_add(URL_BAR_HEIGHT);
    let available = area.height.saturating_sub(URL_BAR_HEIGHT).max(1);
    let from_top = row.saturating_sub(top).min(available);
    let response = available - from_top;
    ((u32::from(response) * 100) / u32::from(available)) as u16
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EnvironmentsLayout {
    pub list: Rect,
    pub vars: Rect,
    pub auth: Rect,
}

pub fn environments(area: Rect, list_width: u16) -> EnvironmentsLayout {
    let (list, right) = if area.width < NARROW_WIDTH {
        let list_height = (area.height * 2 / 5).max(5).min(area.height);
        (
            Rect::new(area.x, area.y, area.width, list_height),
            Rect::new(
                area.x,
                area.y + list_height,
                area.width,
                area.height - list_height,
            ),
        )
    } else {
        let width = clamp_sidebar_width(list_width, area.width);
        (
            Rect::new(area.x, area.y, width, area.height),
            Rect::new(area.x + width, area.y, area.width - width, area.height),
        )
    };

    let auth_height = AUTH_PANEL_HEIGHT.min(right.height);
    EnvironmentsLayout {
        list,
        vars: Rect::new(right.x, right.y, right.width, right.height - auth_height),
        auth: Rect::new(
            right.x,
            right.y + right.height - auth_height,
            right.width,
            auth_height,
        ),
    }
}

/// Horizontal spans of labels rendered one after another starting at `x`, separated by
/// `gap` columns. Used for clickable tab strips in headers and block titles.
pub fn label_spans(x: u16, y: u16, labels: &[String], gap: u16) -> Vec<Rect> {
    let mut cursor = x;
    labels
        .iter()
        .map(|label| {
            let width = label.width() as u16;
            let rect = Rect::new(cursor, y, width, 1);
            cursor = cursor.saturating_add(width).saturating_add(gap);
            rect
        })
        .collect()
}

/// First visible row for a list of `height` rows that keeps `selected` on screen with the
/// least scrolling from `offset`.
pub fn keep_in_view(offset: usize, selected: usize, height: usize) -> usize {
    let height = height.max(1);
    if selected < offset {
        selected
    } else if selected >= offset + height {
        selected + 1 - height
    } else {
        offset
    }
}

/// Stateless variant of [`keep_in_view`] for lists that do not remember their offset.
pub fn follow_offset(selected: usize, height: usize) -> usize {
    keep_in_view(0, selected, height)
}

pub fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}

pub fn inner(area: Rect) -> Rect {
    Rect::new(
        area.x.saturating_add(1),
        area.y.saturating_add(1),
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    )
}

pub fn contains(rect: Rect, column: u16, row: u16) -> bool {
    column >= rect.x
        && column < rect.x.saturating_add(rect.width)
        && row >= rect.y
        && row < rect.y.saturating_add(rect.height)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> RequestsParams {
        RequestsParams {
            sidebar_visible: true,
            sidebar_focused: false,
            sidebar_width: 30,
            response_percent: 55,
            response_zoomed: false,
            editor_focused: false,
        }
    }

    #[test]
    fn response_keeps_usable_height_on_small_terminals() {
        let shell = shell(Rect::new(0, 0, 80, 24));
        let layout = requests(shell.body, params());

        assert!(layout.response.height >= 8, "{layout:?}");
        assert!(layout.editor.height >= MIN_EDITOR_HEIGHT, "{layout:?}");
        assert_eq!(
            layout.url.height + layout.editor.height + layout.response.height,
            shell.body.height
        );
    }

    #[test]
    fn narrow_terminals_show_sidebar_or_workspace() {
        let area = Rect::new(0, 0, 60, 30);
        let workspace = requests(area, params());
        assert!(workspace.sidebar.is_empty());
        assert_eq!(workspace.url.width, 60);

        let sidebar = requests(
            area,
            RequestsParams {
                sidebar_focused: true,
                ..params()
            },
        );
        assert_eq!(sidebar.sidebar, area);
        assert!(sidebar.url.is_empty());
    }

    #[test]
    fn zoom_gives_response_the_whole_area() {
        let area = Rect::new(0, 1, 100, 30);
        let layout = requests(
            area,
            RequestsParams {
                response_zoomed: true,
                ..params()
            },
        );
        assert_eq!(layout.response, area);
        assert!(layout.sidebar.is_empty());
    }

    #[test]
    fn focused_panel_keeps_room_on_short_terminals() {
        let area = Rect::new(0, 1, 60, 10);
        let editing = requests(
            area,
            RequestsParams {
                editor_focused: true,
                ..params()
            },
        );
        assert!(editing.editor.height >= 4, "{editing:?}");
        let reading = requests(area, params());
        assert!(reading.response.height >= 4, "{reading:?}");
    }

    #[test]
    fn response_percent_round_trips_through_split_row() {
        let area = Rect::new(0, 1, 100, 40);
        let layout = requests(area, params());
        let percent = response_percent_at(layout.response.y, area);
        assert!((50..=60).contains(&percent), "{percent}");
    }
}
