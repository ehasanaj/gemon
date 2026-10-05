use ratatui::layout::{Constraint, Direction, Layout, Rect};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RootLayout {
    pub header: Rect,
    pub body: Rect,
    pub footer: Rect,
    pub tabs: Rect,
    pub context: Rect,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RequestsLayout {
    pub saved: Rect,
    pub workspace: Rect,
    pub saved_filter: Rect,
    pub saved_list: Rect,
    pub composer: Rect,
    pub pairs: Rect,
    pub headers: Rect,
    pub form_data: Rect,
    pub body: Rect,
    pub response: Rect,
    pub response_metadata: Rect,
    pub response_body: Rect,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EnvironmentsLayout {
    pub list: Rect,
    pub values: Rect,
}

pub fn root(area: Rect) -> RootLayout {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(area);

    let header = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(chunks[0]);

    RootLayout {
        header: chunks[0],
        body: chunks[1],
        footer: chunks[2],
        tabs: header[0],
        context: header[1],
    }
}

pub fn requests(
    area: Rect,
    saved_width: u16,
    pair_left_percent: u16,
    body_percent: u16,
) -> RequestsLayout {
    let saved_width = clamp_panel_width(saved_width, area.width);
    let pair_left_percent = pair_left_percent.clamp(20, 80);
    let body_percent = body_percent.clamp(20, 75);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(saved_width), Constraint::Min(0)])
        .split(area);

    let saved_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(chunks[0]);

    let workspace = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),
            Constraint::Length(9),
            Constraint::Percentage(body_percent),
            Constraint::Min(0),
        ])
        .split(chunks[1]);

    let pairs = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(pair_left_percent),
            Constraint::Percentage(100 - pair_left_percent),
        ])
        .split(workspace[1]);

    let response = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(workspace[3]);

    RequestsLayout {
        saved: chunks[0],
        workspace: chunks[1],
        saved_filter: saved_chunks[0],
        saved_list: saved_chunks[1],
        composer: workspace[0],
        pairs: workspace[1],
        headers: pairs[0],
        form_data: pairs[1],
        body: workspace[2],
        response: workspace[3],
        response_metadata: response[0],
        response_body: response[1],
    }
}

pub fn environments(area: Rect, list_width: u16) -> EnvironmentsLayout {
    let list_width = clamp_panel_width(list_width, area.width);
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(list_width), Constraint::Min(0)])
        .split(area);

    EnvironmentsLayout {
        list: chunks[0],
        values: chunks[1],
    }
}

pub fn contains(rect: Rect, column: u16, row: u16) -> bool {
    column >= rect.x
        && column < rect.x.saturating_add(rect.width)
        && row >= rect.y
        && row < rect.y.saturating_add(rect.height)
}

pub fn clamp_panel_width(width: u16, total_width: u16) -> u16 {
    if total_width == 0 {
        return 0;
    }

    let min = 24.min(total_width);
    let max = total_width.saturating_sub(30).max(min);
    width.clamp(min, max)
}
