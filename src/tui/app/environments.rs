use super::{
    move_index, App, AppCommand, Confirm, ConfirmKind, EnvironmentView, Focus, KeyValue, Overlay,
    Picker, PickerItem, PickerKind, Prompt, PromptKind,
};
use crate::{
    constants::NO_ENV,
    project::project_handler::{
        add_env, add_env_value, clear_selected_env, remove_env, remove_env_value, rename_env,
        set_authorization_for, set_selected_env, validate_env_name,
    },
};
use crossterm::event::{KeyCode, KeyEvent};

/// A row of the environment list: the implicit "no environment" row comes first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvRow<'a> {
    NoEnvironment,
    Environment(&'a EnvironmentView),
}

impl EnvRow<'_> {
    pub fn name(&self) -> &str {
        match self {
            EnvRow::NoEnvironment => "No environment",
            EnvRow::Environment(env) => &env.name,
        }
    }

    /// Key under which this row's authorization is stored.
    pub fn authorization_key(&self) -> String {
        match self {
            EnvRow::NoEnvironment => NO_ENV.to_string(),
            EnvRow::Environment(env) => env.name.clone(),
        }
    }
}

impl App {
    pub(super) fn handle_env_list_key(&mut self, key: KeyEvent) -> AppCommand {
        let rows = self.env_row_count();
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.select_env_row(move_index(self.selected_env, rows, -1)),
            KeyCode::Down | KeyCode::Char('j') => self.select_env_row(move_index(self.selected_env, rows, 1)),
            KeyCode::Home => self.select_env_row(0),
            KeyCode::End => self.select_env_row(rows.saturating_sub(1)),
            KeyCode::Enter | KeyCode::Char(' ') => self.activate_env_row(self.selected_env),
            KeyCode::Right | KeyCode::Char('l') => self.set_focus(Focus::EnvVars),
            KeyCode::Char('n') => self.open_new_environment_prompt(),
            KeyCode::Char('r') => self.open_rename_environment_prompt(),
            KeyCode::Char('d') | KeyCode::Char('x') | KeyCode::Delete => self.confirm_delete_environment(),
            KeyCode::Char('a') => self.open_env_variable_prompt(None),
            KeyCode::Char('u') => self.open_authorization_prompt_for_selected(),
            _ => {}
        }
        AppCommand::None
    }

    pub(super) fn handle_env_vars_key(&mut self, key: KeyEvent) -> AppCommand {
        let count = self.selected_env_values().len();
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected_env_var = move_index(self.selected_env_var, count, -1)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.selected_env_var = move_index(self.selected_env_var, count, 1)
            }
            KeyCode::Home => self.selected_env_var = 0,
            KeyCode::End => self.selected_env_var = count.saturating_sub(1),
            KeyCode::Left | KeyCode::Char('h') => self.set_focus(Focus::EnvList),
            KeyCode::Char('a') | KeyCode::Char('n') | KeyCode::Insert => {
                self.open_env_variable_prompt(None)
            }
            KeyCode::Enter | KeyCode::Char('e') if count > 0 => {
                self.open_env_variable_prompt(Some(self.selected_env_var))
            }
            KeyCode::Enter => self.open_env_variable_prompt(None),
            KeyCode::Char('d') | KeyCode::Char('x') | KeyCode::Delete => self.confirm_delete_env_variable(),
            KeyCode::Char('u') => self.open_authorization_prompt_for_selected(),
            _ => {}
        }
        AppCommand::None
    }

    pub fn env_rows(&self) -> Vec<EnvRow<'_>> {
        std::iter::once(EnvRow::NoEnvironment)
            .chain(self.project.environments.iter().map(EnvRow::Environment))
            .collect()
    }

    fn env_row_count(&self) -> usize {
        self.project.environments.len() + 1
    }

    pub fn selected_env_row(&self) -> EnvRow<'_> {
        self.env_rows()
            .get(self.selected_env)
            .copied()
            .unwrap_or(EnvRow::NoEnvironment)
    }

    pub fn selected_env_values(&self) -> &[KeyValue] {
        match self.selected_env_row() {
            EnvRow::NoEnvironment => &[],
            EnvRow::Environment(env) => &env.values,
        }
    }

    /// Row of the environment requests currently resolve against.
    pub fn active_env_row(&self) -> usize {
        self.project
            .selected_environment
            .as_deref()
            .and_then(|name| self.project.environments.iter().position(|env| env.name == name))
            .map(|index| index + 1)
            .unwrap_or(0)
    }

    pub fn row_authorization(&self, row: EnvRow<'_>) -> Option<String> {
        match row {
            EnvRow::NoEnvironment => self.project.default_authorization.clone(),
            EnvRow::Environment(env) => env.authorization.clone(),
        }
    }

    pub fn select_env_row(&mut self, row: usize) {
        if row < self.env_row_count() {
            self.selected_env = row;
            self.selected_env_var = 0;
        }
    }

    pub(super) fn clamp_environment_selection(&mut self) {
        self.selected_env = self.selected_env.min(self.env_row_count() - 1);
        self.selected_env_var = self
            .selected_env_var
            .min(self.selected_env_values().len().saturating_sub(1));
    }

    fn select_env_named(&mut self, name: &str) {
        if let Some(index) = self.project.environments.iter().position(|env| env.name == name) {
            self.selected_env = index + 1;
        }
    }

    pub(super) fn open_environment_picker(&mut self) {
        if !self.require_project() {
            return;
        }
        let active = self.active_env_row();
        let items = self
            .env_rows()
            .iter()
            .enumerate()
            .map(|(index, row)| PickerItem {
                key: row.authorization_key(),
                label: row.name().to_string(),
                detail: match row {
                    EnvRow::NoEnvironment => String::from("placeholders stay unresolved"),
                    EnvRow::Environment(env) => plural(env.values.len(), "variable"),
                },
                active: index == active,
            })
            .collect();
        self.overlay = Some(Overlay::Picker(Picker {
            title: String::from("Switch environment"),
            items,
            selected: active,
            kind: PickerKind::Environment,
            hint: String::from("Enter select · 1-9 pick · m manage"),
        }));
    }

    pub(super) fn activate_env_row(&mut self, row: usize) {
        let name = match self.env_rows().get(row) {
            Some(EnvRow::Environment(env)) => Some(env.name.clone()),
            Some(EnvRow::NoEnvironment) => None,
            None => return,
        };
        self.activate_environment(name);
    }

    /// Makes `name` the active environment, or deactivates environments for `None`.
    pub(super) fn activate_environment(&mut self, name: Option<String>) {
        let result = match &name {
            Some(name) => set_selected_env(name),
            None => clear_selected_env(),
        };
        match result {
            Ok(()) => {
                self.refresh_workspace();
                match name {
                    Some(name) => self.set_success(format!("Active environment: {name}")),
                    None => self.set_success("No active environment; placeholders stay unresolved"),
                }
            }
            Err(err) => self.set_error(err.to_string()),
        }
    }

    pub(super) fn open_new_environment_prompt(&mut self) {
        if !self.require_project() {
            return;
        }
        self.overlay = Some(Overlay::Prompt(
            Prompt::new("New environment", PromptKind::NewEnvironment)
                .field("Name", "", "e.g. local, staging, production")
                .hint("Then add variables such as base_uri and use them as {base_uri}."),
        ));
    }

    pub(super) fn submit_new_environment(&mut self, prompt: Prompt) {
        let name = prompt.value(0).trim().to_string();
        if let Err(err) = validate_env_name(&name) {
            return self.reject_prompt(prompt, err);
        }
        match add_env(&name) {
            Ok(()) => {
                self.refresh_workspace();
                self.select_env_named(&name);
                self.set_focus(Focus::EnvList);
                self.set_success(format!(
                    "Created environment '{name}'. Press a to add a variable, Enter to activate."
                ));
            }
            Err(err) => self.reject_prompt(prompt, err.to_string()),
        }
    }

    fn open_rename_environment_prompt(&mut self) {
        let EnvRow::Environment(env) = self.selected_env_row() else {
            self.set_info("Highlight an environment to rename it");
            return;
        };
        let old = env.name.clone();
        self.overlay = Some(Overlay::Prompt(
            Prompt::new(
                "Rename environment",
                PromptKind::RenameEnvironment { old: old.clone() },
            )
            .field("Name", old, "environment name"),
        ));
    }

    pub(super) fn submit_rename_environment(&mut self, prompt: Prompt, old: String) {
        let new = prompt.value(0).trim().to_string();
        if let Err(err) = validate_env_name(&new) {
            return self.reject_prompt(prompt, err);
        }
        match rename_env(&old, &new) {
            Ok(()) => {
                self.refresh_workspace();
                self.select_env_named(&new);
                self.set_success(format!("Renamed environment '{old}' to '{new}'"));
            }
            Err(err) => self.reject_prompt(prompt, err.to_string()),
        }
    }

    fn confirm_delete_environment(&mut self) {
        let EnvRow::Environment(env) = self.selected_env_row() else {
            self.set_info("Highlight an environment to delete it");
            return;
        };
        let name = env.name.clone();
        let count = env.values.len();
        self.overlay = Some(Overlay::Confirm(Confirm {
            title: String::from("Delete environment"),
            message: format!(
                "Delete environment '{name}' with its {count} variable(s) and authorization?"
            ),
            kind: ConfirmKind::DeleteEnvironment(name),
        }));
    }

    pub(super) fn delete_environment(&mut self, name: String) {
        match remove_env(&name) {
            Ok(()) => {
                self.refresh_workspace();
                self.set_success(format!("Deleted environment '{name}'"));
            }
            Err(err) => self.set_error(err.to_string()),
        }
    }

    fn open_env_variable_prompt(&mut self, index: Option<usize>) {
        let EnvRow::Environment(env) = self.selected_env_row() else {
            if self.project.environments.is_empty() {
                self.open_new_environment_prompt();
                self.set_info("Variables live in environments. Create one first.");
            } else {
                self.set_info("Highlight an environment to add variables to it");
            }
            return;
        };
        let env_name = env.name.clone();
        let pair = index
            .and_then(|index| env.values.get(index).cloned())
            .unwrap_or_default();
        let title = if index.is_some() {
            format!("Edit variable in {env_name}")
        } else {
            format!("Add variable to {env_name}")
        };
        let old_key = index.map(|_| pair.key.clone());
        self.overlay = Some(Overlay::Prompt(
            Prompt::new(
                title,
                PromptKind::EnvVariable {
                    env: env_name,
                    old_key,
                },
            )
            .field("Name", pair.key, "e.g. base_uri")
            .field("Value", pair.value, "e.g. http://localhost:8080")
            .hint("Use it in requests as {name}.")
            .focus_field(usize::from(index.is_some())),
        ));
    }

    pub(super) fn submit_env_variable(
        &mut self,
        prompt: Prompt,
        env: String,
        old_key: Option<String>,
    ) {
        let key = prompt.value(0).trim().to_string();
        if key.is_empty() {
            return self.reject_prompt(prompt, "Name is required");
        }
        if key.contains(['{', '}']) {
            return self.reject_prompt(prompt, "Leave out the braces: use base_uri, not {base_uri}");
        }
        let exists = self
            .project
            .environment(&env)
            .is_some_and(|env| env.values.iter().any(|pair| pair.key == key));
        if exists && old_key.as_deref() != Some(key.as_str()) {
            return self.reject_prompt(prompt, format!("'{key}' already exists in {env}"));
        }

        // Write the new key before removing a renamed one, so a failure never loses data.
        if let Err(err) = add_env_value(&env, (key.clone(), prompt.value(1))) {
            return self.reject_prompt(prompt, err.to_string());
        }
        if let Some(old_key) = old_key.filter(|old_key| *old_key != key) {
            if let Err(err) = remove_env_value(&env, &old_key) {
                self.set_error(format!("Saved '{key}' but could not remove '{old_key}': {err}"));
            }
        }

        self.refresh_workspace();
        self.select_env_named(&env);
        self.selected_env_var = self
            .selected_env_values()
            .iter()
            .position(|pair| pair.key == key)
            .unwrap_or_default();
        self.set_focus(Focus::EnvVars);
        self.set_success(format!("Saved {{{key}}} in {env}"));
    }

    fn confirm_delete_env_variable(&mut self) {
        let EnvRow::Environment(env) = self.selected_env_row() else {
            return;
        };
        let Some(pair) = env.values.get(self.selected_env_var) else {
            self.set_info("No variable selected");
            return;
        };
        self.overlay = Some(Overlay::Confirm(Confirm {
            title: String::from("Delete variable"),
            message: format!("Delete {{{}}} from '{}'?", pair.key, env.name),
            kind: ConfirmKind::DeleteVariable {
                env: env.name.clone(),
                key: pair.key.clone(),
            },
        }));
    }

    pub(super) fn delete_env_variable(&mut self, env: String, key: String) {
        match remove_env_value(&env, &key) {
            Ok(()) => {
                self.refresh_workspace();
                self.set_success(format!("Deleted {{{key}}} from '{env}'"));
            }
            Err(err) => self.set_error(err.to_string()),
        }
    }

    fn open_authorization_prompt_for_selected(&mut self) {
        let key = self.selected_env_row().authorization_key();
        self.open_authorization_prompt(key);
    }

    /// Edits the authorization stored for `env` (`NO_ENV` is the default authorization).
    pub(super) fn open_authorization_prompt(&mut self, env: String) {
        if !self.require_project() {
            return;
        }
        let current = if env == NO_ENV {
            self.project.default_authorization.clone()
        } else {
            self.project
                .environment(&env)
                .and_then(|env| env.authorization.clone())
        };
        let target = if env == NO_ENV {
            String::from("requests sent without an environment")
        } else {
            format!("environment '{env}'")
        };
        self.overlay = Some(Overlay::Prompt(
            Prompt::new("Authorization", PromptKind::Authorization { env })
                .field("Value", current.unwrap_or_default(), "e.g. Bearer eyJhbGciOi…")
                .hint(format!(
                    "Sent as the Authorization header of secure requests for {target}. Leave empty to remove."
                )),
        ));
    }

    pub(super) fn submit_authorization(&mut self, prompt: Prompt, env: String) {
        let value = prompt.value(0).trim().to_string();
        match set_authorization_for(&env, &value) {
            Ok(()) => {
                self.refresh_workspace();
                let target = if env == NO_ENV { "no environment" } else { env.as_str() };
                if value.is_empty() {
                    self.set_success(format!("Removed authorization for {target}"));
                } else {
                    self.set_success(format!("Saved authorization for {target}"));
                }
            }
            Err(err) => self.reject_prompt(prompt, err.to_string()),
        }
    }
}

fn plural(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("1 {noun}")
    } else {
        format!("{count} {noun}s")
    }
}
