use super::{
    actions::Action,
    input::TextInput,
    viewer::{self, ContentKind, Match},
};
use crate::{
    config::types::GemonMethodType,
    constants::{NO_ENV, PROJECT_ROOT_FILE},
    project::{
        project_handler::{list_saved_requests, try_get_project, SavedRequestInfo},
        Project,
    },
    request::{
        request_builder::GemonResponse,
        rest_request::{GemonRestRequest, GemonRestRequestBuilder},
    },
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::Value;
use std::{
    cell::Cell,
    collections::{BTreeSet, HashMap},
    fs,
    time::{Duration, Instant, SystemTime},
};

mod environments;
mod mouse;
mod overlay;
mod requests;
#[cfg(test)]
mod tests;

pub use environments::EnvRow;
pub use mouse::BRAND;
pub use overlay::{
    Confirm, ConfirmKind, Overlay, Palette, PendingAction, Picker, PickerItem, PickerKind,
    Prompt, PromptKind, TextView,
};

const DEFAULT_SIDEBAR_WIDTH: u16 = 30;
const DEFAULT_RESPONSE_PERCENT: u16 = 55;

/// Work the event loop performs on behalf of the app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppCommand {
    None,
    Send(GemonRestRequest),
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Requests,
    Environments,
}

impl Screen {
    pub const ALL: [Screen; 2] = [Screen::Requests, Screen::Environments];

    pub fn title(self) -> &'static str {
        match self {
            Screen::Requests => "Requests",
            Screen::Environments => "Environments",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Screen::Requests => "F1",
            Screen::Environments => "F2",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Sidebar,
    Method,
    Url,
    Headers,
    Body,
    Form,
    Auth,
    Response,
    EnvList,
    EnvVars,
}

impl Focus {
    fn screen(self) -> Screen {
        match self {
            Focus::EnvList | Focus::EnvVars => Screen::Environments,
            _ => Screen::Requests,
        }
    }
}

const REQUEST_FOCUS_ORDER: [Focus; 8] = [
    Focus::Sidebar,
    Focus::Method,
    Focus::Url,
    Focus::Headers,
    Focus::Body,
    Focus::Form,
    Focus::Auth,
    Focus::Response,
];
const ENVIRONMENT_FOCUS_ORDER: [Focus; 2] = [Focus::EnvList, Focus::EnvVars];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestTab {
    Headers,
    Body,
    Form,
    Auth,
}

impl RequestTab {
    pub const ALL: [RequestTab; 4] = [
        RequestTab::Headers,
        RequestTab::Body,
        RequestTab::Form,
        RequestTab::Auth,
    ];

    pub fn focus(self) -> Focus {
        match self {
            RequestTab::Headers => Focus::Headers,
            RequestTab::Body => Focus::Body,
            RequestTab::Form => Focus::Form,
            RequestTab::Auth => Focus::Auth,
        }
    }

    fn from_focus(focus: Focus) -> Option<RequestTab> {
        RequestTab::ALL.into_iter().find(|tab| tab.focus() == focus)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseTab {
    Body,
    Headers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusLine {
    pub message: String,
    pub kind: StatusKind,
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

/// The request being edited. Pair selections live here so loading a request resets them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestDraft {
    pub method: GemonMethodType,
    pub url: TextInput,
    pub secure: bool,
    pub headers: Vec<KeyValue>,
    pub selected_header: usize,
    pub form: Vec<KeyValue>,
    pub selected_form: usize,
    pub body: TextInput,
}

impl Default for RequestDraft {
    fn default() -> Self {
        RequestDraft {
            method: GemonMethodType::Get,
            url: TextInput::single(""),
            secure: false,
            headers: Vec::new(),
            selected_header: 0,
            form: Vec::new(),
            selected_form: 0,
            body: TextInput::multiline(""),
        }
    }
}

impl RequestDraft {
    pub fn from_saved(request: &GemonRestRequest) -> RequestDraft {
        RequestDraft {
            method: request.method(),
            url: TextInput::single(request.uri()),
            secure: request.secure(),
            headers: KeyValue::from_map(request.headers()),
            selected_header: 0,
            form: KeyValue::from_map(request.form_data()),
            selected_form: 0,
            body: TextInput::multiline(request.body().unwrap_or_default()),
        }
    }

    /// Equality of everything that would be saved, ignoring cursor and selection state.
    pub fn same_content(&self, other: &RequestDraft) -> bool {
        self.method == other.method
            && self.url.value() == other.url.value()
            && self.secure == other.secure
            && self.headers == other.headers
            && self.form == other.form
            && self.body_text() == other.body_text()
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.url.value().trim().is_empty() {
            return Err(String::from("Enter a URL first"));
        }
        if self
            .headers
            .iter()
            .chain(self.form.iter())
            .any(|pair| pair.key.trim().is_empty())
        {
            return Err(String::from("Every header and form field needs a name"));
        }
        Ok(())
    }

    pub fn body_text(&self) -> Option<String> {
        let body = self.body.value();
        if body.trim().is_empty() {
            None
        } else {
            Some(body)
        }
    }

    pub fn pairs(&self, target: PairTarget) -> &[KeyValue] {
        match target {
            PairTarget::Header => &self.headers,
            PairTarget::Form => &self.form,
        }
    }

    pub fn selected_pair(&self, target: PairTarget) -> usize {
        match target {
            PairTarget::Header => self.selected_header,
            PairTarget::Form => self.selected_form,
        }
    }

    fn pairs_mut(&mut self, target: PairTarget) -> (&mut Vec<KeyValue>, &mut usize) {
        match target {
            PairTarget::Header => (&mut self.headers, &mut self.selected_header),
            PairTarget::Form => (&mut self.form, &mut self.selected_form),
        }
    }

    /// The request exactly as saved: placeholders stay unresolved.
    pub fn to_saved_request(&self) -> GemonRestRequest {
        self.build(&HashMap::new())
    }

    /// The request as sent: environment placeholders are resolved.
    pub fn to_request(&self, env: &HashMap<String, String>) -> GemonRestRequest {
        self.build(env)
    }

    fn build(&self, env: &HashMap<String, String>) -> GemonRestRequest {
        let pairs = |pairs: &[KeyValue]| {
            pairs
                .iter()
                .map(|pair| {
                    (
                        substitute(pair.key.trim(), env),
                        substitute(&pair.value, env),
                    )
                })
                .collect::<HashMap<_, _>>()
        };
        GemonRestRequestBuilder::new()
            .set_gemon_method_type(self.method)
            .set_url(substitute(self.url.value().trim(), env))
            .set_headers(&pairs(&self.headers))
            .set_body(self.body_text().map(|body| substitute(&body, env)))
            .set_form_data(&pairs(&self.form))
            .set_secure(self.secure)
            .build()
    }

    /// Placeholders such as `{base_uri}` that `env` does not define.
    pub fn unresolved_placeholders(&self, env: &HashMap<String, String>) -> Vec<String> {
        let mut texts = vec![self.url.value()];
        texts.extend(self.body_text());
        for pair in self.headers.iter().chain(self.form.iter()) {
            texts.push(pair.key.clone());
            texts.push(pair.value.clone());
        }
        texts
            .iter()
            .flat_map(|text| placeholders(text))
            .filter(|name| !env.contains_key(name))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairTarget {
    Header,
    Form,
}

impl PairTarget {
    pub fn noun(self) -> &'static str {
        match self {
            PairTarget::Header => "header",
            PairTarget::Form => "form field",
        }
    }
}

/// Replaces `{name}` with the environment value, as the CLI does.
pub fn substitute(text: &str, env: &HashMap<String, String>) -> String {
    env.iter().fold(text.to_string(), |text, (key, value)| {
        text.replace(&format!("{{{key}}}"), value)
    })
}

fn placeholders(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find('{') {
        let after = &rest[start + 1..];
        match after.find('}') {
            Some(end) => {
                let name = &after[..end];
                let is_name = !name.is_empty()
                    && name
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
                    && name.chars().any(|c| c.is_ascii_alphabetic());
                if is_name {
                    names.push(name.to_string());
                    rest = &after[end + 1..];
                } else {
                    rest = after;
                }
            }
            None => break,
        }
    }
    names
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnvironmentView {
    pub name: String,
    pub values: Vec<KeyValue>,
    pub authorization: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectView {
    pub exists: bool,
    pub name: Option<String>,
    pub selected_environment: Option<String>,
    pub default_authorization: Option<String>,
    pub last_response_path: Option<String>,
    pub environments: Vec<EnvironmentView>,
}

impl ProjectView {
    fn from_project(project: Project) -> ProjectView {
        let authorization = project.authorization_entries();
        let mut environments = project
            .environments()
            .iter()
            .map(|(name, environment)| EnvironmentView {
                name: name.clone(),
                values: KeyValue::from_map(environment.values_ref()),
                authorization: authorization.get(name).cloned(),
            })
            .collect::<Vec<_>>();
        environments.sort_by(|left, right| left.name.cmp(&right.name));

        ProjectView {
            exists: true,
            name: Some(project.name().to_string()),
            selected_environment: project.selected_environment_name().map(String::from),
            default_authorization: authorization.get(NO_ENV).cloned(),
            last_response_path: project.last_called_request_path().map(String::from),
            environments,
        }
    }

    pub fn environment(&self, name: &str) -> Option<&EnvironmentView> {
        self.environments.iter().find(|env| env.name == name)
    }

    pub fn active_environment(&self) -> Option<&EnvironmentView> {
        self.selected_environment
            .as_deref()
            .and_then(|name| self.environment(name))
    }

    pub fn active_values(&self) -> HashMap<String, String> {
        self.active_environment()
            .map(|env| {
                env.values
                    .iter()
                    .map(|pair| (pair.key.clone(), pair.value.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Authorization sent with secure requests: the active environment's, or the default.
    pub fn active_authorization(&self) -> Option<&str> {
        match self.selected_environment.as_deref() {
            Some(_) => self
                .active_environment()
                .and_then(|env| env.authorization.as_deref()),
            None => self.default_authorization.as_deref(),
        }
    }

    pub fn active_environment_label(&self) -> &str {
        self.selected_environment
            .as_deref()
            .unwrap_or("no environment")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponseState {
    Empty,
    Loading { started: Instant, summary: String },
    Ready(ResponseView),
    Failed { summary: String, message: String },
    Cancelled { summary: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseView {
    pub status: Option<u16>,
    pub elapsed: Option<Duration>,
    pub size_bytes: usize,
    pub headers: Vec<KeyValue>,
    pub body: String,
    pub body_lines: Vec<String>,
    pub header_lines: Vec<String>,
    pub kind: ContentKind,
    pub origin: String,
}

impl ResponseView {
    fn live(response: GemonResponse, elapsed: Duration, origin: String) -> ResponseView {
        let headers = KeyValue::from_map(response.headers());
        let (body, kind) = format_body(response.data().as_ref());
        ResponseView::new(
            Some(response.status()),
            Some(elapsed),
            response.data().len(),
            headers,
            body,
            kind,
            origin,
        )
    }

    fn from_file(path: &str, contents: &[u8]) -> ResponseView {
        let (body, kind) = format_body(contents);
        ResponseView::new(
            None,
            None,
            contents.len(),
            Vec::new(),
            body,
            kind,
            format!("file {path}"),
        )
    }

    fn new(
        status: Option<u16>,
        elapsed: Option<Duration>,
        size_bytes: usize,
        headers: Vec<KeyValue>,
        body: String,
        kind: ContentKind,
        origin: String,
    ) -> ResponseView {
        let header_lines = headers
            .iter()
            .map(|pair| format!("{}: {}", pair.key, pair.value))
            .flat_map(|line| viewer::sanitize(&line))
            .collect();
        ResponseView {
            status,
            elapsed,
            size_bytes,
            body_lines: viewer::sanitize(&body),
            header_lines,
            headers,
            body,
            kind,
            origin,
        }
    }

    pub fn lines(&self, tab: ResponseTab) -> &[String] {
        match tab {
            ResponseTab::Body => &self.body_lines,
            ResponseTab::Headers => &self.header_lines,
        }
    }

    pub fn content_kind(&self, tab: ResponseTab) -> ContentKind {
        match tab {
            ResponseTab::Body => self.kind,
            ResponseTab::Headers => ContentKind::Headers,
        }
    }
}

fn format_body(bytes: &[u8]) -> (String, ContentKind) {
    match serde_json::from_slice::<Value>(bytes) {
        Ok(value) => (
            serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string()),
            ContentKind::Json,
        ),
        Err(_) => (
            String::from_utf8_lossy(bytes).to_string(),
            ContentKind::Plain,
        ),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseSearch {
    pub input: TextInput,
    pub editing: bool,
    pub matches: Vec<Match>,
    pub current: usize,
}

impl ResponseSearch {
    pub fn current_match(&self) -> Option<Match> {
        self.matches.get(self.current).copied()
    }
}

/// Size of the response viewer at the last render, used to clamp scrolling.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Viewport {
    pub total_rows: usize,
    pub height: usize,
    pub width: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DragTarget {
    Sidebar,
    ResponseSplit,
    EnvironmentList,
}

#[derive(Debug)]
pub struct App {
    pub should_quit: bool,
    pub screen: Screen,
    pub focus: Focus,
    pub request_tab: RequestTab,
    pub project: ProjectView,
    pub project_error: Option<String>,
    pub saved_requests: Vec<SavedRequestInfo>,
    pub selected_request: usize,
    pub sidebar_filter: TextInput,
    pub sidebar_searching: bool,
    pub sidebar_visible: bool,
    /// Width chosen by the user; `None` fits the longest request name.
    pub sidebar_width: Option<u16>,
    pub sidebar_offset: Cell<usize>,
    pub draft: RequestDraft,
    pub clean_draft: RequestDraft,
    pub loaded_name: Option<String>,
    pub response: ResponseState,
    pub response_tab: ResponseTab,
    pub response_scroll: usize,
    pub response_zoomed: bool,
    pub response_percent: u16,
    pub response_viewport: Cell<Viewport>,
    pub response_search: Option<ResponseSearch>,
    pub selected_env: usize,
    pub selected_env_var: usize,
    pub env_list_width: u16,
    pub overlay: Option<Overlay>,
    pub overlay_scroll_limit: Cell<usize>,
    pub status: StatusLine,
    pub spinner: usize,
    focus_at_send: Option<Focus>,
    drag: Option<DragTarget>,
    disk_stamp: Option<DiskStamp>,
    disk_checked: Option<Instant>,
}

/// Modification times of the project file and folder, to notice edits made elsewhere
/// (for example with the CLI in another terminal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DiskStamp {
    project_file: Option<SystemTime>,
    folder: Option<SystemTime>,
}

impl DiskStamp {
    fn read() -> DiskStamp {
        let modified = |path: &str| fs::metadata(path).and_then(|meta| meta.modified()).ok();
        DiskStamp {
            project_file: modified(PROJECT_ROOT_FILE),
            folder: modified("."),
        }
    }
}

impl App {
    /// Opens the project in the current directory, offering to create one if missing.
    pub fn new() -> App {
        let mut app = App::detached();
        app.refresh_workspace();
        if app.project_error.is_none() && !app.project.exists {
            app.open_create_project_prompt();
            app.set_info("No gemon.json here yet. Create a project, or press Esc to send ad-hoc requests.");
        }
        app
    }

    /// An app that has not read anything from disk.
    pub fn detached() -> App {
        App {
            should_quit: false,
            screen: Screen::Requests,
            focus: Focus::Url,
            request_tab: RequestTab::Headers,
            project: ProjectView::default(),
            project_error: None,
            saved_requests: Vec::new(),
            selected_request: 0,
            sidebar_filter: TextInput::single(""),
            sidebar_searching: false,
            sidebar_visible: true,
            sidebar_width: None,
            sidebar_offset: Cell::new(0),
            draft: RequestDraft::default(),
            clean_draft: RequestDraft::default(),
            loaded_name: None,
            response: ResponseState::Empty,
            response_tab: ResponseTab::Body,
            response_scroll: 0,
            response_zoomed: false,
            response_percent: DEFAULT_RESPONSE_PERCENT,
            response_viewport: Cell::new(Viewport::default()),
            response_search: None,
            selected_env: 0,
            selected_env_var: 0,
            env_list_width: DEFAULT_SIDEBAR_WIDTH,
            overlay: None,
            overlay_scroll_limit: Cell::new(usize::MAX),
            status: StatusLine {
                message: String::from("Ready"),
                kind: StatusKind::Info,
            },
            spinner: 0,
            focus_at_send: None,
            drag: None,
            disk_stamp: None,
            disk_checked: None,
        }
    }

    /// Reloads the project when it changed on disk since the last check, at most once a second.
    pub fn sync_with_disk(&mut self) {
        // Dialogs hold names and positions from when they opened; reload once they close.
        if self.overlay.is_some() {
            return;
        }
        if self
            .disk_checked
            .is_some_and(|checked| checked.elapsed() < Duration::from_secs(1))
        {
            return;
        }
        self.disk_checked = Some(Instant::now());
        if self.disk_stamp.is_some_and(|stamp| stamp != DiskStamp::read()) {
            self.refresh_workspace();
        }
    }

    pub fn refresh_workspace(&mut self) {
        // Remember highlighted items by name: reloading can insert or remove items before them.
        let highlighted_request = self
            .saved_requests
            .get(self.selected_request)
            .map(|request| request.name.clone());
        let highlighted_env = self.selected_env_row().name().to_string();

        self.disk_stamp = Some(DiskStamp::read());
        match try_get_project() {
            Ok(project) => {
                self.project_error = None;
                self.project = project.map(ProjectView::from_project).unwrap_or_default();
            }
            Err(err) => {
                self.project = ProjectView::default();
                self.set_error(format!("{err}. Fix the file and reload (Ctrl+P → Reload)."));
                self.project_error = Some(err);
            }
        }
        self.saved_requests = if self.project.exists {
            list_saved_requests().unwrap_or_else(|err| {
                self.set_error(format!("Could not list saved requests: {err}"));
                Vec::new()
            })
        } else {
            Vec::new()
        };

        if let Some(index) = highlighted_request
            .and_then(|name| self.saved_requests.iter().position(|request| request.name == name))
        {
            self.selected_request = index;
        }
        if let Some(row) = self
            .env_rows()
            .iter()
            .position(|row| row.name() == highlighted_env)
        {
            if row != self.selected_env {
                self.selected_env = row;
                self.selected_env_var = 0;
            }
        }
        self.clamp_request_selection();
        self.clamp_environment_selection();
    }

    pub fn effective_sidebar_width(&self) -> u16 {
        self.sidebar_width.unwrap_or_else(|| {
            let longest = self
                .saved_requests
                .iter()
                .map(|request| unicode_width::UnicodeWidthStr::width(request.name.as_str()))
                .max()
                .unwrap_or_default();
            // Marker, method column, borders and scrollbar take 11 columns.
            (longest as u16 + 11).max(DEFAULT_SIDEBAR_WIDTH)
        })
    }

    pub fn is_dirty(&self) -> bool {
        !self.draft.same_content(&self.clean_draft)
    }

    pub fn is_busy(&self) -> bool {
        matches!(self.response, ResponseState::Loading { .. })
    }

    /// Whether keystrokes currently go into a text field, so plain letters are not commands.
    pub fn is_typing(&self) -> bool {
        if let Some(overlay) = &self.overlay {
            return matches!(overlay, Overlay::Prompt(_) | Overlay::Palette(_));
        }
        match self.focus {
            Focus::Url | Focus::Body => true,
            Focus::Sidebar => self.sidebar_searching,
            Focus::Response => self
                .response_search
                .as_ref()
                .is_some_and(|search| search.editing),
            _ => false,
        }
    }

    pub fn on_tick(&mut self) {
        if self.is_busy() {
            self.spinner = self.spinner.wrapping_add(1);
        }
    }

    /// Any input after sending means the user moved on; the response must not grab focus.
    fn note_user_input(&mut self) {
        self.focus_at_send = None;
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> AppCommand {
        self.note_user_input();
        if is_ctrl(key, 'c') || is_ctrl(key, 'q') {
            return self.perform(Action::Quit);
        }

        if self.overlay.is_some() {
            return self.handle_overlay_key(key);
        }

        if is_ctrl(key, 'p') {
            self.open_palette();
            return AppCommand::None;
        }

        if let Some(action) = self.global_action(key) {
            return self.perform(action);
        }

        match key.code {
            KeyCode::Tab => {
                self.cycle_focus(1);
                return AppCommand::None;
            }
            KeyCode::BackTab => {
                self.cycle_focus(-1);
                return AppCommand::None;
            }
            KeyCode::Esc => return self.handle_escape(),
            KeyCode::Char('?') if !self.is_typing() => return self.perform(Action::Help),
            _ => {}
        }

        match self.focus {
            Focus::Sidebar => self.handle_sidebar_key(key),
            Focus::Method => self.handle_method_key(key),
            Focus::Url => self.handle_url_key(key),
            Focus::Headers => self.handle_pair_key(key, PairTarget::Header),
            Focus::Form => self.handle_pair_key(key, PairTarget::Form),
            Focus::Body => {
                self.draft.body.handle_key(key);
                AppCommand::None
            }
            Focus::Auth => self.handle_auth_key(key),
            Focus::Response => self.handle_response_key(key),
            Focus::EnvList => self.handle_env_list_key(key),
            Focus::EnvVars => self.handle_env_vars_key(key),
        }
    }

    /// Shortcuts that work from anywhere outside overlays. They only use keys that legacy
    /// terminals deliver reliably (Ctrl+letter and function keys).
    fn global_action(&self, key: KeyEvent) -> Option<Action> {
        if key.code == KeyCode::Enter && key.modifiers.contains(KeyModifiers::CONTROL) {
            return Some(Action::Send);
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && !key.modifiers.contains(KeyModifiers::ALT)
        {
            return match key.code {
                KeyCode::Char('r') => Some(Action::Send),
                KeyCode::Char('s') => Some(Action::Save),
                KeyCode::Char('n') => Some(Action::NewRequest),
                KeyCode::Char('l') => Some(Action::FocusUrl),
                KeyCode::Char('f') => Some(Action::FindRequest),
                KeyCode::Char('g') => Some(Action::SwitchEnvironment),
                KeyCode::Char('t') => Some(Action::ChangeMethod),
                KeyCode::Char('o') => Some(Action::ImportOpenApi),
                _ => None,
            };
        }
        match key.code {
            KeyCode::F(1) => Some(Action::ShowRequests),
            KeyCode::F(2) => Some(Action::ShowEnvironments),
            KeyCode::F(3) => Some(Action::Help),
            _ => None,
        }
    }

    pub fn handle_paste(&mut self, text: &str) {
        self.note_user_input();
        if let Some(overlay) = self.overlay.as_mut() {
            match overlay {
                Overlay::Prompt(prompt) => {
                    if let Some(field) = prompt.fields.get_mut(prompt.active) {
                        field.input.insert_str(text);
                    }
                }
                Overlay::Palette(palette) => {
                    palette.input.insert_str(text);
                    palette.selected = 0;
                }
                _ => {}
            }
            return;
        }

        match self.focus {
            Focus::Url => self.draft.url.insert_str(text.trim()),
            Focus::Body => self.draft.body.insert_str(text),
            Focus::Sidebar => {
                self.sidebar_searching = true;
                self.sidebar_filter.insert_str(text);
                self.clamp_request_selection();
            }
            Focus::Response => {
                if let Some(search) = self.response_search.as_mut().filter(|s| s.editing) {
                    search.input.insert_str(text);
                    self.update_response_search();
                }
            }
            _ => {}
        }
    }

    /// Runs a user-facing action, whether triggered by a key, the palette, or the mouse.
    pub fn perform(&mut self, action: Action) -> AppCommand {
        match action {
            Action::Send => return self.send_request(),
            Action::CancelRequest => return self.cancel_request(),
            Action::Save => self.save_draft(None),
            Action::SaveAs => self.open_save_prompt(None, true),
            Action::NewRequest => self.guard_unsaved(PendingAction::New),
            Action::OpenRequest => self.open_selected_request(),
            Action::RenameRequest => self.open_rename_request_prompt(),
            Action::DuplicateRequest => self.open_duplicate_request_prompt(),
            Action::DeleteRequest => self.confirm_delete_selected_request(),
            Action::FindRequest => self.start_sidebar_search(),
            Action::FocusUrl => self.set_focus(Focus::Url),
            Action::FocusBody => self.set_focus(Focus::Body),
            Action::FocusResponse => self.set_focus(Focus::Response),
            Action::ChangeMethod => self.open_method_picker(),
            Action::ToggleSecure => self.toggle_secure(),
            Action::FormatBody => self.format_body(),
            Action::SwitchEnvironment => self.open_environment_picker(),
            Action::ShowRequests => self.show_screen(Screen::Requests),
            Action::ShowEnvironments => self.show_screen(Screen::Environments),
            Action::NewEnvironment => self.open_new_environment_prompt(),
            Action::EditAuthorization => {
                let env = self
                    .project
                    .selected_environment
                    .clone()
                    .unwrap_or_else(|| NO_ENV.to_string());
                self.open_authorization_prompt(env);
            }
            Action::ToggleSidebar => self.toggle_sidebar(),
            Action::ToggleZoom => self.toggle_zoom(),
            Action::SearchResponse => self.start_response_search(),
            Action::CopyResponse => self.copy_response(),
            Action::SaveResponse => self.open_save_response_prompt(false),
            Action::SaveResponseTimestamped => self.open_save_response_prompt(true),
            Action::OpenLastResponse => self.open_last_response(),
            Action::ShowCliCommand => self.show_cli_command(),
            Action::ShowCurlCommand => self.show_curl_command(),
            Action::ImportOpenApi => self.confirm_import_openapi(),
            Action::Reload => {
                self.refresh_workspace();
                if self.project_error.is_none() {
                    self.set_success("Reloaded project from disk");
                }
            }
            Action::CreateProject => {
                if self.project.exists {
                    self.set_info("This folder already has a gemon project");
                } else {
                    self.open_create_project_prompt();
                }
            }
            Action::Help => self.overlay = Some(Overlay::Help { scroll: 0 }),
            Action::Quit => self.guard_unsaved(PendingAction::Quit),
        }
        AppCommand::None
    }

    fn handle_escape(&mut self) -> AppCommand {
        if self.focus == Focus::Sidebar && (self.sidebar_searching || !self.sidebar_filter.is_empty())
        {
            if self.sidebar_filter.is_empty() {
                self.sidebar_searching = false;
            } else {
                self.sidebar_filter.clear();
                self.clamp_request_selection();
            }
            return AppCommand::None;
        }
        if self.focus == Focus::Response && self.response_search.is_some() {
            self.response_search = None;
            return AppCommand::None;
        }
        if self.is_busy() {
            return self.cancel_request();
        }
        if self.response_zoomed {
            self.response_zoomed = false;
            return AppCommand::None;
        }
        if self.screen == Screen::Environments {
            self.show_screen(Screen::Requests);
        }
        AppCommand::None
    }

    pub fn set_focus(&mut self, focus: Focus) {
        self.screen = focus.screen();
        if let Some(tab) = RequestTab::from_focus(focus) {
            self.request_tab = tab;
        }
        if focus == Focus::Sidebar {
            self.sidebar_visible = true;
        } else {
            self.sidebar_searching = false;
        }
        if focus != Focus::Response {
            self.response_zoomed = false;
            if let Some(search) = self.response_search.as_mut() {
                search.editing = false;
            }
        }
        self.focus = focus;
    }

    fn focus_order(&self) -> Vec<Focus> {
        match self.screen {
            Screen::Requests => REQUEST_FOCUS_ORDER
                .into_iter()
                .filter(|focus| *focus != Focus::Sidebar || self.sidebar_visible)
                .collect(),
            Screen::Environments => ENVIRONMENT_FOCUS_ORDER.to_vec(),
        }
    }

    fn cycle_focus(&mut self, delta: isize) {
        let order = self.focus_order();
        let current = order
            .iter()
            .position(|focus| *focus == self.focus)
            .unwrap_or_default();
        let next = move_index(current, order.len(), delta);
        self.set_focus(order[next]);
    }

    fn show_screen(&mut self, screen: Screen) {
        if self.screen == screen {
            return;
        }
        match screen {
            Screen::Requests => self.set_focus(Focus::Url),
            Screen::Environments => {
                self.selected_env = self.active_env_row();
                self.selected_env_var = 0;
                self.set_focus(Focus::EnvList);
            }
        }
    }

    fn toggle_sidebar(&mut self) {
        self.sidebar_visible = !self.sidebar_visible;
        if !self.sidebar_visible && self.focus == Focus::Sidebar {
            self.set_focus(Focus::Url);
        }
    }

    fn require_project(&mut self) -> bool {
        if self.project.exists {
            return true;
        }
        if let Some(err) = &self.project_error {
            self.set_error(format!("{err}. Fix gemon.json and reload."));
            return false;
        }
        self.open_create_project_prompt();
        self.set_warning("This needs a project. Create one first.");
        false
    }

    pub fn set_info(&mut self, message: impl Into<String>) {
        self.set_status(message, StatusKind::Info);
    }

    pub fn set_success(&mut self, message: impl Into<String>) {
        self.set_status(message, StatusKind::Success);
    }

    pub fn set_warning(&mut self, message: impl Into<String>) {
        self.set_status(message, StatusKind::Warning);
    }

    pub fn set_error(&mut self, message: impl Into<String>) {
        self.set_status(message, StatusKind::Error);
    }

    fn set_status(&mut self, message: impl Into<String>, kind: StatusKind) {
        self.status = StatusLine {
            message: message.into(),
            kind,
        };
    }
}

fn is_ctrl(key: KeyEvent, character: char) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(key.code, KeyCode::Char(c) if c.eq_ignore_ascii_case(&character))
}

pub fn move_index(current: usize, len: usize, delta: isize) -> usize {
    if len == 0 {
        return 0;
    }
    (current as isize + delta).rem_euclid(len as isize) as usize
}

/// Moves within `0..len` without wrapping, for paging through long lists.
pub fn step_index(current: usize, len: usize, delta: isize) -> usize {
    if len == 0 {
        return 0;
    }
    (current as isize + delta).clamp(0, len as isize - 1) as usize
}

pub fn format_size(bytes: usize) -> String {
    match bytes {
        0..=1023 => format!("{bytes} B"),
        1024..=1_048_575 => format!("{:.1} KB", bytes as f64 / 1024.0),
        _ => format!("{:.1} MB", bytes as f64 / 1_048_576.0),
    }
}

pub fn format_duration(duration: Duration) -> String {
    let millis = duration.as_millis();
    if millis < 1000 {
        format!("{millis} ms")
    } else {
        format!("{:.2} s", duration.as_secs_f64())
    }
}

pub fn status_text(status: u16) -> String {
    match reqwest::StatusCode::from_u16(status)
        .ok()
        .and_then(|code| code.canonical_reason())
    {
        Some(reason) => format!("{status} {reason}"),
        None => status.to_string(),
    }
}

/// Shows the first and last characters of a secret.
pub fn mask_secret(secret: &str) -> String {
    let chars = secret.chars().collect::<Vec<_>>();
    if chars.len() <= 12 {
        return "•".repeat(chars.len().max(4));
    }
    let head = chars[..6].iter().collect::<String>();
    let tail = chars[chars.len() - 4..].iter().collect::<String>();
    format!("{head}…{tail}")
}
