use super::{
    move_index, step_index, App, AppCommand, Confirm, ConfirmKind, Focus, KeyValue, Overlay,
    PairTarget, PendingAction, Prompt, PromptKind, RequestDraft, RequestTab, ResponseSearch,
    ResponseState, ResponseTab, ResponseView, TextView,
};
use crate::{
    config::types::GemonMethodType,
    project::{
        import_openapi_requests,
        project_handler::{
            create_project, delete_request, duplicate_request, existing_entry_name, read_saved_rest_request,
            rename_request, request_slot, save_rest_request, set_last_response_path,
            validate_request_name, RequestSlot, SavedRequestInfo,
        },
    },
    request::request_builder::GemonResponse,
    tui::{
        export::{self, ExportRequest},
        viewer,
    },
};
use chrono::Local;
use crossterm::event::{KeyCode, KeyEvent};
use serde_json::Value;
use std::{fs, path::Path, time::Instant};

const PAGE: isize = 10;

impl App {
    pub(super) fn handle_sidebar_key(&mut self, key: KeyEvent) -> AppCommand {
        let len = self.visible_request_indices().len();
        if self.sidebar_searching {
            match key.code {
                KeyCode::Enter => {
                    // A search used to open something is done; clear it like a quick-open box.
                    self.sidebar_searching = false;
                    let selected = self.highlighted_request().map(|request| request.name.clone());
                    self.open_selected_request();
                    if let Some(name) = selected {
                        self.sidebar_filter.clear();
                        self.select_request_named(&name);
                    }
                }
                KeyCode::Up => self.move_request_selection(-1),
                KeyCode::Down => self.move_request_selection(1),
                KeyCode::PageUp => self.move_request_selection(-PAGE),
                KeyCode::PageDown => self.move_request_selection(PAGE),
                _ => {
                    if self.sidebar_filter.handle_key(key) {
                        self.clamp_request_selection();
                    }
                }
            }
            return AppCommand::None;
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.move_request_selection(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_request_selection(1),
            KeyCode::PageUp => self.move_request_selection(-PAGE),
            KeyCode::PageDown => self.move_request_selection(PAGE),
            KeyCode::Home | KeyCode::Char('g') => self.select_visible_request(0),
            KeyCode::End | KeyCode::Char('G') => self.select_visible_request(len.saturating_sub(1)),
            KeyCode::Enter | KeyCode::Char('o') => self.open_selected_request(),
            KeyCode::Char('/') => self.start_sidebar_search(),
            KeyCode::Char('n') => self.guard_unsaved(PendingAction::New),
            KeyCode::Char('r') => self.open_rename_request_prompt(),
            KeyCode::Char('c') => self.open_duplicate_request_prompt(),
            KeyCode::Char('d') | KeyCode::Char('x') | KeyCode::Delete => {
                self.confirm_delete_selected_request()
            }
            KeyCode::Char('<') => self.resize_sidebar(-4),
            KeyCode::Char('>') => self.resize_sidebar(4),
            _ => {}
        }
        AppCommand::None
    }

    fn resize_sidebar(&mut self, delta: i16) {
        let width = self.effective_sidebar_width().saturating_add_signed(delta);
        self.sidebar_width = Some(width.clamp(20, 120));
    }

    pub(super) fn handle_method_key(&mut self, key: KeyEvent) -> AppCommand {
        match key.code {
            KeyCode::Left | KeyCode::Up => self.draft.method = self.draft.method.previous(),
            KeyCode::Right | KeyCode::Down | KeyCode::Char(' ') => {
                self.draft.method = self.draft.method.next()
            }
            KeyCode::Enter => self.open_method_picker(),
            _ => {}
        }
        AppCommand::None
    }

    pub(super) fn handle_url_key(&mut self, key: KeyEvent) -> AppCommand {
        if key.code == KeyCode::Enter {
            return self.send_request();
        }
        self.draft.url.handle_key(key);
        AppCommand::None
    }

    pub(super) fn handle_pair_key(&mut self, key: KeyEvent, target: PairTarget) -> AppCommand {
        let len = self.draft.pairs(target).len();
        let selected = self.draft.selected_pair(target);
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.select_pair(target, move_index(selected, len, -1)),
            KeyCode::Down | KeyCode::Char('j') => self.select_pair(target, move_index(selected, len, 1)),
            KeyCode::Home => self.select_pair(target, 0),
            KeyCode::End => self.select_pair(target, len.saturating_sub(1)),
            KeyCode::Char('a') | KeyCode::Char('n') | KeyCode::Insert => {
                self.open_pair_prompt(target, None)
            }
            KeyCode::Enter | KeyCode::Char('e') if len > 0 => {
                self.open_pair_prompt(target, Some(selected))
            }
            KeyCode::Enter => self.open_pair_prompt(target, None),
            KeyCode::Char('d') | KeyCode::Char('x') | KeyCode::Delete => self.remove_pair(target),
            _ => {}
        }
        AppCommand::None
    }

    pub(super) fn handle_auth_key(&mut self, key: KeyEvent) -> AppCommand {
        match key.code {
            KeyCode::Char(' ') | KeyCode::Enter => self.toggle_secure(),
            KeyCode::Char('e') => {
                return self.perform(crate::tui::actions::Action::EditAuthorization)
            }
            _ => {}
        }
        AppCommand::None
    }

    pub(super) fn handle_response_key(&mut self, key: KeyEvent) -> AppCommand {
        if let Some(search) = self.response_search.as_mut().filter(|search| search.editing) {
            match key.code {
                KeyCode::Enter => {
                    search.editing = false;
                    if search.matches.is_empty() && !search.input.is_empty() {
                        self.set_warning("No matches");
                    }
                }
                KeyCode::Down => self.step_search_match(1),
                KeyCode::Up => self.step_search_match(-1),
                _ => {
                    if search.input.handle_key(key) {
                        self.update_response_search();
                    }
                }
            }
            return AppCommand::None;
        }

        let page = self.response_viewport.get().height.max(1) as isize;
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.scroll_response(-1),
            KeyCode::Down | KeyCode::Char('j') => self.scroll_response(1),
            KeyCode::PageUp | KeyCode::Char('b') => self.scroll_response(-page),
            KeyCode::PageDown | KeyCode::Char(' ') => self.scroll_response(page),
            KeyCode::Home | KeyCode::Char('g') => self.response_scroll = 0,
            KeyCode::End | KeyCode::Char('G') => self.scroll_response(isize::MAX / 2),
            KeyCode::Left | KeyCode::Right | KeyCode::Char('h') => self.toggle_response_tab(),
            KeyCode::Char('/') => self.start_response_search(),
            KeyCode::Char('n') => self.step_search_match(1),
            KeyCode::Char('N') => self.step_search_match(-1),
            KeyCode::Char('y') => self.copy_response(),
            KeyCode::Char('s') => self.open_save_response_prompt(false),
            KeyCode::Char('S') => self.open_save_response_prompt(true),
            KeyCode::Char('z') => self.toggle_zoom(),
            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.response_percent = (self.response_percent + 5).min(85)
            }
            KeyCode::Char('-') => self.response_percent = self.response_percent.saturating_sub(5).max(20),
            _ => {}
        }
        AppCommand::None
    }

    // Sidebar ------------------------------------------------------------------------------

    pub fn visible_request_indices(&self) -> Vec<usize> {
        let filter = self.sidebar_filter.value().trim().to_lowercase();
        self.saved_requests
            .iter()
            .enumerate()
            .filter(|(_, request)| {
                filter.is_empty()
                    || request.name.to_lowercase().contains(&filter)
                    || request
                        .method
                        .is_some_and(|method| method.as_str().eq_ignore_ascii_case(&filter))
            })
            .map(|(index, _)| index)
            .collect()
    }

    pub fn selected_visible_position(&self) -> Option<usize> {
        self.visible_request_indices()
            .iter()
            .position(|index| *index == self.selected_request)
    }

    pub fn highlighted_request(&self) -> Option<&SavedRequestInfo> {
        self.selected_visible_position()
            .and_then(|_| self.saved_requests.get(self.selected_request))
    }

    pub(super) fn move_request_selection(&mut self, delta: isize) {
        let visible = self.visible_request_indices();
        if visible.is_empty() {
            return;
        }
        let current = self.selected_visible_position().unwrap_or_default();
        let next = if delta.abs() == 1 {
            move_index(current, visible.len(), delta)
        } else {
            step_index(current, visible.len(), delta)
        };
        self.selected_request = visible[next];
    }

    pub(super) fn select_visible_request(&mut self, position: usize) {
        if let Some(index) = self.visible_request_indices().get(position) {
            self.selected_request = *index;
        }
    }

    pub(super) fn clamp_request_selection(&mut self) {
        let visible = self.visible_request_indices();
        if !visible.contains(&self.selected_request) {
            self.selected_request = visible.first().copied().unwrap_or_default();
        }
    }

    fn select_request_named(&mut self, name: &str) {
        if let Some(index) = self.saved_requests.iter().position(|r| r.name == name) {
            self.selected_request = index;
            if self.selected_visible_position().is_none() {
                self.sidebar_filter.clear();
            }
        }
    }

    pub(super) fn start_sidebar_search(&mut self) {
        self.set_focus(Focus::Sidebar);
        self.sidebar_searching = true;
    }

    pub(super) fn open_selected_request(&mut self) {
        let Some(request) = self.highlighted_request().cloned() else {
            self.set_info(if self.saved_requests.is_empty() {
                "No saved requests yet. Build one and press Ctrl+S."
            } else {
                "No request matches the filter"
            });
            return;
        };
        if self.loaded_name.as_deref() == Some(request.name.as_str()) && !self.is_dirty() {
            self.set_focus(Focus::Url);
            return;
        }
        self.guard_unsaved(PendingAction::Load(request.name));
    }

    /// Runs `pending` now, or asks first when it would throw away unsaved edits.
    pub(super) fn guard_unsaved(&mut self, pending: PendingAction) {
        let worth_keeping = self.loaded_name.is_some() || !self.draft.same_content(&RequestDraft::default());
        if self.is_dirty() && worth_keeping {
            let name = self.loaded_name.as_deref().unwrap_or("the new request");
            let consequence = match &pending {
                PendingAction::Load(other) => format!("before opening '{other}'"),
                PendingAction::New => String::from("before starting a new request"),
                PendingAction::Quit => String::from("before quitting"),
            };
            self.overlay = Some(Overlay::Confirm(Confirm {
                title: String::from("Unsaved changes"),
                message: format!("Save changes to {name} {consequence}?"),
                kind: ConfirmKind::Discard(pending),
            }));
            return;
        }
        self.run_pending(pending);
    }

    pub(super) fn run_pending(&mut self, pending: PendingAction) {
        match pending {
            PendingAction::Load(name) => self.load_request(&name),
            PendingAction::New => {
                self.draft = RequestDraft::default();
                self.clean_draft = RequestDraft::default();
                self.loaded_name = None;
                self.reset_response();
                self.request_tab = RequestTab::Headers;
                self.set_focus(Focus::Url);
                self.set_info("New request. Type a URL and press Enter to send.");
            }
            PendingAction::Quit => self.should_quit = true,
        }
    }

    fn load_request(&mut self, name: &str) {
        let Some(info) = self.saved_requests.iter().find(|r| r.name == name) else {
            self.set_error(format!("Saved request '{name}' no longer exists"));
            return;
        };
        if info.request_type != "REST" {
            self.set_error(format!(
                "'{name}' is a {} request; the TUI edits REST requests",
                info.request_type
            ));
            return;
        }

        match read_saved_rest_request(name) {
            Ok(request) => {
                self.draft = RequestDraft::from_saved(&request);
                self.clean_draft = self.draft.clone();
                self.loaded_name = Some(name.to_string());
                self.select_request_named(name);
                self.reset_response();
                self.request_tab = if self.draft.body_text().is_some() {
                    RequestTab::Body
                } else if !self.draft.headers.is_empty() || self.draft.form.is_empty() {
                    RequestTab::Headers
                } else {
                    RequestTab::Form
                };
                self.set_focus(Focus::Url);
                self.set_success(format!("Opened '{name}'. Enter sends it."));
            }
            Err(err) => self.set_error(format!("Could not open '{name}': {err}")),
        }
    }

    pub(super) fn open_rename_request_prompt(&mut self) {
        let Some(request) = self.highlighted_request().cloned() else {
            self.set_info("Highlight a saved request to rename it");
            return;
        };
        self.overlay = Some(Overlay::Prompt(
            Prompt::new(
                "Rename request",
                PromptKind::RenameRequest {
                    old: request.name.clone(),
                },
            )
            .field("Name", request.name, "request name")
            .hint("Renames the request folder in the project."),
        ));
    }

    pub(super) fn submit_rename_request(&mut self, prompt: Prompt, old: String) {
        let new = prompt.value(0).trim().to_string();
        if let Err(err) = validate_request_name(&new) {
            return self.reject_prompt(prompt, err);
        }
        match rename_request(&old, &new) {
            Ok(()) => {
                if self.loaded_name.as_deref() == Some(old.as_str()) {
                    self.loaded_name = Some(new.clone());
                }
                self.refresh_workspace();
                self.select_request_named(&new);
                self.set_success(format!("Renamed '{old}' to '{new}'"));
            }
            Err(err) => self.reject_prompt(prompt, err.to_string()),
        }
    }

    pub(super) fn open_duplicate_request_prompt(&mut self) {
        let Some(request) = self.highlighted_request().cloned() else {
            self.set_info("Highlight a saved request to duplicate it");
            return;
        };
        let suggestion = self.unused_name(&format!("{}_copy", request.name));
        self.overlay = Some(Overlay::Prompt(
            Prompt::new(
                "Duplicate request",
                PromptKind::DuplicateRequest {
                    source: request.name,
                },
            )
            .field("New name", suggestion, "request name"),
        ));
    }

    pub(super) fn submit_duplicate_request(&mut self, prompt: Prompt, source: String) {
        let new = prompt.value(0).trim().to_string();
        if let Err(err) = validate_request_name(&new) {
            return self.reject_prompt(prompt, err);
        }
        match duplicate_request(&source, &new) {
            Ok(()) => {
                self.refresh_workspace();
                self.select_request_named(&new);
                self.set_success(format!("Duplicated '{source}' as '{new}'"));
            }
            Err(err) => self.reject_prompt(prompt, err.to_string()),
        }
    }

    pub(super) fn confirm_delete_selected_request(&mut self) {
        let Some(request) = self.highlighted_request().cloned() else {
            self.set_info("Highlight a saved request to delete it");
            return;
        };
        self.overlay = Some(Overlay::Confirm(Confirm {
            title: String::from("Delete request"),
            message: format!(
                "Delete '{}' and everything in its folder, including saved responses?",
                request.name
            ),
            kind: ConfirmKind::DeleteRequest(request.name),
        }));
    }

    pub(super) fn delete_saved_request(&mut self, name: String) {
        if request_slot(&name) != RequestSlot::SavedRequest {
            self.set_error(format!("'{name}' is not a saved request"));
            return;
        }
        match delete_request(&name) {
            Ok(()) => {
                if self.loaded_name.as_deref() == Some(name.as_str()) {
                    // Keep the editor content as an unsaved request.
                    self.loaded_name = None;
                    self.clean_draft = RequestDraft::default();
                }
                self.refresh_workspace();
                self.set_success(format!("Deleted '{name}'"));
            }
            Err(err) => self.set_error(format!("Could not delete '{name}': {err}")),
        }
    }

    fn unused_name(&self, base: &str) -> String {
        if request_slot(base) == RequestSlot::Free {
            return base.to_string();
        }
        (2..)
            .map(|index| format!("{base}_{index}"))
            .find(|name| request_slot(name) == RequestSlot::Free)
            .unwrap_or_else(|| base.to_string())
    }

    // Draft editing ---------------------------------------------------------------------------

    fn select_pair(&mut self, target: PairTarget, index: usize) {
        let (_, selected) = self.draft.pairs_mut(target);
        *selected = index;
    }

    pub(super) fn open_pair_prompt(&mut self, target: PairTarget, index: Option<usize>) {
        let pair = index
            .and_then(|index| self.draft.pairs(target).get(index).cloned())
            .unwrap_or_default();
        let noun = target.noun();
        let title = if index.is_some() {
            format!("Edit {noun}")
        } else {
            format!("Add {noun}")
        };
        let key_placeholder = match target {
            PairTarget::Header => "e.g. Content-Type",
            PairTarget::Form => "field name",
        };
        let focus_value = index.is_some();
        self.overlay = Some(Overlay::Prompt(
            Prompt::new(title, PromptKind::Pair { target, index })
                .field("Name", pair.key, key_placeholder)
                .field("Value", pair.value, "value, may use {placeholders}")
                .hint("Environment placeholders like {token} are resolved when sending.")
                .focus_field(usize::from(focus_value)),
        ));
    }

    pub(super) fn submit_pair(&mut self, prompt: Prompt, target: PairTarget, index: Option<usize>) {
        let key = prompt.value(0).trim().to_string();
        if key.is_empty() {
            return self.reject_prompt(prompt, "Name is required");
        }
        let duplicate = self
            .draft
            .pairs(target)
            .iter()
            .enumerate()
            .any(|(position, pair)| Some(position) != index && pair.key.eq_ignore_ascii_case(&key));
        if duplicate {
            return self.reject_prompt(prompt, format!("'{key}' is already set; edit that row instead"));
        }

        let pair = KeyValue {
            key,
            value: prompt.value(1),
        };
        let (pairs, selected) = self.draft.pairs_mut(target);
        match index {
            Some(index) if index < pairs.len() => {
                pairs[index] = pair;
                *selected = index;
            }
            _ => {
                pairs.push(pair);
                *selected = pairs.len() - 1;
            }
        }
        self.set_focus(match target {
            PairTarget::Header => Focus::Headers,
            PairTarget::Form => Focus::Form,
        });
    }

    fn remove_pair(&mut self, target: PairTarget) {
        let noun = target.noun();
        let (pairs, selected) = self.draft.pairs_mut(target);
        if pairs.is_empty() {
            return;
        }
        let index = (*selected).min(pairs.len() - 1);
        let removed = pairs.remove(index);
        *selected = index.min(pairs.len().saturating_sub(1));
        self.set_info(format!("Removed {noun} '{}'", removed.key));
    }

    pub(super) fn toggle_secure(&mut self) {
        self.draft.secure = !self.draft.secure;
        if !self.draft.secure {
            self.set_info("This request no longer sends the project authorization");
        } else if self.project.active_authorization().is_some() {
            self.set_info(format!(
                "Sends the authorization of {}",
                self.project.active_environment_label()
            ));
        } else {
            self.set_warning(format!(
                "Will send authorization, but {} has none yet. Press e to set it.",
                self.project.active_environment_label()
            ));
        }
    }

    pub(super) fn format_body(&mut self) {
        let Some(body) = self.draft.body_text() else {
            self.set_info("The body is empty");
            return;
        };
        match serde_json::from_str::<Value>(&body) {
            Ok(value) => {
                let pretty = serde_json::to_string_pretty(&value).unwrap_or(body);
                self.draft.body.set_value(pretty);
                self.set_success("Formatted body as JSON");
            }
            Err(err) => self.set_error(format!("Body is not valid JSON: {err}")),
        }
    }

    // Sending ---------------------------------------------------------------------------------

    pub(super) fn send_request(&mut self) -> AppCommand {
        if let Err(message) = self.draft.validate() {
            self.set_error(message);
            if self.draft.url.value().trim().is_empty() {
                self.set_focus(Focus::Url);
            }
            return AppCommand::None;
        }

        let env = self.project.active_values();
        let request = self.draft.to_request(&env);
        let unresolved = self.draft.unresolved_placeholders(&env);
        let summary = format!("{} {}", request.method(), request.uri());

        self.response = ResponseState::Loading {
            started: Instant::now(),
            summary,
        };
        self.response_scroll = 0;
        self.response_search = None;
        self.focus_at_send = Some(self.focus);

        if unresolved.is_empty() {
            self.set_info("Sending…");
        } else {
            let names = unresolved
                .iter()
                .map(|name| format!("{{{name}}}"))
                .collect::<Vec<_>>()
                .join(", ");
            self.set_warning(format!(
                "Sending with unresolved {names} ({})",
                self.project.active_environment_label()
            ));
        }
        AppCommand::Send(request)
    }

    pub fn finish_request(&mut self, result: Result<GemonResponse, String>) {
        let ResponseState::Loading { started, summary } = &self.response else {
            return;
        };
        let elapsed = started.elapsed();
        let summary = summary.clone();

        match result {
            Ok(response) => {
                let view = ResponseView::live(response, elapsed, summary);
                let status = view.status.unwrap_or_default();
                let message = format!(
                    "{} in {} · {}",
                    super::status_text(status),
                    super::format_duration(elapsed),
                    super::format_size(view.size_bytes)
                );
                self.response = ResponseState::Ready(view);
                if status >= 400 {
                    self.set_warning(message);
                } else {
                    self.set_success(message);
                }
                if self.focus_at_send == Some(self.focus) && self.overlay.is_none() {
                    self.set_focus(Focus::Response);
                }
            }
            Err(message) => {
                self.response = ResponseState::Failed {
                    summary,
                    message: message.clone(),
                };
                self.set_error(format!("Request failed: {message}"));
            }
        }
        self.focus_at_send = None;
    }

    pub(super) fn cancel_request(&mut self) -> AppCommand {
        let ResponseState::Loading { summary, .. } = &self.response else {
            self.set_info("No request is running");
            return AppCommand::None;
        };
        self.response = ResponseState::Cancelled {
            summary: summary.clone(),
        };
        self.focus_at_send = None;
        self.set_info("Request cancelled");
        AppCommand::Cancel
    }

    fn reset_response(&mut self) {
        if !self.is_busy() {
            self.response = ResponseState::Empty;
        }
        self.response_scroll = 0;
        self.response_search = None;
        self.response_zoomed = false;
        self.response_tab = ResponseTab::Body;
    }

    // Saving ----------------------------------------------------------------------------------

    pub(super) fn save_draft(&mut self, then: Option<PendingAction>) {
        if !self.require_project() {
            return;
        }
        if let Err(message) = self.draft.validate() {
            self.set_error(format!("Cannot save yet: {message}"));
            return;
        }
        match self.loaded_name.clone() {
            Some(name) => self.write_draft(name, then),
            None => self.open_save_prompt(then, false),
        }
    }

    pub(super) fn open_save_prompt(&mut self, then: Option<PendingAction>, save_as: bool) {
        if !self.require_project() {
            return;
        }
        let suggestion = match (&self.loaded_name, save_as) {
            (Some(name), true) => self.unused_name(&format!("{name}_copy")),
            _ => self.suggested_request_name(),
        };
        let title = if save_as { "Save request as" } else { "Save request" };
        self.overlay = Some(Overlay::Prompt(
            Prompt::new(title, PromptKind::SaveRequest { then })
                .field("Name", suggestion, "request name")
                .hint("Saved as a folder in the project; call it from the CLI with -c=NAME."),
        ));
    }

    pub(super) fn submit_save_prompt(&mut self, prompt: Prompt, then: Option<PendingAction>) {
        let name = prompt.value(0).trim().to_string();
        if let Err(err) = validate_request_name(&name) {
            return self.reject_prompt(prompt, err);
        }
        if let Err(err) = self.draft.validate() {
            return self.reject_prompt(prompt, err);
        }
        // On case-insensitive file systems "Users" is the existing "users" folder.
        let name = existing_entry_name(&name).unwrap_or(name);
        match request_slot(&name) {
            RequestSlot::Taken => self.reject_prompt(
                prompt,
                format!("'{name}' is an existing file or folder that is not a request"),
            ),
            RequestSlot::SavedRequest if self.loaded_name.as_deref() != Some(name.as_str()) => {
                self.overlay = Some(Overlay::Confirm(Confirm {
                    title: String::from("Overwrite request"),
                    message: format!("A request named '{name}' already exists. Replace it?"),
                    kind: ConfirmKind::Overwrite { name, then },
                }));
            }
            _ => self.write_draft(name, then),
        }
    }

    /// Writes the draft under `name` and, on success, continues with `then`.
    pub(super) fn write_draft(&mut self, name: String, then: Option<PendingAction>) {
        match save_rest_request(&name, &self.draft.to_saved_request()) {
            Ok(()) => {
                self.loaded_name = Some(name.clone());
                self.clean_draft = self.draft.clone();
                self.refresh_workspace();
                self.select_request_named(&name);
                self.set_success(format!("Saved '{name}'"));
                if let Some(then) = then {
                    self.run_pending(then);
                }
            }
            Err(err) => self.set_error(format!("Could not save '{name}': {err}")),
        }
    }

    /// A readable default name from method and path, e.g. `get_users_id`.
    fn suggested_request_name(&self) -> String {
        let url = self.draft.url.value();
        let path = url
            .split("://")
            .last()
            .unwrap_or_default()
            .split(['?', '#'])
            .next()
            .unwrap_or_default();
        let path = path.split_once('/').map(|(_, path)| path).unwrap_or_default();
        let mut name = self.draft.method.as_str().to_lowercase();
        for segment in path.split('/').filter(|segment| !segment.is_empty()) {
            let cleaned = segment
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
                .collect::<String>();
            let cleaned = cleaned.trim_matches('_');
            if !cleaned.is_empty() {
                name.push('_');
                name.push_str(cleaned);
            }
        }
        self.unused_name(&name)
    }

    pub(super) fn open_create_project_prompt(&mut self) {
        let folder = std::env::current_dir()
            .ok()
            .and_then(|dir| dir.file_name().map(|name| name.to_string_lossy().to_string()))
            .unwrap_or_default();
        self.overlay = Some(Overlay::Prompt(
            Prompt::new("Create gemon project", PromptKind::CreateProject)
                .field("Name", folder, "project name")
                .hint("Creates gemon.json in this folder to store requests and environments."),
        ));
    }

    pub(super) fn submit_create_project(&mut self, prompt: Prompt) {
        let name = prompt.value(0).trim().to_string();
        if name.is_empty() {
            return self.reject_prompt(prompt, "Project name is required");
        }
        match create_project(&name) {
            Ok(()) => {
                self.refresh_workspace();
                self.set_success(format!("Created project '{name}'"));
            }
            Err(err) => self.reject_prompt(prompt, err.to_string()),
        }
    }

    // Import ----------------------------------------------------------------------------------

    pub(super) fn confirm_import_openapi(&mut self) {
        if !self.require_project() {
            return;
        }
        self.overlay = Some(Overlay::Confirm(Confirm {
            title: String::from("Import OpenAPI"),
            message: String::from(
                "Import every operation from openapi.yaml files in this folder and its subfolders? \
                 Saved requests with the same names are replaced.",
            ),
            kind: ConfirmKind::ImportOpenApi,
        }));
    }

    pub(super) fn import_openapi(&mut self) {
        match import_openapi_requests() {
            Ok(report) => {
                self.refresh_workspace();
                if let Some(first) = report.request_names.first() {
                    self.sidebar_filter.clear();
                    self.select_request_named(first);
                    self.set_focus(Focus::Sidebar);
                    self.set_success(report.summary());
                } else {
                    self.set_info(report.summary());
                }
            }
            Err(err) => self.set_error(format!("Import failed: {err}")),
        }
    }

    // Response --------------------------------------------------------------------------------

    pub fn response_view(&self) -> Option<&ResponseView> {
        match &self.response {
            ResponseState::Ready(view) => Some(view),
            _ => None,
        }
    }

    pub(super) fn scroll_response(&mut self, delta: isize) {
        let viewport = self.response_viewport.get();
        let max = viewport.total_rows.saturating_sub(viewport.height) as isize;
        self.response_scroll = (self.response_scroll as isize)
            .saturating_add(delta)
            .clamp(0, max.max(0)) as usize;
    }

    fn toggle_response_tab(&mut self) {
        self.response_tab = match self.response_tab {
            ResponseTab::Body => ResponseTab::Headers,
            ResponseTab::Headers => ResponseTab::Body,
        };
        self.response_scroll = 0;
        if self.response_search.is_some() {
            self.update_response_search();
        }
    }

    pub(super) fn toggle_zoom(&mut self) {
        if self.response_zoomed {
            self.response_zoomed = false;
        } else {
            self.set_focus(Focus::Response);
            self.response_zoomed = true;
        }
    }

    pub(super) fn start_response_search(&mut self) {
        if self.response_view().is_none() {
            self.set_info("Nothing to search yet. Send a request first.");
            return;
        }
        self.set_focus(Focus::Response);
        let input = self
            .response_search
            .take()
            .map(|search| search.input)
            .unwrap_or_default();
        self.response_search = Some(ResponseSearch {
            input,
            editing: true,
            matches: Vec::new(),
            current: 0,
        });
        self.update_response_search();
    }

    pub(super) fn update_response_search(&mut self) {
        let tab = self.response_tab;
        let Some(view) = self.response_view() else {
            return;
        };
        let lines = view.lines(tab).to_vec();
        if let Some(search) = self.response_search.as_mut() {
            search.matches = viewer::search_matches(&lines, &search.input.value());
            search.current = 0;
        }
        self.reveal_current_match();
    }

    fn step_search_match(&mut self, delta: isize) {
        let Some(search) = self.response_search.as_mut() else {
            return;
        };
        if search.matches.is_empty() {
            return;
        }
        search.current = move_index(search.current, search.matches.len(), delta);
        self.reveal_current_match();
    }

    fn reveal_current_match(&mut self) {
        let Some(found) = self.response_search.as_ref().and_then(|s| s.current_match()) else {
            return;
        };
        let Some(view) = self.response_view() else {
            return;
        };
        let viewport = self.response_viewport.get();
        let row = viewer::visual_row_of(view.lines(self.response_tab), viewport.width.max(1), found);
        if row < self.response_scroll || row >= self.response_scroll + viewport.height.max(1) {
            self.response_scroll = row.saturating_sub(viewport.height / 3);
        }
    }

    pub(super) fn copy_response(&mut self) {
        let Some(view) = self.response_view() else {
            self.set_info("No response to copy yet");
            return;
        };
        let text = match self.response_tab {
            ResponseTab::Body => view.body.clone(),
            ResponseTab::Headers => view.header_lines.join("\n"),
        };
        let what = match self.response_tab {
            ResponseTab::Body => "response body",
            ResponseTab::Headers => "response headers",
        };
        self.copy_text(&text, what);
    }

    /// Defaults match the CLI: `NAME/response.json` (`-f`) or a timestamped file (`-l`).
    pub(super) fn open_save_response_prompt(&mut self, timestamped: bool) {
        if self.response_view().is_none() {
            self.set_info("No response to save yet");
            return;
        }
        let file = if timestamped {
            format!("response_{}.json", Local::now().format("%Y_%m_%d_%H_%M_%S"))
        } else {
            String::from("response.json")
        };
        let default = match &self.loaded_name {
            Some(name) => format!("{name}/{file}"),
            None => file,
        };
        self.overlay = Some(Overlay::Prompt(
            Prompt::new("Save response", PromptKind::SaveResponse)
                .field("File", default, "path/to/response.json")
                .hint("Like the CLI's -f, -l and -rf. Ctrl+P → Open last saved response reopens it."),
        ));
    }

    pub(super) fn submit_save_response(&mut self, prompt: Prompt) {
        let path = prompt.value(0).trim().to_string();
        if path.is_empty() {
            return self.reject_prompt(prompt, "File path is required");
        }
        let Some(body) = self.response_view().map(|view| view.body.clone()) else {
            return;
        };
        if Path::new(&path).is_dir() {
            return self.reject_prompt(prompt, format!("'{path}' is a folder"));
        }
        if let Some(parent) = Path::new(&path).parent().filter(|p| !p.as_os_str().is_empty()) {
            if let Err(err) = fs::create_dir_all(parent) {
                return self.reject_prompt(prompt, format!("Could not create folder: {err}"));
            }
        }
        if let Err(err) = fs::write(&path, body) {
            return self.reject_prompt(prompt, format!("Could not write file: {err}"));
        }
        if self.project.exists {
            if let Err(err) = set_last_response_path(&path) {
                self.set_warning(format!("Saved to {path}, but could not record it: {err}"));
                return;
            }
            self.refresh_workspace();
        }
        self.set_success(format!("Saved response to {path}"));
    }

    pub(super) fn open_last_response(&mut self) {
        let Some(path) = self.project.last_response_path.clone() else {
            self.set_info("No saved response yet. Press s in the response panel to save one.");
            return;
        };
        match fs::read(&path) {
            Ok(contents) => {
                self.response = ResponseState::Ready(ResponseView::from_file(&path, &contents));
                self.response_tab = ResponseTab::Body;
                self.response_scroll = 0;
                self.response_search = None;
                self.set_focus(Focus::Response);
                self.set_success(format!("Showing {path}"));
            }
            Err(err) => self.set_error(format!("Could not read {path}: {err}")),
        }
    }

    // Export ----------------------------------------------------------------------------------

    pub(super) fn show_cli_command(&mut self) {
        let pairs = |pairs: &[KeyValue]| {
            pairs
                .iter()
                .map(|pair| (pair.key.clone(), pair.value.clone()))
                .collect::<Vec<_>>()
        };
        let headers = pairs(&self.draft.headers);
        let form = pairs(&self.draft.form);
        let body = self.draft.body_text();
        let url = self.draft.url.value();
        let command = export::gemon_command(&ExportRequest {
            method: self.draft.method,
            url: &url,
            headers: &headers,
            body: body.as_deref(),
            form: &form,
            secure: self.draft.secure,
        });
        self.overlay = Some(Overlay::TextView(TextView {
            title: String::from("gemon CLI command"),
            text: command,
            scroll: 0,
        }));
    }

    pub(super) fn show_curl_command(&mut self) {
        let env = self.project.active_values();
        let request = self.draft.to_request(&env);
        let mut headers = request
            .headers()
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect::<Vec<_>>();
        let has_authorization = headers
            .iter()
            .any(|(key, _)| key.eq_ignore_ascii_case("authorization"));
        if request.secure() && !has_authorization {
            if let Some(authorization) = self.project.active_authorization() {
                headers.push((String::from("authorization"), authorization.to_string()));
            }
        }
        headers.sort();
        let mut form = request
            .form_data()
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect::<Vec<_>>();
        form.sort();
        let command = export::curl_command(&ExportRequest {
            method: request.method(),
            url: request.uri(),
            headers: &headers,
            body: request.body(),
            form: &form,
            secure: false,
        });
        self.overlay = Some(Overlay::TextView(TextView {
            title: format!("cURL command ({})", self.project.active_environment_label()),
            text: command,
            scroll: 0,
        }));
    }

    pub fn method_label(method: Option<GemonMethodType>) -> &'static str {
        method.map(|method| method.as_str()).unwrap_or("?")
    }

    pub fn draft_title(&self) -> String {
        self.loaded_name
            .clone()
            .unwrap_or_else(|| String::from("Untitled request"))
    }
}
