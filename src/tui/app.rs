use super::input::TextInput;
use crate::{
    config::{effector::Effector, types::GemonMethodType, GemonConfig},
    constants::NO_ENV,
    project::{
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
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
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
    pub draft: RequestDraft,
    pub response: Option<ResponseView>,
    pub response_scroll: u16,
    pub selected_env: usize,
    pub selected_env_value: usize,
    pub modal: Option<Modal>,
    pub status: StatusLine,
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
            draft: RequestDraft::default(),
            response: None,
            response_scroll: 0,
            selected_env: 0,
            selected_env_value: 0,
            modal: None,
            status: StatusLine::info("Ready"),
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
                self.active_tab = self.active_tab.previous();
                self.align_focus_to_tab();
                return AppCommand::None;
            }
            _ => {}
        }

        if let Some(input) = self.active_input_mut() {
            if input.handle_key(key) {
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
                KeyCode::Char('n') => self.new_draft(),
                KeyCode::Char('x') => self.confirm_delete_selected_request(),
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
            KeyCode::Up => self.response_scroll = self.response_scroll.saturating_sub(1),
            KeyCode::Down => self.response_scroll = self.response_scroll.saturating_add(1),
            KeyCode::PageUp => self.response_scroll = self.response_scroll.saturating_sub(10),
            KeyCode::PageDown => self.response_scroll = self.response_scroll.saturating_add(10),
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
        self.selected_request = move_index(self.selected_request, self.saved_requests.len(), delta);
    }

    fn load_selected_request(&mut self) {
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

    fn move_selected_env_value(&mut self, delta: isize) {
        let len = self
            .project
            .environments
            .get(self.selected_env)
            .map(|env| env.values.len())
            .unwrap_or_default();
        self.selected_env_value = move_index(self.selected_env_value, len, delta);
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
        self.selected_request = self
            .selected_request
            .min(self.saved_requests.len().saturating_sub(1));
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

fn format_response_body(bytes: &[u8]) -> String {
    match serde_json::from_slice::<Value>(bytes) {
        Ok(value) => serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string()),
        Err(_) => String::from_utf8_lossy(bytes).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{move_index, App, Focus, Modal, RequestDraft, Tab, TextInput};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn ctrl_key(character: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(character), KeyModifiers::CONTROL)
    }

    fn ctrl_code(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    fn function_key(number: u8) -> KeyEvent {
        KeyEvent::new(KeyCode::F(number), KeyModifiers::NONE)
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
}
