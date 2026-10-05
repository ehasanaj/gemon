use super::{
    input::TextInput,
    layout::{self, contains},
};
use crate::{
    config::{effector::Effector, types::GemonMethodType, GemonConfig},
    constants::NO_ENV,
    project::{
        import_openapi_requests as import_openapi_project_requests,
        project_handler::{
            add_authorization, add_env_value, create_project, delete_request, get_project,
            list_saved_requests, read_saved_rest_request, remove_authorization, remove_env,
            remove_env_value, save_request, set_selected_env, SavedRequestInfo,
        },
        Project,
    },
    request::{
        request_builder::{GemonRequest, GemonResponse, RequestBuilder},
        rest_request::GemonRestRequest,
    },
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use serde_json::Value;
use std::{collections::HashMap, time::Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppCommand {
    None,
    SendRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Requests,
    Environments,
    Help,
}

impl Tab {
    pub fn title(&self) -> &'static str {
        match self {
            Tab::Requests => "Requests",
            Tab::Environments => "Environments",
            Tab::Help => "Help",
        }
    }

    fn next(self) -> Tab {
        match self {
            Tab::Requests => Tab::Environments,
            Tab::Environments => Tab::Help,
            Tab::Help => Tab::Requests,
        }
    }

    fn previous(self) -> Tab {
        match self {
            Tab::Requests => Tab::Help,
            Tab::Environments => Tab::Requests,
            Tab::Help => Tab::Environments,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    SavedRequests,
    RequestFilter,
    Method,
    Url,
    RequestName,
    Secure,
    Headers,
    FormData,
    Body,
    Response,
    EnvList,
    EnvValues,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    Info,
    Success,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusLine {
    pub message: String,
    pub kind: StatusKind,
}

impl StatusLine {
    fn info(message: impl Into<String>) -> StatusLine {
        StatusLine {
            message: message.into(),
            kind: StatusKind::Info,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KeyValue {
    pub key: String,
    pub value: String,
}

impl KeyValue {
    fn from_map(map: &HashMap<String, String>) -> Vec<KeyValue> {
        let mut pairs = map
            .iter()
            .map(|(key, value)| KeyValue {
                key: key.clone(),
                value: value.clone(),
            })
            .collect::<Vec<_>>();
        pairs.sort_by(|left, right| left.key.cmp(&right.key));
        pairs
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestDraft {
    pub name: TextInput,
    pub method: GemonMethodType,
    pub url: TextInput,
    pub secure: bool,
    pub headers: Vec<KeyValue>,
    pub selected_header: usize,
    pub form_data: Vec<KeyValue>,
    pub selected_form_data: usize,
    pub body: TextInput,
}

impl RequestDraft {
    pub fn from_saved(name: &str, request: GemonRestRequest) -> RequestDraft {
        RequestDraft {
            name: TextInput::single(name),
            method: request.method(),
            url: TextInput::single(request.uri()),
            secure: false,
            headers: KeyValue::from_map(request.headers()),
            selected_header: 0,
            form_data: KeyValue::from_map(request.form_data()),
            selected_form_data: 0,
            body: TextInput::multiline(request.body().unwrap_or_default()),
        }
    }

    pub fn save_name(&self) -> String {
        self.name.value().trim().to_string()
    }

    fn validate_request(&self) -> Result<(), String> {
        if self.url.value().trim().is_empty() {
            return Err(String::from("URI is required before sending a request"));
        }

        if self
            .headers
            .iter()
            .chain(self.form_data.iter())
            .any(|pair| pair.key.trim().is_empty() && !pair.value.trim().is_empty())
        {
            return Err(String::from("Key/value rows with values also need keys"));
        }

        Ok(())
    }

    fn validate_save(&self) -> Result<(), String> {
        self.validate_request()?;
        if self.save_name().is_empty() {
            return Err(String::from("Request name is required before saving"));
        }
        Ok(())
    }

    fn to_config(&self, apply_env: bool, secure: bool) -> GemonConfig {
        GemonConfig::rest_request(
            self.method,
            self.text_value(self.url.value(), apply_env),
            Self::pairs_to_map(&self.headers, apply_env),
            self.body_value(apply_env),
            Self::pairs_to_map(&self.form_data, apply_env),
            secure,
        )
    }

    fn body_value(&self, apply_env: bool) -> Option<String> {
        let body = self.body.value();
        if body.is_empty() {
            None
        } else {
            Some(self.text_value(body, apply_env))
        }
    }

    fn text_value(&self, value: String, apply_env: bool) -> String {
        if apply_env {
            Effector::apply_env_to_string(value)
        } else {
            value
        }
    }

    fn pairs_to_map(pairs: &[KeyValue], apply_env: bool) -> HashMap<String, String> {
        pairs
            .iter()
            .filter(|pair| !pair.key.trim().is_empty())
            .map(|pair| {
                let key = pair.key.trim().to_string();
                let value = pair.value.clone();
                if apply_env {
                    (
                        Effector::apply_env_to_string(key),
                        Effector::apply_env_to_string(value),
                    )
                } else {
                    (key, value)
                }
            })
            .collect()
    }

    pub fn command_preview(&self) -> String {
        let mut args = vec![
            String::from("gemon"),
            String::from("-t=REST"),
            format!("-m={}", self.method),
        ];

        if !self.url.value().trim().is_empty() {
            args.push(format!("-u={}", self.url.value()));
        }

        for header in &self.headers {
            if !header.key.trim().is_empty() {
                args.push(format!("-h={}::{}", header.key.trim(), header.value));
            }
        }

        if self.secure {
            args.push(String::from("-sec"));
        }

        args.join(" ")
    }
}

impl Default for RequestDraft {
    fn default() -> Self {
        RequestDraft {
            name: TextInput::single(""),
            method: GemonMethodType::Get,
            url: TextInput::single(""),
            secure: false,
            headers: Vec::new(),
            selected_header: 0,
            form_data: Vec::new(),
            selected_form_data: 0,
            body: TextInput::multiline(""),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnvironmentView {
    pub name: String,
    pub selected: bool,
    pub values: Vec<KeyValue>,
    pub authorization_set: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectView {
    pub exists: bool,
    pub name: Option<String>,
    pub selected_environment: Option<String>,
    pub default_authorization_set: bool,
    pub last_response_path: Option<String>,
    pub environments: Vec<EnvironmentView>,
}

impl ProjectView {
    fn from_project(project: Project) -> ProjectView {
        let mut environments = project
            .environments()
            .iter()
            .map(|(name, environment)| EnvironmentView {
                name: name.clone(),
                selected: project.selected_environment_name() == Some(name.as_str()),
                values: KeyValue::from_map(environment.values_ref()),
                authorization_set: project.authorization_entries().contains_key(name),
            })
            .collect::<Vec<_>>();
        environments.sort_by(|left, right| left.name.cmp(&right.name));

        ProjectView {
            exists: true,
            name: Some(project.name().to_string()),
            selected_environment: project.selected_environment_name().map(String::from),
            default_authorization_set: project.authorization_entries().contains_key(NO_ENV),
            last_response_path: project.last_called_request_path().map(String::from),
            environments,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseView {
    pub status: u16,
    pub elapsed_ms: u128,
    pub size_bytes: usize,
    pub headers: Vec<KeyValue>,
    pub body: String,
}

impl ResponseView {
    fn from_response(response: GemonResponse, elapsed_ms: u128) -> ResponseView {
        let size_bytes = response.data().len();
        ResponseView {
            status: response.status(),
            elapsed_ms,
            size_bytes,
            headers: KeyValue::from_map(response.headers()),
            body: format_response_body(response.data().as_ref()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairField {
    Key,
    Value,
}

impl PairField {
    fn next(self) -> PairField {
        match self {
            PairField::Key => PairField::Value,
            PairField::Value => PairField::Key,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvField {
    Environment,
    Key,
    Value,
}

impl EnvField {
    fn next(self) -> EnvField {
        match self {
            EnvField::Environment => EnvField::Key,
            EnvField::Key => EnvField::Value,
            EnvField::Value => EnvField::Environment,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Modal {
    ProjectName {
        name: TextInput,
    },
    SaveRequest {
        name: TextInput,
    },
    Header {
        index: Option<usize>,
        key: TextInput,
        value: TextInput,
        active: PairField,
    },
    FormData {
        index: Option<usize>,
        key: TextInput,
        value: TextInput,
        active: PairField,
    },
    EnvValue {
        index: Option<usize>,
        old_key: Option<String>,
        env: TextInput,
        key: TextInput,
        value: TextInput,
        active: EnvField,
    },
    Authorization {
        value: TextInput,
    },
    ConfirmDeleteRequest {
        name: String,
    },
    ConfirmDeleteEnv {
        name: String,
    },
    ConfirmDeleteEnvValue {
        env: String,
        key: String,
    },
}

impl Modal {
    pub fn title(&self) -> &'static str {
        match self {
            Modal::ProjectName { .. } => "Create Gemon Project",
            Modal::SaveRequest { .. } => "Save Request",
            Modal::Header { index, .. } if index.is_some() => "Edit Header",
            Modal::Header { .. } => "Add Header",
            Modal::FormData { index, .. } if index.is_some() => "Edit Form Data",
            Modal::FormData { .. } => "Add Form Data",
            Modal::EnvValue { index, .. } if index.is_some() => "Edit Environment Value",
            Modal::EnvValue { .. } => "Add Environment Value",
            Modal::Authorization { .. } => "Authorization",
            Modal::ConfirmDeleteRequest { .. } => "Delete Request",
            Modal::ConfirmDeleteEnv { .. } => "Delete Environment",
            Modal::ConfirmDeleteEnvValue { .. } => "Delete Environment Value",
        }
    }
}

#[derive(Debug)]
pub struct App {
    pub should_quit: bool,
    pub active_tab: Tab,
    pub focus: Focus,
    pub project: ProjectView,
    pub saved_requests: Vec<SavedRequestInfo>,
    pub selected_request: usize,
    pub request_filter: TextInput,
    pub draft: RequestDraft,
    pub response: Option<ResponseView>,
    pub response_scroll: u16,
    pub selected_env: usize,
    pub selected_env_value: usize,
    pub modal: Option<Modal>,
    pub status: StatusLine,
    pub request_list_width: u16,
    pub environment_list_width: u16,
    pub pair_split_percent: u16,
    pub body_split_percent: u16,
    drag_target: Option<DragTarget>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DragTarget {
    RequestListWidth,
    EnvironmentListWidth,
    PairSplit,
    BodySplit,
}

impl App {
    pub fn new() -> App {
        let mut app = App {
            should_quit: false,
            active_tab: Tab::Requests,
            focus: Focus::SavedRequests,
            project: ProjectView::default(),
            saved_requests: Vec::new(),
            selected_request: 0,
            request_filter: TextInput::single(""),
            draft: RequestDraft::default(),
            response: None,
            response_scroll: 0,
            selected_env: 0,
            selected_env_value: 0,
            modal: None,
            status: StatusLine::info("Ready"),
            request_list_width: 32,
            environment_list_width: 34,
            pair_split_percent: 50,
            body_split_percent: 42,
            drag_target: None,
        };

        app.refresh_workspace();
        if !app.project.exists {
            app.modal = Some(Modal::ProjectName {
                name: TextInput::single(""),
            });
            app.set_info(
                "No gemon.json found. Create a project to save requests and environments.",
            );
        }

        app
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> AppCommand {
        if Self::is_quit_key(key) {
            self.should_quit = true;
            return AppCommand::None;
        }

        if self.handle_numbered_focus_shortcut(key) {
            return AppCommand::None;
        }

        if self.modal.is_some() {
            return self.handle_modal_key(key);
        }

        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return self.handle_control_key(key);
        }

        match key.code {
            KeyCode::F(1) => {
                self.focus_request(Focus::SavedRequests);
                return AppCommand::None;
            }
            KeyCode::F(2) => {
                self.focus_environment(Focus::EnvList);
                return AppCommand::None;
            }
            KeyCode::F(3) => {
                self.active_tab = Tab::Help;
                return AppCommand::None;
            }
            KeyCode::Tab => {
                self.next_focus();
                return AppCommand::None;
            }
            KeyCode::BackTab => {
                self.previous_focus();
                return AppCommand::None;
            }
            KeyCode::Esc => {
                if self.focus == Focus::RequestFilter {
                    if self.request_filter.value().is_empty() {
                        self.focus = Focus::SavedRequests;
                    } else {
                        self.request_filter.set_value(String::new());
                        self.clamp_request_selection();
                    }
                    return AppCommand::None;
                }
                self.active_tab = self.active_tab.previous();
                self.align_focus_to_tab();
                return AppCommand::None;
            }
            _ => {}
        }

        if let Some(input) = self.active_input_mut() {
            if input.handle_key(key) {
                if self.focus == Focus::RequestFilter {
                    self.clamp_request_selection();
                }
                return AppCommand::None;
            }
        }

        match self.active_tab {
            Tab::Requests => self.handle_request_key(key),
            Tab::Environments => self.handle_environment_key(key),
            Tab::Help => {
                match key.code {
                    KeyCode::Left => self.active_tab = self.active_tab.previous(),
                    KeyCode::Right | KeyCode::Enter => self.active_tab = self.active_tab.next(),
                    _ => {}
                }
                AppCommand::None
            }
        }
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent, area: Rect) -> AppCommand {
        if self.modal.is_some() {
            return AppCommand::None;
        }

        let root = layout::root(area);
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if !self.start_drag(mouse.column, mouse.row, root) {
                    self.handle_mouse_click(mouse.column, mouse.row, root);
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                self.update_drag(mouse.column, mouse.row, root);
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.drag_target = None;
            }
            MouseEventKind::ScrollUp => self.handle_mouse_scroll(mouse.column, mouse.row, -1, root),
            MouseEventKind::ScrollDown => {
                self.handle_mouse_scroll(mouse.column, mouse.row, 1, root)
            }
            _ => {}
        }

        AppCommand::None
    }

    fn start_drag(&mut self, column: u16, row: u16, root: layout::RootLayout) -> bool {
        match self.active_tab {
            Tab::Requests => {
                let requests = layout::requests(
                    root.body,
                    self.request_list_width,
                    self.pair_split_percent,
                    self.body_split_percent,
                );
                if on_vertical_boundary(requests.saved, requests.workspace, column, row) {
                    self.drag_target = Some(DragTarget::RequestListWidth);
                    return true;
                }
                if on_vertical_boundary(requests.headers, requests.form_data, column, row) {
                    self.drag_target = Some(DragTarget::PairSplit);
                    return true;
                }
                if on_horizontal_boundary(requests.body, requests.response, column, row) {
                    self.drag_target = Some(DragTarget::BodySplit);
                    return true;
                }
            }
            Tab::Environments => {
                let env = layout::environments(root.body, self.environment_list_width);
                if on_vertical_boundary(env.list, env.values, column, row) {
                    self.drag_target = Some(DragTarget::EnvironmentListWidth);
                    return true;
                }
            }
            Tab::Help => {}
        }

        false
    }

    fn update_drag(&mut self, column: u16, row: u16, root: layout::RootLayout) {
        match self.drag_target {
            Some(DragTarget::RequestListWidth) => {
                let width = column.saturating_sub(root.body.x).saturating_add(1);
                self.request_list_width = layout::clamp_panel_width(width, root.body.width);
            }
            Some(DragTarget::EnvironmentListWidth) => {
                let width = column.saturating_sub(root.body.x).saturating_add(1);
                self.environment_list_width = layout::clamp_panel_width(width, root.body.width);
            }
            Some(DragTarget::PairSplit) => {
                let requests = layout::requests(
                    root.body,
                    self.request_list_width,
                    self.pair_split_percent,
                    self.body_split_percent,
                );
                self.pair_split_percent =
                    percent_at(column, requests.pairs.x, requests.pairs.width).clamp(20, 80);
            }
            Some(DragTarget::BodySplit) => {
                let requests = layout::requests(
                    root.body,
                    self.request_list_width,
                    self.pair_split_percent,
                    self.body_split_percent,
                );
                let top = requests.body.y;
                let height = requests
                    .body
                    .height
                    .saturating_add(requests.response.height)
                    .max(1);
                self.body_split_percent = percent_at(row, top, height).clamp(20, 75);
            }
            None => {}
        }
    }

    fn handle_mouse_click(&mut self, column: u16, row: u16, root: layout::RootLayout) {
        if contains(root.tabs, column, row) {
            self.click_header_tab(column, root.tabs);
            return;
        }

        match self.active_tab {
            Tab::Requests => self.handle_request_click(column, row, root.body),
            Tab::Environments => self.handle_environment_click(column, row, root.body),
            Tab::Help => {}
        }
    }

    fn click_header_tab(&mut self, column: u16, tabs: Rect) {
        let relative = column.saturating_sub(tabs.x);
        let tab_width = (tabs.width / 3).max(1);
        self.active_tab = match (relative / tab_width).min(2) {
            0 => Tab::Requests,
            1 => Tab::Environments,
            _ => Tab::Help,
        };
        self.align_focus_to_tab();
    }

    fn handle_request_click(&mut self, column: u16, row: u16, area: Rect) {
        let requests = layout::requests(
            area,
            self.request_list_width,
            self.pair_split_percent,
            self.body_split_percent,
        );

        if contains(requests.saved_filter, column, row) {
            self.focus_request(Focus::RequestFilter);
        } else if contains(requests.saved_list, column, row) {
            self.focus_request(Focus::SavedRequests);
            if let Some(index) = list_row_at(requests.saved_list, row) {
                self.select_visible_request(index);
            }
        } else if contains(requests.composer, column, row) {
            self.focus_request(composer_focus_at(requests.composer, column, row));
        } else if contains(requests.headers, column, row) {
            self.focus_request(Focus::Headers);
            if let Some(index) = table_row_at(requests.headers, row) {
                self.select_pair_at(true, index);
            }
        } else if contains(requests.form_data, column, row) {
            self.focus_request(Focus::FormData);
            if let Some(index) = table_row_at(requests.form_data, row) {
                self.select_pair_at(false, index);
            }
        } else if contains(requests.body, column, row) {
            self.focus_request(Focus::Body);
        } else if contains(requests.response, column, row) {
            self.focus_request(Focus::Response);
        }
    }

    fn handle_environment_click(&mut self, column: u16, row: u16, area: Rect) {
        let env = layout::environments(area, self.environment_list_width);
        if contains(env.list, column, row) {
            self.focus_environment(Focus::EnvList);
            if let Some(index) = list_row_at(env.list, row) {
                self.select_env_at(index);
            }
        } else if contains(env.values, column, row) {
            self.focus_environment(Focus::EnvValues);
            if let Some(index) = table_row_at(env.values, row) {
                self.select_env_value_at(index);
            }
        }
    }

    fn handle_mouse_scroll(
        &mut self,
        column: u16,
        row: u16,
        delta: isize,
        root: layout::RootLayout,
    ) {
        match self.active_tab {
            Tab::Requests => {
                let requests = layout::requests(
                    root.body,
                    self.request_list_width,
                    self.pair_split_percent,
                    self.body_split_percent,
                );
                if contains(requests.saved, column, row) {
                    self.focus_request(Focus::SavedRequests);
                    self.move_selected_request(delta);
                } else if contains(requests.headers, column, row) {
                    self.focus_request(Focus::Headers);
                    self.move_selected_pair(true, delta);
                } else if contains(requests.form_data, column, row) {
                    self.focus_request(Focus::FormData);
                    self.move_selected_pair(false, delta);
                } else if contains(requests.response, column, row) {
                    self.focus_request(Focus::Response);
                    self.scroll_response(delta);
                }
            }
            Tab::Environments => {
                let env = layout::environments(root.body, self.environment_list_width);
                if contains(env.list, column, row) {
                    self.focus_environment(Focus::EnvList);
                    self.move_selected_env(delta);
                } else if contains(env.values, column, row) {
                    self.focus_environment(Focus::EnvValues);
                    self.move_selected_env_value(delta);
                }
            }
            Tab::Help => {}
        }
    }

    fn is_quit_key(key: KeyEvent) -> bool {
        key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char(character) if matches!(character.to_ascii_lowercase(), 'c' | 'q'))
    }

    fn handle_numbered_focus_shortcut(&mut self, key: KeyEvent) -> bool {
        if !key.modifiers.contains(KeyModifiers::CONTROL) {
            return false;
        }

        match key.code {
            KeyCode::Char('1') => self.focus_request(Focus::SavedRequests),
            KeyCode::Char('2') | KeyCode::Char(' ') | KeyCode::Null => {
                self.focus_request(Focus::Url)
            }
            KeyCode::Char('5') => self.focus_request(Focus::Headers),
            KeyCode::Char('6') => self.focus_request(Focus::FormData),
            KeyCode::Char('7') => self.focus_request(Focus::Body),
            KeyCode::Char('8') => self.focus_request(Focus::Response),
            KeyCode::Char('9') => self.focus_environment(Focus::EnvList),
            KeyCode::Char('0') => self.focus_environment(Focus::EnvValues),
            _ => return false,
        }

        self.modal = None;
        true
    }

    fn focus_request(&mut self, focus: Focus) {
        self.active_tab = Tab::Requests;
        self.focus = focus;
    }

    fn focus_environment(&mut self, focus: Focus) {
        self.active_tab = Tab::Environments;
        self.focus = focus;
    }

    pub async fn send_request(&mut self) {
        if let Err(message) = self.draft.validate_request() {
            self.set_error(message);
            return;
        }

        self.set_info("Sending request...");
        let config = self.draft.to_config(true, self.draft.secure);
        let request = RequestBuilder::build(&config);
        let started = Instant::now();

        match request.execute().await {
            Ok(response) => {
                self.response = Some(ResponseView::from_response(
                    response,
                    started.elapsed().as_millis(),
                ));
                self.response_scroll = 0;
                self.focus = Focus::Response;
                self.set_success("Response received");
            }
            Err(err) => {
                self.set_error(format!("Request failed: {err}"));
            }
        }
    }

    pub fn refresh_workspace(&mut self) {
        self.project = get_project()
            .map(ProjectView::from_project)
            .unwrap_or_default();
        self.saved_requests = if self.project.exists {
            list_saved_requests().unwrap_or_default()
        } else {
            Vec::new()
        };

        self.clamp_request_selection();
        self.clamp_environment_selection();
    }

    fn handle_control_key(&mut self, key: KeyEvent) -> AppCommand {
        match key.code {
            KeyCode::Char('r') if self.active_tab == Tab::Requests => {
                return AppCommand::SendRequest;
            }
            KeyCode::Char('s') if self.active_tab == Tab::Requests => self.save_draft(),
            KeyCode::Char('n') if self.active_tab == Tab::Requests => self.new_draft(),
            KeyCode::Char('d') if self.active_tab == Tab::Requests => {
                self.confirm_delete_selected_request()
            }
            KeyCode::Char('f') if self.active_tab == Tab::Requests => {
                self.focus_request(Focus::RequestFilter)
            }
            KeyCode::Char('o') if self.active_tab == Tab::Requests => self.import_openapi(),
            KeyCode::Char('l') => {
                self.refresh_workspace();
                self.set_success("Workspace reloaded");
            }
            _ => {}
        }
        AppCommand::None
    }

    fn handle_request_key(&mut self, key: KeyEvent) -> AppCommand {
        match self.focus {
            Focus::SavedRequests => match key.code {
                KeyCode::Up => self.move_selected_request(-1),
                KeyCode::Down => self.move_selected_request(1),
                KeyCode::Enter => self.load_selected_request(),
                KeyCode::Char('/') => self.focus = Focus::RequestFilter,
                KeyCode::Char('n') => self.new_draft(),
                KeyCode::Char('x') => self.confirm_delete_selected_request(),
                _ => {}
            },
            Focus::RequestFilter => match key.code {
                KeyCode::Down => self.focus = Focus::SavedRequests,
                KeyCode::Enter => self.load_selected_request(),
                _ => {}
            },
            Focus::Method => match key.code {
                KeyCode::Left | KeyCode::Up => self.draft.method = self.draft.method.previous(),
                KeyCode::Right | KeyCode::Down | KeyCode::Enter | KeyCode::Char(' ') => {
                    self.draft.method = self.draft.method.next()
                }
                _ => {}
            },
            Focus::Secure => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => self.draft.secure = !self.draft.secure,
                _ => {}
            },
            Focus::Headers => self.handle_pair_list_key(key, true),
            Focus::FormData => self.handle_pair_list_key(key, false),
            Focus::Response => self.handle_response_key(key),
            Focus::Url | Focus::RequestName | Focus::Body => {}
            Focus::EnvList | Focus::EnvValues => {}
        }
        AppCommand::None
    }

    fn handle_environment_key(&mut self, key: KeyEvent) -> AppCommand {
        match self.focus {
            Focus::EnvList => match key.code {
                KeyCode::Up => self.move_selected_env(-1),
                KeyCode::Down => self.move_selected_env(1),
                KeyCode::Enter => self.select_current_env(),
                KeyCode::Char('a') => self.open_env_value_modal(None),
                KeyCode::Char('u') => self.open_authorization_modal(),
                KeyCode::Char('x') => self.confirm_delete_current_env(),
                _ => {}
            },
            Focus::EnvValues => match key.code {
                KeyCode::Up => self.move_selected_env_value(-1),
                KeyCode::Down => self.move_selected_env_value(1),
                KeyCode::Char('a') => self.open_env_value_modal(None),
                KeyCode::Enter | KeyCode::Char('e') => {
                    self.open_env_value_modal(Some(self.selected_env_value))
                }
                KeyCode::Char('u') => self.open_authorization_modal(),
                KeyCode::Char('x') => self.confirm_delete_current_env_value(),
                _ => {}
            },
            _ => {}
        }
        AppCommand::None
    }

    fn handle_pair_list_key(&mut self, key: KeyEvent, is_header: bool) {
        match key.code {
            KeyCode::Up => self.move_selected_pair(is_header, -1),
            KeyCode::Down => self.move_selected_pair(is_header, 1),
            KeyCode::Char('a') => self.open_pair_modal(is_header, None),
            KeyCode::Enter | KeyCode::Char('e') => {
                let index = if is_header {
                    self.draft.selected_header
                } else {
                    self.draft.selected_form_data
                };
                self.open_pair_modal(is_header, Some(index));
            }
            KeyCode::Char('x') => self.remove_selected_pair(is_header),
            _ => {}
        }
    }

    fn handle_response_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up => self.scroll_response(-1),
            KeyCode::Down => self.scroll_response(1),
            KeyCode::PageUp => self.scroll_response(-10),
            KeyCode::PageDown => self.scroll_response(10),
            KeyCode::Home => self.response_scroll = 0,
            _ => {}
        }
    }

    fn handle_modal_key(&mut self, key: KeyEvent) -> AppCommand {
        if self.handle_confirmation_key(key) {
            return AppCommand::None;
        }

        match key.code {
            KeyCode::Esc => self.modal = None,
            KeyCode::Tab | KeyCode::BackTab => self.advance_modal_field(),
            KeyCode::Enter => self.submit_modal(),
            _ => self.edit_modal_input(key),
        }
        AppCommand::None
    }

    fn handle_confirmation_key(&mut self, key: KeyEvent) -> bool {
        let confirmation = matches!(
            self.modal,
            Some(Modal::ConfirmDeleteRequest { .. })
                | Some(Modal::ConfirmDeleteEnv { .. })
                | Some(Modal::ConfirmDeleteEnvValue { .. })
        );

        if !confirmation {
            return false;
        }

        match key.code {
            KeyCode::Esc | KeyCode::Char('n') => {
                self.modal = None;
                true
            }
            KeyCode::Enter | KeyCode::Char('y') => {
                self.submit_modal();
                true
            }
            _ => true,
        }
    }

    fn submit_modal(&mut self) {
        let Some(modal) = self.modal.take() else {
            return;
        };

        match modal {
            Modal::ProjectName { name } => {
                let project_name = name.value().trim().to_string();
                if project_name.is_empty() {
                    self.modal = Some(Modal::ProjectName { name });
                    self.set_error("Project name is required");
                    return;
                }
                match create_project(&project_name) {
                    Ok(()) => {
                        self.refresh_workspace();
                        self.set_success(format!("Project '{project_name}' created"));
                    }
                    Err(err) => self.set_error(err.to_string()),
                }
            }
            Modal::SaveRequest { name } => {
                let request_name = name.value().trim().to_string();
                if request_name.is_empty() {
                    self.modal = Some(Modal::SaveRequest { name });
                    self.set_error("Request name is required");
                    return;
                }
                self.draft.name.set_value(request_name);
                self.save_draft();
            }
            Modal::Header {
                index, key, value, ..
            } => self.upsert_draft_pair(true, index, key, value),
            Modal::FormData {
                index, key, value, ..
            } => self.upsert_draft_pair(false, index, key, value),
            Modal::EnvValue {
                index,
                old_key,
                env,
                key,
                value,
                ..
            } => self.upsert_env_value(index, old_key, env, key, value),
            Modal::Authorization { value } => self.save_authorization(value),
            Modal::ConfirmDeleteRequest { name } => self.delete_saved_request(name),
            Modal::ConfirmDeleteEnv { name } => self.delete_environment(name),
            Modal::ConfirmDeleteEnvValue { env, key } => self.delete_env_value(env, key),
        }
    }

    fn edit_modal_input(&mut self, key: KeyEvent) {
        let Some(modal) = self.modal.as_mut() else {
            return;
        };

        match modal {
            Modal::ProjectName { name } | Modal::SaveRequest { name } => {
                name.handle_key(key);
            }
            Modal::Header {
                key: pair_key,
                value,
                active,
                ..
            }
            | Modal::FormData {
                key: pair_key,
                value,
                active,
                ..
            } => match active {
                PairField::Key => {
                    pair_key.handle_key(key);
                }
                PairField::Value => {
                    value.handle_key(key);
                }
            },
            Modal::EnvValue {
                env,
                key: env_key,
                value,
                active,
                ..
            } => match active {
                EnvField::Environment => {
                    env.handle_key(key);
                }
                EnvField::Key => {
                    env_key.handle_key(key);
                }
                EnvField::Value => {
                    value.handle_key(key);
                }
            },
            Modal::Authorization { value } => {
                value.handle_key(key);
            }
            Modal::ConfirmDeleteRequest { .. }
            | Modal::ConfirmDeleteEnv { .. }
            | Modal::ConfirmDeleteEnvValue { .. } => {}
        }
    }

    fn advance_modal_field(&mut self) {
        let Some(modal) = self.modal.as_mut() else {
            return;
        };

        match modal {
            Modal::Header { active, .. } | Modal::FormData { active, .. } => {
                *active = active.next();
            }
            Modal::EnvValue { active, .. } => {
                *active = active.next();
            }
            _ => {}
        }
    }

    fn active_input_mut(&mut self) -> Option<&mut TextInput> {
        match (self.active_tab, self.focus) {
            (Tab::Requests, Focus::RequestFilter) => Some(&mut self.request_filter),
            (Tab::Requests, Focus::Url) => Some(&mut self.draft.url),
            (Tab::Requests, Focus::RequestName) => Some(&mut self.draft.name),
            (Tab::Requests, Focus::Body) => Some(&mut self.draft.body),
            _ => None,
        }
    }

    fn next_focus(&mut self) {
        let order = self.focus_order();
        let index = order
            .iter()
            .position(|focus| *focus == self.focus)
            .unwrap_or_default();
        self.focus = order[(index + 1) % order.len()];
    }

    fn previous_focus(&mut self) {
        let order = self.focus_order();
        let index = order
            .iter()
            .position(|focus| *focus == self.focus)
            .unwrap_or_default();
        self.focus = order[(index + order.len() - 1) % order.len()];
    }

    fn align_focus_to_tab(&mut self) {
        let order = self.focus_order();
        if !order.contains(&self.focus) {
            self.focus = order[0];
        }
    }

    fn focus_order(&self) -> &'static [Focus] {
        match self.active_tab {
            Tab::Requests => &[
                Focus::SavedRequests,
                Focus::RequestFilter,
                Focus::Method,
                Focus::Url,
                Focus::RequestName,
                Focus::Secure,
                Focus::Headers,
                Focus::FormData,
                Focus::Body,
                Focus::Response,
            ],
            Tab::Environments => &[Focus::EnvList, Focus::EnvValues],
            Tab::Help => &[Focus::Response],
        }
    }

    fn move_selected_request(&mut self, delta: isize) {
        let visible = self.filtered_request_indices();
        if visible.is_empty() {
            self.selected_request = 0;
            return;
        }

        let current = visible
            .iter()
            .position(|index| *index == self.selected_request)
            .unwrap_or_default();
        let next = move_index(current, visible.len(), delta);
        self.selected_request = visible[next];
    }

    pub fn visible_saved_requests(&self) -> Vec<(usize, &SavedRequestInfo)> {
        self.filtered_request_indices()
            .into_iter()
            .filter_map(|index| {
                self.saved_requests
                    .get(index)
                    .map(|request| (index, request))
            })
            .collect()
    }

    pub fn selected_visible_request_position(&self) -> Option<usize> {
        self.filtered_request_indices()
            .iter()
            .position(|index| *index == self.selected_request)
    }

    fn filtered_request_indices(&self) -> Vec<usize> {
        let filter = self.request_filter.value().trim().to_ascii_lowercase();
        self.saved_requests
            .iter()
            .enumerate()
            .filter(|(_, request)| {
                filter.is_empty()
                    || request.name.to_ascii_lowercase().contains(&filter)
                    || request.request_type.to_ascii_lowercase().contains(&filter)
            })
            .map(|(index, _)| index)
            .collect()
    }

    fn select_visible_request(&mut self, visible_position: usize) {
        if let Some(index) = self
            .filtered_request_indices()
            .get(visible_position)
            .copied()
        {
            self.selected_request = index;
        }
    }

    fn load_selected_request(&mut self) {
        if self.selected_visible_request_position().is_none() {
            self.set_info("No matching saved request selected");
            return;
        }

        let Some(saved) = self.saved_requests.get(self.selected_request) else {
            self.set_info("No saved request selected");
            return;
        };

        if saved.request_type != "REST" {
            self.set_error("Only REST requests can be edited in the TUI");
            return;
        }

        match read_saved_rest_request(&saved.name) {
            Ok(request) => {
                self.draft = RequestDraft::from_saved(&saved.name, request);
                self.response = None;
                self.focus = Focus::Url;
                self.set_success(format!("Loaded '{}'", saved.name));
            }
            Err(err) => self.set_error(err.to_string()),
        }
    }

    fn new_draft(&mut self) {
        self.draft = RequestDraft::default();
        self.response = None;
        self.response_scroll = 0;
        self.focus = Focus::Url;
        self.set_info("New request draft");
    }

    fn save_draft(&mut self) {
        if !self.ensure_project() {
            return;
        }

        if self.draft.save_name().is_empty() {
            self.modal = Some(Modal::SaveRequest {
                name: TextInput::single(""),
            });
            return;
        }

        if let Err(message) = self.draft.validate_save() {
            self.set_error(message);
            return;
        }

        let name = self.draft.save_name();
        let config = self.draft.to_config(false, false);
        let request = RequestBuilder::build(&config);
        save_request(request, &name);
        self.refresh_workspace();
        self.selected_request = self
            .saved_requests
            .iter()
            .position(|request| request.name == name)
            .unwrap_or(self.selected_request);
        self.set_success(format!("Saved '{name}'"));
    }

    fn confirm_delete_selected_request(&mut self) {
        if self.selected_visible_request_position().is_none() {
            self.set_info("No matching saved request selected");
            return;
        }

        let Some(saved) = self.saved_requests.get(self.selected_request) else {
            self.set_info("No saved request selected");
            return;
        };

        self.modal = Some(Modal::ConfirmDeleteRequest {
            name: saved.name.clone(),
        });
    }

    fn delete_saved_request(&mut self, name: String) {
        match delete_request(&name) {
            Ok(()) => {
                self.refresh_workspace();
                self.set_success(format!("Deleted '{name}'"));
            }
            Err(err) => self.set_error(err.to_string()),
        }
    }

    fn import_openapi(&mut self) {
        if !self.ensure_project() {
            return;
        }

        match import_openapi_project_requests() {
            Ok(report) => {
                self.refresh_workspace();
                if let Some(name) = report.request_names.first() {
                    self.request_filter.set_value(String::new());
                    self.selected_request = self
                        .saved_requests
                        .iter()
                        .position(|request| request.name == *name)
                        .unwrap_or(self.selected_request);
                    self.focus = Focus::SavedRequests;
                }

                if report.requests_imported == 0 {
                    self.set_info(report.summary());
                } else {
                    self.set_success(report.summary());
                }
            }
            Err(err) => self.set_error(err.to_string()),
        }
    }

    fn move_selected_pair(&mut self, is_header: bool, delta: isize) {
        if is_header {
            self.draft.selected_header =
                move_index(self.draft.selected_header, self.draft.headers.len(), delta);
        } else {
            self.draft.selected_form_data = move_index(
                self.draft.selected_form_data,
                self.draft.form_data.len(),
                delta,
            );
        }
    }

    fn select_pair_at(&mut self, is_header: bool, index: usize) {
        if is_header {
            if index < self.draft.headers.len() {
                self.draft.selected_header = index;
            }
        } else if index < self.draft.form_data.len() {
            self.draft.selected_form_data = index;
        }
    }

    fn open_pair_modal(&mut self, is_header: bool, index: Option<usize>) {
        let pair = if is_header {
            index.and_then(|idx| self.draft.headers.get(idx).cloned())
        } else {
            index.and_then(|idx| self.draft.form_data.get(idx).cloned())
        }
        .unwrap_or_default();

        self.modal = if is_header {
            Some(Modal::Header {
                index,
                key: TextInput::single(pair.key),
                value: TextInput::single(pair.value),
                active: PairField::Key,
            })
        } else {
            Some(Modal::FormData {
                index,
                key: TextInput::single(pair.key),
                value: TextInput::single(pair.value),
                active: PairField::Key,
            })
        };
    }

    fn upsert_draft_pair(
        &mut self,
        is_header: bool,
        index: Option<usize>,
        key: TextInput,
        value: TextInput,
    ) {
        let key_value = KeyValue {
            key: key.value().trim().to_string(),
            value: value.value(),
        };

        if key_value.key.is_empty() {
            self.set_error("Key is required");
            return;
        }

        let (pairs, selected) = if is_header {
            (&mut self.draft.headers, &mut self.draft.selected_header)
        } else {
            (
                &mut self.draft.form_data,
                &mut self.draft.selected_form_data,
            )
        };

        match index {
            Some(index) if index < pairs.len() => {
                pairs[index] = key_value;
                *selected = index;
            }
            _ => {
                pairs.push(key_value);
                *selected = pairs.len().saturating_sub(1);
            }
        }

        self.focus = if is_header {
            Focus::Headers
        } else {
            Focus::FormData
        };
        self.set_success("Request value updated");
    }

    fn remove_selected_pair(&mut self, is_header: bool) {
        let (pairs, selected) = if is_header {
            (&mut self.draft.headers, &mut self.draft.selected_header)
        } else {
            (
                &mut self.draft.form_data,
                &mut self.draft.selected_form_data,
            )
        };

        if pairs.is_empty() {
            return;
        }

        let index = (*selected).min(pairs.len() - 1);
        pairs.remove(index);
        *selected = (*selected).min(pairs.len().saturating_sub(1));
        self.set_success("Request value removed");
    }

    fn move_selected_env(&mut self, delta: isize) {
        self.selected_env = move_index(self.selected_env, self.project.environments.len(), delta);
        self.selected_env_value = 0;
    }

    fn select_env_at(&mut self, index: usize) {
        if index < self.project.environments.len() {
            self.selected_env = index;
            self.selected_env_value = 0;
        }
    }

    fn move_selected_env_value(&mut self, delta: isize) {
        let len = self
            .project
            .environments
            .get(self.selected_env)
            .map(|env| env.values.len())
            .unwrap_or_default();
        self.selected_env_value = move_index(self.selected_env_value, len, delta);
    }

    fn select_env_value_at(&mut self, index: usize) {
        if index < self.current_env_value_count() {
            self.selected_env_value = index;
        }
    }

    fn scroll_response(&mut self, delta: isize) {
        if delta.is_negative() {
            self.response_scroll = self
                .response_scroll
                .saturating_sub(delta.unsigned_abs() as u16);
        } else {
            self.response_scroll = self.response_scroll.saturating_add(delta as u16);
        }
    }

    fn select_current_env(&mut self) {
        let Some(env) = self.project.environments.get(self.selected_env) else {
            self.set_info("No environment selected");
            return;
        };
        let env_name = env.name.clone();

        match set_selected_env(&env_name) {
            Ok(()) => {
                self.refresh_workspace();
                self.set_success(format!("Selected environment '{env_name}'"));
            }
            Err(err) => self.set_error(err.to_string()),
        }
    }

    fn open_env_value_modal(&mut self, index: Option<usize>) {
        if !self.ensure_project() {
            return;
        }

        let env = self.project.environments.get(self.selected_env);
        let pair = env
            .and_then(|env| index.and_then(|idx| env.values.get(idx).cloned()))
            .unwrap_or_default();

        self.modal = Some(Modal::EnvValue {
            index,
            old_key: if pair.key.is_empty() {
                None
            } else {
                Some(pair.key.clone())
            },
            env: TextInput::single(env.map(|env| env.name.clone()).unwrap_or_default()),
            key: TextInput::single(pair.key),
            value: TextInput::single(pair.value),
            active: if env.is_some() {
                EnvField::Key
            } else {
                EnvField::Environment
            },
        });
    }

    fn upsert_env_value(
        &mut self,
        _index: Option<usize>,
        old_key: Option<String>,
        env: TextInput,
        key: TextInput,
        value: TextInput,
    ) {
        let env_name = env.value().trim().to_string();
        let key_name = key.value().trim().to_string();

        if env_name.is_empty() || key_name.is_empty() {
            self.set_error("Environment and key are required");
            return;
        }

        if let Some(old_key) = old_key {
            if old_key != key_name {
                let _ = remove_env_value(&env_name, &old_key);
            }
        }

        match add_env_value(&env_name, (key_name.clone(), value.value())) {
            Ok(()) => {
                self.refresh_workspace();
                self.selected_env = self
                    .project
                    .environments
                    .iter()
                    .position(|env| env.name == env_name)
                    .unwrap_or(self.selected_env);
                self.selected_env_value = self
                    .project
                    .environments
                    .get(self.selected_env)
                    .and_then(|env| env.values.iter().position(|pair| pair.key == key_name))
                    .unwrap_or_default();
                self.focus = Focus::EnvValues;
                self.set_success("Environment value saved");
            }
            Err(err) => self.set_error(err.to_string()),
        }
    }

    fn confirm_delete_current_env(&mut self) {
        let Some(env) = self.project.environments.get(self.selected_env) else {
            self.set_info("No environment selected");
            return;
        };

        self.modal = Some(Modal::ConfirmDeleteEnv {
            name: env.name.clone(),
        });
    }

    fn delete_environment(&mut self, name: String) {
        match remove_env(&name) {
            Ok(()) => {
                self.refresh_workspace();
                self.set_success(format!("Deleted environment '{name}'"));
            }
            Err(err) => self.set_error(err.to_string()),
        }
    }

    fn confirm_delete_current_env_value(&mut self) {
        let Some(env) = self.project.environments.get(self.selected_env) else {
            self.set_info("No environment selected");
            return;
        };
        let Some(pair) = env.values.get(self.selected_env_value) else {
            self.set_info("No environment value selected");
            return;
        };

        self.modal = Some(Modal::ConfirmDeleteEnvValue {
            env: env.name.clone(),
            key: pair.key.clone(),
        });
    }

    fn delete_env_value(&mut self, env: String, key: String) {
        match remove_env_value(&env, &key) {
            Ok(()) => {
                self.refresh_workspace();
                self.set_success(format!("Deleted '{key}' from '{env}'"));
            }
            Err(err) => self.set_error(err.to_string()),
        }
    }

    fn open_authorization_modal(&mut self) {
        if !self.ensure_project() {
            return;
        }

        self.modal = Some(Modal::Authorization {
            value: TextInput::single(""),
        });
    }

    fn save_authorization(&mut self, value: TextInput) {
        let authorization = value.value();
        let result = if authorization.trim().is_empty() {
            remove_authorization()
        } else {
            add_authorization(&authorization)
        };

        match result {
            Ok(()) => {
                self.refresh_workspace();
                if authorization.trim().is_empty() {
                    self.set_success("Authorization removed");
                } else {
                    self.set_success("Authorization saved");
                }
            }
            Err(err) => self.set_error(err.to_string()),
        }
    }

    fn ensure_project(&mut self) -> bool {
        if self.project.exists {
            return true;
        }

        self.modal = Some(Modal::ProjectName {
            name: TextInput::single(""),
        });
        self.set_error("Create a project before using this action");
        false
    }

    fn clamp_request_selection(&mut self) {
        if self.saved_requests.is_empty() {
            self.selected_request = 0;
            return;
        }

        self.selected_request = self
            .selected_request
            .min(self.saved_requests.len().saturating_sub(1));

        let visible = self.filtered_request_indices();
        if !visible.is_empty() && !visible.contains(&self.selected_request) {
            self.selected_request = visible[0];
        }
    }

    fn clamp_environment_selection(&mut self) {
        self.selected_env = self
            .selected_env
            .min(self.project.environments.len().saturating_sub(1));
        self.selected_env_value = self
            .selected_env_value
            .min(self.current_env_value_count().saturating_sub(1));
    }

    fn current_env_value_count(&self) -> usize {
        self.project
            .environments
            .get(self.selected_env)
            .map(|env| env.values.len())
            .unwrap_or_default()
    }

    fn set_info(&mut self, message: impl Into<String>) {
        self.status = StatusLine {
            message: message.into(),
            kind: StatusKind::Info,
        };
    }

    fn set_success(&mut self, message: impl Into<String>) {
        self.status = StatusLine {
            message: message.into(),
            kind: StatusKind::Success,
        };
    }

    fn set_error(&mut self, message: impl Into<String>) {
        self.status = StatusLine {
            message: message.into(),
            kind: StatusKind::Error,
        };
    }
}

fn move_index(current: usize, len: usize, delta: isize) -> usize {
    if len == 0 {
        return 0;
    }

    let len = len as isize;
    let next = (current as isize + delta).rem_euclid(len);
    next as usize
}

fn list_row_at(area: Rect, row: u16) -> Option<usize> {
    row.checked_sub(area.y.saturating_add(1))
        .map(usize::from)
        .filter(|index| *index < area.height.saturating_sub(2) as usize)
}

fn table_row_at(area: Rect, row: u16) -> Option<usize> {
    row.checked_sub(area.y.saturating_add(2))
        .map(usize::from)
        .filter(|index| *index < area.height.saturating_sub(3) as usize)
}

fn composer_focus_at(area: Rect, column: u16, row: u16) -> Focus {
    match row.saturating_sub(area.y.saturating_add(1)) {
        0 if column < area.x.saturating_add(20) => Focus::Method,
        0 => Focus::Secure,
        1 => Focus::Url,
        2 => Focus::RequestName,
        _ => Focus::Url,
    }
}

fn on_vertical_boundary(left: Rect, right: Rect, column: u16, row: u16) -> bool {
    let boundary_left = left.x.saturating_add(left.width).saturating_sub(1);
    let boundary_right = right.x;
    row >= left.y.min(right.y)
        && row
            < left
                .y
                .saturating_add(left.height)
                .max(right.y.saturating_add(right.height))
        && (column == boundary_left || column == boundary_right)
}

fn on_horizontal_boundary(top: Rect, bottom: Rect, column: u16, row: u16) -> bool {
    let boundary_top = top.y.saturating_add(top.height).saturating_sub(1);
    let boundary_bottom = bottom.y;
    column >= top.x.min(bottom.x)
        && column
            < top
                .x
                .saturating_add(top.width)
                .max(bottom.x.saturating_add(bottom.width))
        && (row == boundary_top || row == boundary_bottom)
}

fn percent_at(position: u16, origin: u16, length: u16) -> u16 {
    if length == 0 {
        return 50;
    }

    let relative = position.saturating_sub(origin).min(length);
    ((u32::from(relative) * 100) / u32::from(length)) as u16
}

fn format_response_body(bytes: &[u8]) -> String {
    match serde_json::from_slice::<Value>(bytes) {
        Ok(value) => serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string()),
        Err(_) => String::from_utf8_lossy(bytes).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        layout, move_index, App, Focus, Modal, RequestDraft, ResponseView, Tab, TextInput,
    };
    use crate::project::project_handler::SavedRequestInfo;
    use crossterm::event::{
        KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    };
    use ratatui::layout::Rect;

    fn ctrl_key(character: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(character), KeyModifiers::CONTROL)
    }

    fn ctrl_code(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    fn function_key(number: u8) -> KeyEvent {
        KeyEvent::new(KeyCode::F(number), KeyModifiers::NONE)
    }

    fn plain_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn mouse(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::NONE,
        }
    }

    fn saved(name: &str) -> SavedRequestInfo {
        SavedRequestInfo {
            name: name.to_string(),
            request_type: String::from("REST"),
        }
    }

    #[test]
    fn move_index_wraps_around_lists() {
        assert_eq!(move_index(0, 3, -1), 2);
        assert_eq!(move_index(2, 3, 1), 0);
        assert_eq!(move_index(0, 0, 1), 0);
    }

    #[test]
    fn request_validation_requires_uri() {
        let draft = RequestDraft::default();

        assert!(draft.validate_request().is_err());
    }

    #[test]
    fn request_filter_limits_visible_requests_and_selection() {
        let mut app = App::new();
        app.modal = None;
        app.saved_requests = vec![saved("listPets"), saved("createPet"), saved("healthCheck")];
        app.selected_request = 0;

        app.request_filter.set_value(String::from("create"));
        app.clamp_request_selection();

        let visible = app.visible_saved_requests();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].1.name, "createPet");
        assert_eq!(app.selected_request, 1);

        app.move_selected_request(1);
        assert_eq!(app.selected_request, 1);
    }

    #[test]
    fn slash_focuses_filter_and_escape_clears_it() {
        let mut app = App::new();
        app.modal = None;
        app.focus = Focus::SavedRequests;
        app.request_filter.set_value(String::from("pet"));

        app.handle_key(plain_key(KeyCode::Char('/')));
        assert_eq!(app.focus, Focus::RequestFilter);

        app.handle_key(plain_key(KeyCode::Esc));
        assert_eq!(app.focus, Focus::RequestFilter);
        assert!(app.request_filter.value().is_empty());

        app.handle_key(plain_key(KeyCode::Esc));
        assert_eq!(app.focus, Focus::SavedRequests);
    }

    #[test]
    fn function_keys_follow_header_order() {
        let mut app = App::new();
        app.modal = None;

        app.handle_key(function_key(1));
        assert_eq!(app.active_tab, Tab::Requests);
        assert_eq!(app.focus, Focus::SavedRequests);

        app.handle_key(function_key(2));
        assert_eq!(app.active_tab, Tab::Environments);
        assert_eq!(app.focus, Focus::EnvList);

        app.handle_key(function_key(3));
        assert_eq!(app.active_tab, Tab::Help);
    }

    #[test]
    fn ctrl_number_shortcuts_focus_sections() {
        let mut app = App::new();
        app.modal = None;

        app.handle_key(ctrl_key('5'));
        assert_eq!(app.active_tab, Tab::Requests);
        assert_eq!(app.focus, Focus::Headers);

        app.handle_key(ctrl_key('0'));
        assert_eq!(app.active_tab, Tab::Environments);
        assert_eq!(app.focus, Focus::EnvValues);

        app.handle_key(ctrl_code(KeyCode::Char(' ')));
        assert_eq!(app.active_tab, Tab::Requests);
        assert_eq!(app.focus, Focus::Url);
    }

    #[test]
    fn ctrl_three_and_four_are_unbound_for_tmux() {
        let mut app = App::new();
        app.modal = None;
        app.focus = Focus::Url;

        app.handle_key(ctrl_key('3'));
        assert_eq!(app.focus, Focus::Url);

        app.handle_key(ctrl_key('4'));
        assert_eq!(app.focus, Focus::Url);
    }

    #[test]
    fn ctrl_number_shortcuts_do_not_edit_focused_text_fields() {
        let mut app = App::new();
        app.modal = None;
        app.focus = Focus::Url;
        app.draft.url.set_value(String::from("https://api.test"));

        app.handle_key(ctrl_key('5'));

        assert_eq!(app.active_tab, Tab::Requests);
        assert_eq!(app.focus, Focus::Headers);
        assert_eq!(app.draft.url.value(), "https://api.test");
    }

    #[test]
    fn ctrl_number_shortcuts_work_when_modal_is_open() {
        let mut app = App::new();
        app.modal = Some(Modal::SaveRequest {
            name: TextInput::single("draft"),
        });

        app.handle_key(ctrl_key('0'));

        assert_eq!(app.active_tab, Tab::Environments);
        assert_eq!(app.focus, Focus::EnvValues);
        assert!(app.modal.is_none());
    }

    #[test]
    fn ctrl_c_quits_even_when_modal_is_open() {
        let mut app = App::new();

        app.handle_key(ctrl_key('c'));

        assert!(app.should_quit);
    }

    #[test]
    fn mouse_click_selects_visible_saved_request() {
        let mut app = App::new();
        app.modal = None;
        app.saved_requests = vec![saved("first"), saved("second"), saved("third")];
        let area = Rect::new(0, 0, 100, 40);
        let root = layout::root(area);
        let requests = layout::requests(
            root.body,
            app.request_list_width,
            app.pair_split_percent,
            app.body_split_percent,
        );

        app.handle_mouse(
            mouse(
                MouseEventKind::Down(MouseButton::Left),
                requests.saved_list.x + 2,
                requests.saved_list.y + 2,
            ),
            area,
        );

        assert_eq!(app.focus, Focus::SavedRequests);
        assert_eq!(app.selected_request, 1);
    }

    #[test]
    fn mouse_wheel_scrolls_saved_request_list() {
        let mut app = App::new();
        app.modal = None;
        app.saved_requests = vec![saved("first"), saved("second"), saved("third")];
        app.selected_request = 0;
        let area = Rect::new(0, 0, 100, 40);
        let root = layout::root(area);
        let requests = layout::requests(
            root.body,
            app.request_list_width,
            app.pair_split_percent,
            app.body_split_percent,
        );

        app.handle_mouse(
            mouse(
                MouseEventKind::ScrollDown,
                requests.saved_list.x + 2,
                requests.saved_list.y + 1,
            ),
            area,
        );

        assert_eq!(app.focus, Focus::SavedRequests);
        assert_eq!(app.selected_request, 1);
    }

    #[test]
    fn mouse_click_focuses_composer_url_field() {
        let mut app = App::new();
        app.modal = None;
        let area = Rect::new(0, 0, 100, 40);
        let root = layout::root(area);
        let requests = layout::requests(
            root.body,
            app.request_list_width,
            app.pair_split_percent,
            app.body_split_percent,
        );

        app.handle_mouse(
            mouse(
                MouseEventKind::Down(MouseButton::Left),
                requests.composer.x + 8,
                requests.composer.y + 2,
            ),
            area,
        );

        assert_eq!(app.focus, Focus::Url);
    }

    #[test]
    fn mouse_click_navigates_header_tabs() {
        let mut app = App::new();
        app.modal = None;
        let area = Rect::new(0, 0, 120, 40);
        let root = layout::root(area);

        app.handle_mouse(
            mouse(
                MouseEventKind::Down(MouseButton::Left),
                root.tabs.x + (root.tabs.width / 3) + 1,
                root.tabs.y + 1,
            ),
            area,
        );

        assert_eq!(app.active_tab, Tab::Environments);
        assert_eq!(app.focus, Focus::EnvList);
    }

    #[test]
    fn mouse_wheel_scrolls_response_under_pointer() {
        let mut app = App::new();
        app.modal = None;
        app.response = Some(ResponseView {
            status: 200,
            elapsed_ms: 10,
            size_bytes: 2,
            headers: Vec::new(),
            body: String::from("one\ntwo\nthree"),
        });
        let area = Rect::new(0, 0, 100, 40);
        let root = layout::root(area);
        let requests = layout::requests(
            root.body,
            app.request_list_width,
            app.pair_split_percent,
            app.body_split_percent,
        );

        app.handle_mouse(
            mouse(
                MouseEventKind::ScrollDown,
                requests.response.x + 2,
                requests.response.y + 1,
            ),
            area,
        );

        assert_eq!(app.focus, Focus::Response);
        assert_eq!(app.response_scroll, 1);
    }

    #[test]
    fn mouse_drag_resizes_request_list() {
        let mut app = App::new();
        app.modal = None;
        let area = Rect::new(0, 0, 120, 40);
        let root = layout::root(area);
        let requests = layout::requests(
            root.body,
            app.request_list_width,
            app.pair_split_percent,
            app.body_split_percent,
        );
        let original_width = app.request_list_width;

        app.handle_mouse(
            mouse(
                MouseEventKind::Down(MouseButton::Left),
                requests.saved.x + requests.saved.width - 1,
                requests.saved.y + 4,
            ),
            area,
        );
        app.handle_mouse(
            mouse(
                MouseEventKind::Drag(MouseButton::Left),
                requests.saved.x + requests.saved.width + 10,
                requests.saved.y + 4,
            ),
            area,
        );
        app.handle_mouse(
            mouse(
                MouseEventKind::Up(MouseButton::Left),
                requests.saved.x + requests.saved.width + 10,
                requests.saved.y + 4,
            ),
            area,
        );

        assert!(app.request_list_width > original_width);
        assert!(app.drag_target.is_none());
    }

    #[test]
    fn mouse_drag_resizes_body_response_split() {
        let mut app = App::new();
        app.modal = None;
        let area = Rect::new(0, 0, 120, 40);
        let root = layout::root(area);
        let requests = layout::requests(
            root.body,
            app.request_list_width,
            app.pair_split_percent,
            app.body_split_percent,
        );
        let original_percent = app.body_split_percent;
        let boundary_row = requests.response.y;

        app.handle_mouse(
            mouse(
                MouseEventKind::Down(MouseButton::Left),
                requests.body.x + 4,
                boundary_row,
            ),
            area,
        );
        app.handle_mouse(
            mouse(
                MouseEventKind::Drag(MouseButton::Left),
                requests.body.x + 4,
                requests.body.y,
            ),
            area,
        );

        assert_ne!(app.body_split_percent, original_percent);
    }
}
