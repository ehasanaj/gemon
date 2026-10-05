use super::{move_index, step_index, App, AppCommand, PairTarget};
use crate::{
    config::types::GemonMethodType,
    constants::NO_ENV,
    tui::{
        actions::{fuzzy_score, Action},
        clipboard,
        input::TextInput,
    },
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Modal UI that captures all input until it is closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Overlay {
    Prompt(Prompt),
    Confirm(Confirm),
    Picker(Picker),
    Palette(Palette),
    Help { scroll: usize },
    TextView(TextView),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prompt {
    pub title: String,
    pub fields: Vec<PromptField>,
    pub active: usize,
    pub kind: PromptKind,
    pub hint: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptField {
    pub label: &'static str,
    pub input: TextInput,
    pub placeholder: &'static str,
}

impl Prompt {
    pub fn new(title: impl Into<String>, kind: PromptKind) -> Prompt {
        Prompt {
            title: title.into(),
            fields: Vec::new(),
            active: 0,
            kind,
            hint: None,
            error: None,
        }
    }

    pub fn field(
        mut self,
        label: &'static str,
        value: impl Into<String>,
        placeholder: &'static str,
    ) -> Prompt {
        self.fields.push(PromptField {
            label,
            input: TextInput::single(value),
            placeholder,
        });
        self
    }

    pub fn hint(mut self, hint: impl Into<String>) -> Prompt {
        self.hint = Some(hint.into());
        self
    }

    /// Starts editing on the given field, e.g. the value when the key is already known.
    pub fn focus_field(mut self, index: usize) -> Prompt {
        self.active = index.min(self.fields.len().saturating_sub(1));
        self
    }

    pub fn value(&self, index: usize) -> String {
        self.fields
            .get(index)
            .map(|field| field.input.value())
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptKind {
    CreateProject,
    SaveRequest {
        then: Option<PendingAction>,
    },
    RenameRequest {
        old: String,
    },
    DuplicateRequest {
        source: String,
    },
    NewEnvironment,
    RenameEnvironment {
        old: String,
    },
    EnvVariable {
        env: String,
        old_key: Option<String>,
    },
    Pair {
        target: PairTarget,
        index: Option<usize>,
    },
    Authorization {
        env: String,
    },
    SaveResponse,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Confirm {
    pub title: String,
    pub message: String,
    pub kind: ConfirmKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmKind {
    DeleteRequest(String),
    DeleteEnvironment(String),
    DeleteVariable { env: String, key: String },
    Discard(PendingAction),
    Overwrite {
        name: String,
        then: Option<PendingAction>,
    },
    ImportOpenApi,
}

impl ConfirmKind {
    /// Key hints shown under the message, as (key, label) pairs.
    pub fn choices(&self) -> Vec<(&'static str, &'static str)> {
        match self {
            ConfirmKind::Discard(_) => vec![("s/Enter", "Save"), ("d", "Discard"), ("Esc", "Cancel")],
            ConfirmKind::DeleteRequest(_)
            | ConfirmKind::DeleteEnvironment(_)
            | ConfirmKind::DeleteVariable { .. } => vec![("y/Enter", "Delete"), ("n/Esc", "Cancel")],
            ConfirmKind::Overwrite { .. } => vec![("y/Enter", "Overwrite"), ("n/Esc", "Cancel")],
            ConfirmKind::ImportOpenApi => vec![("y/Enter", "Import"), ("n/Esc", "Cancel")],
        }
    }

    pub fn is_destructive(&self) -> bool {
        !matches!(self, ConfirmKind::ImportOpenApi)
    }
}

/// Work deferred until unsaved changes are saved or discarded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingAction {
    Load(String),
    New,
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picker {
    pub title: String,
    pub items: Vec<PickerItem>,
    pub selected: usize,
    pub kind: PickerKind,
    pub hint: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerItem {
    /// What choosing the item selects, e.g. an environment name; stable across reloads.
    pub key: String,
    pub label: String,
    pub detail: String,
    pub active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerKind {
    Environment,
    Method,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette {
    pub input: TextInput,
    pub selected: usize,
}

impl Palette {
    pub fn matches(&self) -> Vec<Action> {
        let query = self.input.value();
        let mut scored = Action::ALL
            .iter()
            .enumerate()
            .filter_map(|(order, action)| {
                fuzzy_score(&query, action.label()).map(|score| (score, order, *action))
            })
            .collect::<Vec<_>>();
        scored.sort_by(|left, right| right.0.cmp(&left.0).then(left.1.cmp(&right.1)));
        scored.into_iter().map(|(_, _, action)| action).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextView {
    pub title: String,
    pub text: String,
    pub scroll: usize,
}

impl App {
    pub(super) fn handle_overlay_key(&mut self, key: KeyEvent) -> AppCommand {
        let Some(overlay) = self.overlay.take() else {
            return AppCommand::None;
        };

        match overlay {
            Overlay::Prompt(prompt) => self.handle_prompt_key(prompt, key),
            Overlay::Confirm(confirm) => return self.handle_confirm_key(confirm, key),
            Overlay::Picker(picker) => self.handle_picker_key(picker, key),
            Overlay::Palette(palette) => return self.handle_palette_key(palette, key),
            Overlay::Help { scroll } => {
                if let Some(scroll) = scroll_key(scroll, key) {
                    let scroll = scroll.min(self.overlay_scroll_limit.get());
                    self.overlay = Some(Overlay::Help { scroll });
                } else if !closes_overlay(key) && !matches!(key.code, KeyCode::Char('?') | KeyCode::F(3)) {
                    self.overlay = Some(Overlay::Help { scroll });
                }
            }
            Overlay::TextView(mut view) => match key.code {
                KeyCode::Char('y') => {
                    self.copy_text(&view.text, "command");
                    self.overlay = Some(Overlay::TextView(view));
                }
                _ => {
                    if let Some(scroll) = scroll_key(view.scroll, key) {
                        view.scroll = scroll.min(self.overlay_scroll_limit.get());
                        self.overlay = Some(Overlay::TextView(view));
                    } else if !closes_overlay(key) && key.code != KeyCode::Enter {
                        self.overlay = Some(Overlay::TextView(view));
                    }
                }
            },
        }
        AppCommand::None
    }

    fn handle_prompt_key(&mut self, mut prompt: Prompt, key: KeyEvent) {
        let last_field = prompt.fields.len().saturating_sub(1);
        match key.code {
            KeyCode::Esc => {
                self.on_prompt_cancelled(&prompt.kind);
                return;
            }
            KeyCode::Tab | KeyCode::Down => {
                prompt.active = move_index(prompt.active, prompt.fields.len(), 1)
            }
            KeyCode::BackTab | KeyCode::Up => {
                prompt.active = move_index(prompt.active, prompt.fields.len(), -1)
            }
            KeyCode::Enter if prompt.active < last_field => prompt.active += 1,
            KeyCode::Enter => {
                self.submit_prompt(prompt);
                return;
            }
            _ => {
                if let Some(field) = prompt.fields.get_mut(prompt.active) {
                    if field.input.handle_key(key) {
                        prompt.error = None;
                    }
                }
            }
        }
        self.overlay = Some(Overlay::Prompt(prompt));
    }

    fn on_prompt_cancelled(&mut self, kind: &PromptKind) {
        if matches!(kind, PromptKind::CreateProject) && !self.project.exists {
            self.set_info("No project: you can still send requests. Ctrl+S offers to create one.");
        }
    }

    /// Keeps the prompt open with an inline error, so the user can correct the input.
    pub(super) fn reject_prompt(&mut self, mut prompt: Prompt, error: impl Into<String>) {
        prompt.error = Some(error.into());
        self.overlay = Some(Overlay::Prompt(prompt));
    }

    fn submit_prompt(&mut self, prompt: Prompt) {
        match prompt.kind.clone() {
            PromptKind::CreateProject => self.submit_create_project(prompt),
            PromptKind::SaveRequest { then } => self.submit_save_prompt(prompt, then),
            PromptKind::RenameRequest { old } => self.submit_rename_request(prompt, old),
            PromptKind::DuplicateRequest { source } => self.submit_duplicate_request(prompt, source),
            PromptKind::NewEnvironment => self.submit_new_environment(prompt),
            PromptKind::RenameEnvironment { old } => self.submit_rename_environment(prompt, old),
            PromptKind::EnvVariable { env, old_key } => {
                self.submit_env_variable(prompt, env, old_key)
            }
            PromptKind::Pair { target, index } => self.submit_pair(prompt, target, index),
            PromptKind::Authorization { env } => self.submit_authorization(prompt, env),
            PromptKind::SaveResponse => self.submit_save_response(prompt),
        }
    }

    fn handle_confirm_key(&mut self, confirm: Confirm, key: KeyEvent) -> AppCommand {
        let lower = match key.code {
            KeyCode::Char(c) => Some(c.to_ascii_lowercase()),
            _ => None,
        };
        let cancel = key.code == KeyCode::Esc || lower == Some('n');

        match confirm.kind.clone() {
            ConfirmKind::Discard(pending) => {
                if key.code == KeyCode::Esc {
                    self.set_info("Kept your changes");
                } else if key.code == KeyCode::Enter || lower == Some('s') {
                    self.save_draft(Some(pending));
                } else if lower == Some('d') {
                    self.clean_draft = self.draft.clone();
                    self.run_pending(pending);
                } else {
                    self.overlay = Some(Overlay::Confirm(confirm));
                }
            }
            kind => {
                if cancel {
                    return AppCommand::None;
                }
                if key.code != KeyCode::Enter && lower != Some('y') {
                    self.overlay = Some(Overlay::Confirm(confirm));
                    return AppCommand::None;
                }
                match kind {
                    ConfirmKind::DeleteRequest(name) => self.delete_saved_request(name),
                    ConfirmKind::DeleteEnvironment(name) => self.delete_environment(name),
                    ConfirmKind::DeleteVariable { env, key } => self.delete_env_variable(env, key),
                    ConfirmKind::Overwrite { name, then } => self.write_draft(name, then),
                    ConfirmKind::ImportOpenApi => self.import_openapi(),
                    ConfirmKind::Discard(_) => {}
                }
            }
        }
        AppCommand::None
    }

    fn handle_picker_key(&mut self, mut picker: Picker, key: KeyEvent) {
        let len = picker.items.len();
        match key.code {
            KeyCode::Esc => return,
            KeyCode::Up | KeyCode::Char('k') => picker.selected = move_index(picker.selected, len, -1),
            KeyCode::Down | KeyCode::Char('j') => {
                picker.selected = move_index(picker.selected, len, 1)
            }
            KeyCode::Home => picker.selected = 0,
            KeyCode::End => picker.selected = len.saturating_sub(1),
            KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                let index = c as usize - '1' as usize;
                if index < len {
                    picker.selected = index;
                    self.choose_picker_item(picker);
                    return;
                }
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                self.choose_picker_item(picker);
                return;
            }
            KeyCode::Char('m') if picker.kind == PickerKind::Environment => {
                self.show_screen(super::Screen::Environments);
                return;
            }
            _ => {}
        }
        self.overlay = Some(Overlay::Picker(picker));
    }

    fn choose_picker_item(&mut self, picker: Picker) {
        match picker.kind {
            PickerKind::Environment => {
                if let Some(item) = picker.items.get(picker.selected) {
                    let name = (item.key != NO_ENV).then(|| item.key.clone());
                    self.activate_environment(name);
                }
            }
            PickerKind::Method => {
                if let Some(method) = GemonMethodType::ALL.get(picker.selected) {
                    self.draft.method = *method;
                }
            }
        }
    }

    fn handle_palette_key(&mut self, mut palette: Palette, key: KeyEvent) -> AppCommand {
        let matches = palette.matches();
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Esc => return AppCommand::None,
            KeyCode::Char('p') if ctrl => return AppCommand::None,
            KeyCode::Up => palette.selected = move_index(palette.selected, matches.len(), -1),
            KeyCode::Down | KeyCode::Tab => {
                palette.selected = move_index(palette.selected, matches.len(), 1)
            }
            KeyCode::BackTab => palette.selected = move_index(palette.selected, matches.len(), -1),
            KeyCode::PageUp => palette.selected = step_index(palette.selected, matches.len(), -8),
            KeyCode::PageDown => palette.selected = step_index(palette.selected, matches.len(), 8),
            KeyCode::Enter => {
                return match matches.get(palette.selected) {
                    Some(action) => self.perform(*action),
                    None => {
                        self.overlay = Some(Overlay::Palette(palette));
                        AppCommand::None
                    }
                };
            }
            _ => {
                if palette.input.handle_key(key) {
                    palette.selected = 0;
                }
            }
        }
        self.overlay = Some(Overlay::Palette(palette));
        AppCommand::None
    }

    pub(super) fn open_palette(&mut self) {
        self.overlay = Some(Overlay::Palette(Palette {
            input: TextInput::single(""),
            selected: 0,
        }));
    }

    pub(super) fn open_method_picker(&mut self) {
        let items = GemonMethodType::ALL
            .iter()
            .map(|method| PickerItem {
                key: method.to_string(),
                label: method.to_string(),
                detail: String::new(),
                active: *method == self.draft.method,
            })
            .collect();
        let selected = GemonMethodType::ALL
            .iter()
            .position(|method| *method == self.draft.method)
            .unwrap_or_default();
        self.overlay = Some(Overlay::Picker(Picker {
            title: String::from("Method"),
            items,
            selected,
            kind: PickerKind::Method,
            hint: String::from("Enter select · 1-5 pick · Esc cancel"),
        }));
    }

    pub(super) fn copy_text(&mut self, text: &str, what: &str) {
        match clipboard::copy(text) {
            Ok(mechanism) => self.set_success(format!("Copied {what} ({mechanism})")),
            Err(err) => self.set_error(err),
        }
    }
}

fn closes_overlay(key: KeyEvent) -> bool {
    matches!(key.code, KeyCode::Esc | KeyCode::Char('q'))
}

/// New scroll offset for read-only overlays, or `None` if the key does not scroll.
fn scroll_key(scroll: usize, key: KeyEvent) -> Option<usize> {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => Some(scroll.saturating_sub(1)),
        KeyCode::Down | KeyCode::Char('j') => Some(scroll.saturating_add(1)),
        KeyCode::PageUp => Some(scroll.saturating_sub(10)),
        KeyCode::PageDown | KeyCode::Char(' ') => Some(scroll.saturating_add(10)),
        KeyCode::Home | KeyCode::Char('g') => Some(0),
        _ => None,
    }
}
