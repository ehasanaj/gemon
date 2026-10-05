use super::{App, AppCommand, DragTarget, Focus, Overlay, PairTarget, RequestTab, ResponseTab, Screen};
use crate::tui::layout::{self, contains, RequestsLayout, RequestsParams};
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

/// Width of the brand label at the left of the header bar.
pub const BRAND: &str = " gemon ";

impl App {
    pub fn requests_layout(&self, body: Rect) -> RequestsLayout {
        layout::requests(
            body,
            RequestsParams {
                sidebar_visible: self.sidebar_visible,
                sidebar_focused: self.focus == Focus::Sidebar,
                sidebar_width: self.effective_sidebar_width(),
                response_percent: self.response_percent,
                response_zoomed: self.response_zoomed,
                editor_focused: matches!(
                    self.focus,
                    Focus::Headers | Focus::Body | Focus::Form | Focus::Auth
                ),
            },
        )
    }

    pub fn header_tab_labels(&self) -> Vec<String> {
        Screen::ALL
            .iter()
            .map(|screen| format!(" {} {} ", screen.key(), screen.title()))
            .collect()
    }

    pub fn header_tab_rects(&self, header: Rect) -> Vec<Rect> {
        layout::label_spans(header.x + BRAND.len() as u16 + 1, header.y, &self.header_tab_labels(), 1)
    }

    pub fn request_tab_labels(&self) -> Vec<String> {
        RequestTab::ALL
            .iter()
            .map(|tab| match tab {
                RequestTab::Headers => count_label("Headers", self.draft.headers.len()),
                RequestTab::Body => {
                    if self.draft.body_text().is_some() {
                        String::from(" Body ● ")
                    } else {
                        String::from(" Body ")
                    }
                }
                RequestTab::Form => count_label("Form", self.draft.form.len()),
                RequestTab::Auth => {
                    if self.draft.secure {
                        String::from(" Auth ✓ ")
                    } else {
                        String::from(" Auth ")
                    }
                }
            })
            .collect()
    }

    pub fn response_tab_labels(&self) -> Vec<String> {
        let headers = self.response_view().map(|view| view.headers.len()).unwrap_or(0);
        vec![String::from(" Body "), count_label("Headers", headers)]
    }

    /// Tab labels sit in the top border, one column right of the corner.
    pub fn title_tab_rects(area: Rect, labels: &[String]) -> Vec<Rect> {
        layout::label_spans(area.x + 1, area.y, labels, 1)
    }

    /// Rows of the saved-request list, below the search line.
    pub fn sidebar_list_area(sidebar: Rect) -> Rect {
        let inner = layout::inner(sidebar);
        Rect::new(inner.x, inner.y + 1, inner.width, inner.height.saturating_sub(1))
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent, area: Rect) -> AppCommand {
        if layout::too_small(area) {
            return AppCommand::None;
        }
        if matches!(
            mouse.kind,
            MouseEventKind::Down(_) | MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
        ) {
            self.note_user_input();
        }
        let limit = self.overlay_scroll_limit.get();
        if let Some(overlay) = self.overlay.as_mut() {
            let delta = match mouse.kind {
                MouseEventKind::ScrollUp => -1,
                MouseEventKind::ScrollDown => 1,
                _ => return AppCommand::None,
            };
            match overlay {
                Overlay::Help { scroll } => *scroll = scroll.saturating_add_signed(delta * 3).min(limit),
                Overlay::TextView(view) => {
                    view.scroll = view.scroll.saturating_add_signed(delta * 3).min(limit)
                }
                _ => {}
            }
            return AppCommand::None;
        }

        let shell = layout::shell(area);
        let (column, row) = (mouse.column, mouse.row);
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if contains(shell.header, column, row) {
                    self.click_header(column, row, shell.header);
                } else if !self.start_drag(column, row, shell.body) {
                    self.click_body(column, row, shell.body);
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => self.update_drag(column, row, shell.body),
            MouseEventKind::Up(MouseButton::Left) => self.drag = None,
            MouseEventKind::ScrollUp => self.scroll_at(column, row, shell.body, -3),
            MouseEventKind::ScrollDown => self.scroll_at(column, row, shell.body, 3),
            _ => {}
        }
        AppCommand::None
    }

    fn click_header(&mut self, column: u16, row: u16, header: Rect) {
        let rects = self.header_tab_rects(header);
        if let Some(index) = rects.iter().position(|rect| contains(*rect, column, row)) {
            self.show_screen(Screen::ALL[index]);
        }
    }

    fn start_drag(&mut self, column: u16, row: u16, body: Rect) -> bool {
        match self.screen {
            Screen::Requests => {
                let panels = self.requests_layout(body);
                if !panels.sidebar.is_empty()
                    && !panels.url.is_empty()
                    && column == panels.sidebar.right().saturating_sub(1)
                    && contains(panels.sidebar, column, row)
                {
                    self.drag = Some(DragTarget::Sidebar);
                    return true;
                }
                let on_response_tab = Self::title_tab_rects(panels.response, &self.response_tab_labels())
                    .iter()
                    .any(|rect| contains(*rect, column, row));
                if !panels.editor.is_empty()
                    && row == panels.response.y
                    && contains(panels.response, column, row)
                    && !on_response_tab
                {
                    self.drag = Some(DragTarget::ResponseSplit);
                    return true;
                }
            }
            Screen::Environments => {
                let panels = layout::environments(body, self.env_list_width);
                if panels.list.x != panels.vars.x
                    && column == panels.list.right().saturating_sub(1)
                    && contains(panels.list, column, row)
                {
                    self.drag = Some(DragTarget::EnvironmentList);
                    return true;
                }
            }
        }
        false
    }

    fn update_drag(&mut self, column: u16, row: u16, body: Rect) {
        match self.drag {
            Some(DragTarget::Sidebar) => {
                let width = column.saturating_sub(body.x).saturating_add(1);
                self.sidebar_width = Some(layout::clamp_sidebar_width(width, body.width));
            }
            Some(DragTarget::EnvironmentList) => {
                let width = column.saturating_sub(body.x).saturating_add(1);
                self.env_list_width = layout::clamp_sidebar_width(width, body.width);
            }
            Some(DragTarget::ResponseSplit) => {
                self.response_percent = layout::response_percent_at(row, body).clamp(20, 85);
            }
            None => {}
        }
    }

    fn click_body(&mut self, column: u16, row: u16, body: Rect) {
        match self.screen {
            Screen::Requests => self.click_requests(column, row, body),
            Screen::Environments => self.click_environments(column, row, body),
        }
    }

    fn click_requests(&mut self, column: u16, row: u16, body: Rect) {
        let panels = self.requests_layout(body);

        if contains(panels.sidebar, column, row) {
            let list = Self::sidebar_list_area(panels.sidebar);
            if contains(list, column, row) {
                let position = self.sidebar_offset.get() + usize::from(row - list.y);
                let already_selected = self.selected_visible_position() == Some(position);
                self.set_focus(Focus::Sidebar);
                if position < self.visible_request_indices().len() {
                    self.select_visible_request(position);
                    if already_selected {
                        self.open_selected_request();
                    }
                }
            } else {
                self.start_sidebar_search();
            }
        } else if contains(panels.url, column, row) {
            let method_end = panels.url.x + 10;
            self.set_focus(if column < method_end { Focus::Method } else { Focus::Url });
        } else if contains(panels.editor, column, row) {
            if row == panels.editor.y {
                let rects = Self::title_tab_rects(panels.editor, &self.request_tab_labels());
                if let Some(index) = rects.iter().position(|rect| contains(*rect, column, row)) {
                    self.set_focus(RequestTab::ALL[index].focus());
                }
                return;
            }
            let focus = self.request_tab.focus();
            self.set_focus(focus);
            let target = match self.request_tab {
                RequestTab::Headers => PairTarget::Header,
                RequestTab::Form => PairTarget::Form,
                _ => return,
            };
            let inner = layout::inner(panels.editor);
            let len = self.draft.pairs(target).len();
            let rows = usize::from(inner.height.saturating_sub(1));
            let offset = layout::follow_offset(self.draft.selected_pair(target), rows);
            if let Some(index) = row
                .checked_sub(inner.y + 1)
                .map(|relative| offset + usize::from(relative))
                .filter(|index| *index < len)
            {
                match target {
                    PairTarget::Header => self.draft.selected_header = index,
                    PairTarget::Form => self.draft.selected_form = index,
                }
            }
        } else if contains(panels.response, column, row) {
            self.set_focus(Focus::Response);
            if row == panels.response.y {
                let rects = Self::title_tab_rects(panels.response, &self.response_tab_labels());
                if let Some(index) = rects.iter().position(|rect| contains(*rect, column, row)) {
                    let tab = [ResponseTab::Body, ResponseTab::Headers][index];
                    if tab != self.response_tab {
                        self.response_tab = tab;
                        self.response_scroll = 0;
                        if self.response_search.is_some() {
                            self.update_response_search();
                        }
                    }
                }
            }
        }
    }

    fn click_environments(&mut self, column: u16, row: u16, body: Rect) {
        let panels = layout::environments(body, self.env_list_width);
        if contains(panels.list, column, row) {
            self.set_focus(Focus::EnvList);
            let inner = layout::inner(panels.list);
            let offset = layout::follow_offset(self.selected_env, usize::from(inner.height));
            if let Some(index) = row.checked_sub(inner.y).map(|r| offset + usize::from(r)) {
                self.select_env_row(index);
            }
        } else if contains(panels.vars, column, row) {
            self.set_focus(Focus::EnvVars);
            let inner = layout::inner(panels.vars);
            let rows = usize::from(inner.height.saturating_sub(1));
            let offset = layout::follow_offset(self.selected_env_var, rows);
            if let Some(index) = row
                .checked_sub(inner.y + 1)
                .map(|relative| offset + usize::from(relative))
                .filter(|index| *index < self.selected_env_values().len())
            {
                self.selected_env_var = index;
            }
        } else if contains(panels.auth, column, row) {
            self.set_focus(Focus::EnvVars);
        }
    }

    fn scroll_at(&mut self, column: u16, row: u16, body: Rect, delta: isize) {
        match self.screen {
            Screen::Requests => {
                let panels = self.requests_layout(body);
                if contains(panels.sidebar, column, row) {
                    self.move_request_selection(delta.signum());
                } else if contains(panels.response, column, row) {
                    self.scroll_response(delta);
                } else if contains(panels.editor, column, row) {
                    match self.request_tab {
                        RequestTab::Headers | RequestTab::Form => {
                            let target = if self.request_tab == RequestTab::Headers {
                                PairTarget::Header
                            } else {
                                PairTarget::Form
                            };
                            let len = self.draft.pairs(target).len();
                            let next = super::step_index(self.draft.selected_pair(target), len, delta.signum());
                            match target {
                                PairTarget::Header => self.draft.selected_header = next,
                                PairTarget::Form => self.draft.selected_form = next,
                            }
                        }
                        _ => {}
                    }
                }
            }
            Screen::Environments => {
                let panels = layout::environments(body, self.env_list_width);
                if contains(panels.list, column, row) {
                    let next = super::step_index(self.selected_env, self.env_rows().len(), delta.signum());
                    self.select_env_row(next);
                } else if contains(panels.vars, column, row) {
                    self.selected_env_var = super::step_index(
                        self.selected_env_var,
                        self.selected_env_values().len(),
                        delta.signum(),
                    );
                }
            }
        }
    }
}

fn count_label(name: &str, count: usize) -> String {
    if count == 0 {
        format!(" {name} ")
    } else {
        format!(" {name} {count} ")
    }
}
