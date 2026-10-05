use super::*;
use crate::{
    project::project_handler::{create_project, read_saved_rest_request, request_slot, RequestSlot},
    tui::{layout, ui},
};
use bytes::Bytes;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::{backend::TestBackend, layout::Rect, Terminal};
use std::{
    fs,
    panic::{self, AssertUnwindSafe},
    path::PathBuf,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

/// Project operations use the working directory, which is process-wide.
static CWD_LOCK: Mutex<()> = Mutex::new(());

fn in_temp_project<T>(test: impl FnOnce() -> T) -> T {
    let _guard = CWD_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let dir: PathBuf = std::env::temp_dir().join(format!("gemon_tui_{}_{nanos}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temp project");
    let previous = std::env::current_dir().expect("current dir");
    std::env::set_current_dir(&dir).expect("enter temp project");
    create_project("test").expect("create project");

    let result = panic::catch_unwind(AssertUnwindSafe(test));

    std::env::set_current_dir(previous).expect("restore cwd");
    let _ = fs::remove_dir_all(dir);
    result.unwrap_or_else(|err| panic::resume_unwind(err))
}

fn loaded_app() -> App {
    let mut app = App::detached();
    app.refresh_workspace();
    app
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ctrl(character: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(character), KeyModifiers::CONTROL)
}

fn type_text(app: &mut App, text: &str) {
    for character in text.chars() {
        app.handle_key(key(KeyCode::Char(character)));
    }
}

fn response(status: u16, body: &str) -> GemonResponse {
    let headers = HashMap::from([(String::from("content-type"), String::from("application/json"))]);
    GemonResponse::new(Bytes::from(body.to_string()), status, headers)
}

fn render(app: &App, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test terminal");
    terminal.draw(|frame| ui::draw(frame, app)).expect("draw");
    let buffer = terminal.backend().buffer().clone();
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn with_env(app: &mut App, name: &str, values: &[(&str, &str)]) {
    app.project.exists = true;
    app.project.environments.push(EnvironmentView {
        name: name.to_string(),
        values: values
            .iter()
            .map(|(key, value)| KeyValue {
                key: key.to_string(),
                value: value.to_string(),
            })
            .collect(),
        authorization: None,
    });
    app.project.selected_environment = Some(name.to_string());
}

fn sent_request(command: AppCommand) -> GemonRestRequest {
    match command {
        AppCommand::Send(request) => request,
        other => panic!("expected a request to be sent, got {other:?}"),
    }
}

// Keyboard ----------------------------------------------------------------------------------------

#[test]
fn escape_in_a_text_field_never_switches_screens() {
    let mut app = App::detached();
    app.set_focus(Focus::Url);
    type_text(&mut app, "http://api.test");

    app.handle_key(key(KeyCode::Esc));

    assert_eq!(app.screen, Screen::Requests);
    assert_eq!(app.focus, Focus::Url);
    assert_eq!(app.draft.url.value(), "http://api.test");
}

#[test]
fn tab_visits_every_request_panel_in_order() {
    let mut app = App::detached();
    app.set_focus(Focus::Sidebar);
    let mut visited = vec![app.focus];
    for _ in 0..8 {
        app.handle_key(key(KeyCode::Tab));
        visited.push(app.focus);
    }
    assert_eq!(
        visited,
        vec![
            Focus::Sidebar,
            Focus::Method,
            Focus::Url,
            Focus::Headers,
            Focus::Body,
            Focus::Form,
            Focus::Auth,
            Focus::Response,
            Focus::Sidebar
        ]
    );
    app.handle_key(key(KeyCode::BackTab));
    assert_eq!(app.focus, Focus::Response);
}

#[test]
fn hidden_sidebar_is_skipped_by_tab() {
    let mut app = App::detached();
    app.set_focus(Focus::Response);
    app.perform(Action::ToggleSidebar);
    app.handle_key(key(KeyCode::Tab));
    assert_eq!(app.focus, Focus::Method);
}

#[test]
fn editor_tab_follows_focus() {
    let mut app = App::detached();
    app.set_focus(Focus::Body);
    assert_eq!(app.request_tab, RequestTab::Body);
    app.set_focus(Focus::Url);
    assert_eq!(app.request_tab, RequestTab::Body, "last tab stays visible");
}

#[test]
fn control_digits_do_not_type_into_fields() {
    let mut app = App::detached();
    app.set_focus(Focus::Url);
    for digit in ['1', '5', '9', '0'] {
        app.handle_key(ctrl(digit));
    }
    assert!(app.draft.url.is_empty());
}

#[test]
fn shifted_characters_type_normally() {
    let mut app = App::detached();
    app.set_focus(Focus::Body);
    app.handle_key(KeyEvent::new(KeyCode::Char('{'), KeyModifiers::SHIFT));
    app.handle_key(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT));
    assert_eq!(app.draft.body.value(), "{A");
}

#[test]
fn question_mark_opens_help_only_outside_text_fields() {
    let mut app = App::detached();
    app.set_focus(Focus::Url);
    type_text(&mut app, "?");
    assert!(app.overlay.is_none());
    assert_eq!(app.draft.url.value(), "?");

    app.set_focus(Focus::Response);
    app.handle_key(key(KeyCode::Char('?')));
    assert!(matches!(app.overlay, Some(Overlay::Help { .. })));
    app.handle_key(key(KeyCode::Esc));
    assert!(app.overlay.is_none());
}

#[test]
fn advertised_control_shortcuts_trigger_their_actions() {
    for action in Action::ALL {
        let Some(letter) = action.shortcut().strip_prefix("Ctrl+") else {
            continue;
        };
        let character = letter.chars().next().expect("letter").to_ascii_lowercase();
        let mut app = App::detached();
        assert_eq!(
            app.global_action(ctrl(character)).or_else(|| {
                // Quit is handled before global shortcuts so it also works inside dialogs.
                (character == 'q').then_some(Action::Quit)
            }),
            Some(action),
            "{} is advertised as {}",
            action.label(),
            action.shortcut()
        );
        app.handle_key(ctrl(character));
    }
}

#[test]
fn method_can_be_cycled_and_picked() {
    let mut app = App::detached();
    app.set_focus(Focus::Method);
    app.handle_key(key(KeyCode::Right));
    assert_eq!(app.draft.method, GemonMethodType::Post);

    app.handle_key(ctrl('t'));
    assert!(matches!(app.overlay, Some(Overlay::Picker(_))));
    app.handle_key(key(KeyCode::Char('5')));
    assert_eq!(app.draft.method, GemonMethodType::Patch);
    assert!(app.overlay.is_none());
}

#[test]
fn paste_into_url_drops_line_breaks() {
    let mut app = App::detached();
    app.set_focus(Focus::Url);
    app.handle_paste("http://api.test/users\n");
    assert_eq!(app.draft.url.value(), "http://api.test/users");
}

// Sending -------------------------------------------------------------------------------------

#[test]
fn enter_in_url_sends_with_environment_values() {
    let mut app = App::detached();
    with_env(&mut app, "local", &[("base_uri", "http://localhost:1"), ("token", "abc")]);
    app.set_focus(Focus::Url);
    type_text(&mut app, "{base_uri}/users");
    app.draft.headers.push(KeyValue {
        key: String::from("X-Token"),
        value: String::from("{token}"),
    });

    let request = sent_request(app.handle_key(key(KeyCode::Enter)));

    assert_eq!(request.uri(), "http://localhost:1/users");
    assert_eq!(request.headers().get("X-Token"), Some(&String::from("abc")));
    assert!(app.is_busy());
}

#[test]
fn sending_without_url_explains_why() {
    let mut app = App::detached();
    app.set_focus(Focus::Body);
    assert_eq!(app.handle_key(ctrl('r')), AppCommand::None);
    assert_eq!(app.status.kind, StatusKind::Error);
    assert_eq!(app.focus, Focus::Url);
}

#[test]
fn unresolved_placeholders_are_reported() {
    let mut app = App::detached();
    with_env(&mut app, "local", &[("base_uri", "http://x")]);
    app.draft.url.set_value(String::from("{base_uri}/pets/{petId}"));
    assert_eq!(app.draft.unresolved_placeholders(&app.project.active_values()), vec!["petId"]);

    sent_request(app.perform(Action::Send));
    assert_eq!(app.status.kind, StatusKind::Warning);
    assert!(app.status.message.contains("{petId}"));
}

#[test]
fn json_bodies_are_not_mistaken_for_placeholders() {
    let mut draft = RequestDraft::default();
    draft.body.set_value(String::from("{\"a\": {\"b\": 1}, \"c\": [1, {2}]}"));
    assert!(draft.unresolved_placeholders(&HashMap::new()).is_empty());
}

#[test]
fn response_arrival_moves_focus_to_response() {
    let mut app = App::detached();
    app.set_focus(Focus::Url);
    app.draft.url.set_value(String::from("http://api.test"));
    sent_request(app.handle_key(key(KeyCode::Enter)));

    app.finish_request(Ok(response(201, "{\"id\": 7}")));

    assert_eq!(app.focus, Focus::Response);
    let view = app.response_view().expect("response");
    assert_eq!(view.status, Some(201));
    assert_eq!(view.body_lines, vec!["{", "  \"id\": 7", "}"]);
    assert_eq!(app.status.kind, StatusKind::Success);
}

#[test]
fn response_arrival_does_not_steal_focus_after_user_moved_on() {
    let mut app = App::detached();
    app.draft.url.set_value(String::from("http://api.test"));
    app.set_focus(Focus::Url);
    sent_request(app.perform(Action::Send));
    app.set_focus(Focus::Body);

    app.finish_request(Ok(response(200, "{}")));

    assert_eq!(app.focus, Focus::Body);
}

#[test]
fn escape_cancels_a_running_request() {
    let mut app = App::detached();
    app.draft.url.set_value(String::from("http://api.test"));
    sent_request(app.perform(Action::Send));

    assert_eq!(app.handle_key(key(KeyCode::Esc)), AppCommand::Cancel);
    assert!(matches!(app.response, ResponseState::Cancelled { .. }));

    // A late result for the cancelled request is ignored.
    app.finish_request(Ok(response(200, "{}")));
    assert!(matches!(app.response, ResponseState::Cancelled { .. }));
}

#[test]
fn failed_requests_keep_the_error() {
    let mut app = App::detached();
    app.draft.url.set_value(String::from("http://api.test"));
    sent_request(app.perform(Action::Send));
    app.finish_request(Err(String::from("connection refused")));
    assert!(matches!(&app.response, ResponseState::Failed { message, .. } if message == "connection refused"));
    assert_eq!(app.status.kind, StatusKind::Error);
}

#[test]
fn response_scroll_is_clamped_to_content() {
    let mut app = App::detached();
    app.draft.url.set_value(String::from("http://api.test"));
    sent_request(app.perform(Action::Send));
    let body = format!("[{}]", (0..100).map(|n| n.to_string()).collect::<Vec<_>>().join(","));
    app.finish_request(Ok(response(200, &body)));
    render(&app, 100, 30);

    app.handle_key(key(KeyCode::End));
    let viewport = app.response_viewport.get();
    assert_eq!(app.response_scroll, viewport.total_rows - viewport.height);

    app.handle_key(key(KeyCode::PageDown));
    assert_eq!(app.response_scroll, viewport.total_rows - viewport.height);

    app.handle_key(key(KeyCode::Home));
    assert_eq!(app.response_scroll, 0);
}

#[test]
fn response_search_finds_and_reveals_matches() {
    let mut app = App::detached();
    app.draft.url.set_value(String::from("http://api.test"));
    sent_request(app.perform(Action::Send));
    let items = (0..80)
        .map(|n| format!("{{\"name\": \"item{n}\"}}"))
        .collect::<Vec<_>>()
        .join(",");
    app.finish_request(Ok(response(200, &format!("[{items}]"))));
    render(&app, 100, 30);

    app.handle_key(key(KeyCode::Char('/')));
    type_text(&mut app, "item70");
    let search = app.response_search.as_ref().expect("search");
    assert_eq!(search.matches.len(), 1);
    assert!(app.response_scroll > 0, "scrolled to the match");

    app.handle_key(key(KeyCode::Enter));
    assert!(!app.response_search.as_ref().expect("search").editing);
    app.handle_key(key(KeyCode::Esc));
    assert!(app.response_search.is_none());
}

#[test]
fn response_tabs_switch_with_arrows() {
    let mut app = App::detached();
    app.draft.url.set_value(String::from("http://api.test"));
    sent_request(app.perform(Action::Send));
    app.finish_request(Ok(response(200, "{}")));
    app.handle_key(key(KeyCode::Right));
    assert_eq!(app.response_tab, ResponseTab::Headers);
    assert_eq!(
        app.response_view().expect("view").lines(ResponseTab::Headers),
        ["content-type: application/json"]
    );
}

// Editing -----------------------------------------------------------------------------------------

#[test]
fn headers_are_added_edited_and_removed() {
    let mut app = App::detached();
    app.set_focus(Focus::Headers);

    app.handle_key(key(KeyCode::Char('a')));
    type_text(&mut app, "X-Id");
    app.handle_key(key(KeyCode::Enter));
    type_text(&mut app, "1");
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(
        app.draft.headers,
        vec![KeyValue {
            key: String::from("X-Id"),
            value: String::from("1")
        }]
    );
    assert!(app.overlay.is_none());

    app.handle_key(key(KeyCode::Enter));
    app.handle_key(ctrl('u'));
    type_text(&mut app, "2");
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.draft.headers[0].value, "2");

    app.handle_key(key(KeyCode::Char('d')));
    assert!(app.draft.headers.is_empty());
}

#[test]
fn duplicate_header_names_are_rejected_inline() {
    let mut app = App::detached();
    app.draft.headers.push(KeyValue {
        key: String::from("Accept"),
        value: String::from("a"),
    });
    app.set_focus(Focus::Headers);
    app.handle_key(key(KeyCode::Char('a')));
    type_text(&mut app, "accept");
    app.handle_key(key(KeyCode::Enter));
    app.handle_key(key(KeyCode::Enter));

    match &app.overlay {
        Some(Overlay::Prompt(prompt)) => assert!(prompt.error.as_deref().unwrap_or_default().contains("already")),
        other => panic!("expected prompt with error, got {other:?}"),
    }
}

#[test]
fn auth_tab_toggles_secure() {
    let mut app = App::detached();
    app.set_focus(Focus::Auth);
    app.handle_key(key(KeyCode::Char(' ')));
    assert!(app.draft.secure);
    assert!(app.to_send_is_secure());
}

impl App {
    fn to_send_is_secure(&self) -> bool {
        self.draft.to_request(&HashMap::new()).secure()
    }
}

#[test]
fn format_body_pretty_prints_json() {
    let mut app = App::detached();
    app.draft.body.set_value(String::from("{\"a\":1,\"b\":[1,2]}"));
    app.perform(Action::FormatBody);
    assert_eq!(app.draft.body.value(), "{\n  \"a\": 1,\n  \"b\": [\n    1,\n    2\n  ]\n}");

    app.draft.body.set_value(String::from("{oops"));
    app.perform(Action::FormatBody);
    assert_eq!(app.status.kind, StatusKind::Error);
}

#[test]
fn palette_filters_and_runs_actions() {
    let mut app = App::detached();
    app.draft.url.set_value(String::from("http://api.test"));
    app.handle_key(ctrl('p'));
    type_text(&mut app, "send req");

    let Some(Overlay::Palette(palette)) = &app.overlay else {
        panic!("palette open");
    };
    assert_eq!(palette.matches().first(), Some(&Action::Send));

    let command = app.handle_key(key(KeyCode::Enter));
    assert!(matches!(command, AppCommand::Send(_)));
    assert!(app.overlay.is_none());
}

#[test]
fn quitting_with_unsaved_changes_asks_first() {
    let mut app = App::detached();
    app.draft.url.set_value(String::from("http://api.test"));

    app.handle_key(ctrl('q'));
    assert!(!app.should_quit);
    assert!(matches!(&app.overlay, Some(Overlay::Confirm(confirm)) if matches!(confirm.kind, ConfirmKind::Discard(PendingAction::Quit))));

    app.handle_key(key(KeyCode::Char('d')));
    assert!(app.should_quit);
}

#[test]
fn quitting_a_pristine_draft_is_immediate() {
    let mut app = App::detached();
    app.handle_key(ctrl('c'));
    assert!(app.should_quit);
}

// Project files ---------------------------------------------------------------------------------

#[test]
fn saving_a_new_request_prompts_for_name_and_persists_secure() {
    in_temp_project(|| {
        let mut app = loaded_app();
        app.draft.method = GemonMethodType::Post;
        app.draft.url.set_value(String::from("{base_uri}/users/{id}"));
        app.draft.secure = true;
        app.draft.body.set_value(String::from("{\"name\": \"x\"}"));

        app.handle_key(ctrl('s'));
        let Some(Overlay::Prompt(prompt)) = &app.overlay else {
            panic!("save prompt expected");
        };
        assert_eq!(prompt.value(0), "post_users_id", "suggested from method and path");
        app.handle_key(key(KeyCode::Enter));

        assert!(app.overlay.is_none(), "{:?}", app.status);
        assert_eq!(app.loaded_name.as_deref(), Some("post_users_id"));
        assert!(!app.is_dirty());
        let saved = read_saved_rest_request("post_users_id").expect("saved request");
        assert!(saved.secure());
        assert_eq!(saved.uri(), "{base_uri}/users/{id}");
        assert_eq!(saved.body(), Some("{\"name\": \"x\"}"));
        assert!(app.saved_requests.iter().any(|request| request.name == "post_users_id"
            && request.method == Some(GemonMethodType::Post)));
    });
}

#[test]
fn save_rejects_names_that_escape_or_clobber_the_project() {
    in_temp_project(|| {
        fs::create_dir("src").expect("folder that is not a request");
        let mut app = loaded_app();
        app.draft.url.set_value(String::from("http://api.test"));

        for bad in ["../outside", "a/b", ".hidden", "src"] {
            app.perform(Action::Save);
            app.handle_key(ctrl('u'));
            type_text(&mut app, bad);
            app.handle_key(key(KeyCode::Enter));
            match &app.overlay {
                Some(Overlay::Prompt(prompt)) => assert!(prompt.error.is_some(), "{bad} should be rejected"),
                other => panic!("{bad}: expected prompt with error, got {other:?}"),
            }
            app.handle_key(key(KeyCode::Esc));
        }
        assert!(!PathBuf::from("../outside").exists());
        assert!(!PathBuf::from("src/metadata.json").exists());
    });
}

#[test]
fn saving_over_another_request_asks_to_overwrite() {
    in_temp_project(|| {
        let mut app = loaded_app();
        app.draft.url.set_value(String::from("http://one"));
        app.perform(Action::Save);
        app.handle_key(ctrl('u'));
        type_text(&mut app, "shared");
        app.handle_key(key(KeyCode::Enter));

        app.run_pending(PendingAction::New);
        app.draft.url.set_value(String::from("http://two"));
        app.perform(Action::Save);
        app.handle_key(ctrl('u'));
        type_text(&mut app, "shared");
        app.handle_key(key(KeyCode::Enter));
        assert!(matches!(&app.overlay, Some(Overlay::Confirm(c)) if matches!(c.kind, ConfirmKind::Overwrite { .. })));

        app.handle_key(key(KeyCode::Char('y')));
        assert_eq!(read_saved_rest_request("shared").expect("saved").uri(), "http://two");
    });
}

#[test]
fn opening_another_request_with_unsaved_changes_offers_to_save() {
    in_temp_project(|| {
        let mut app = loaded_app();
        for (name, url) in [("first", "http://one"), ("second", "http://two")] {
            app.run_pending(PendingAction::New);
            app.draft.url.set_value(url.to_string());
            app.perform(Action::Save);
            app.handle_key(ctrl('u'));
            type_text(&mut app, name);
            app.handle_key(key(KeyCode::Enter));
        }
        assert_eq!(app.loaded_name.as_deref(), Some("second"));
        app.draft.url.set_value(String::from("http://two/edited"));

        app.set_focus(Focus::Sidebar);
        app.select_visible_request(0);
        app.handle_key(key(KeyCode::Enter));
        assert!(matches!(&app.overlay, Some(Overlay::Confirm(_))));

        app.handle_key(key(KeyCode::Char('s')));
        assert_eq!(read_saved_rest_request("second").expect("saved").uri(), "http://two/edited");
        assert_eq!(app.loaded_name.as_deref(), Some("first"));
        assert_eq!(app.draft.url.value(), "http://one");
    });
}

#[test]
fn rename_duplicate_and_delete_saved_requests() {
    in_temp_project(|| {
        let mut app = loaded_app();
        app.draft.url.set_value(String::from("http://api.test"));
        app.perform(Action::Save);
        app.handle_key(ctrl('u'));
        type_text(&mut app, "original");
        app.handle_key(key(KeyCode::Enter));
        app.set_focus(Focus::Sidebar);

        app.handle_key(key(KeyCode::Char('r')));
        app.handle_key(ctrl('u'));
        type_text(&mut app, "renamed");
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(request_slot("renamed"), RequestSlot::SavedRequest);
        assert_eq!(request_slot("original"), RequestSlot::Free);
        assert_eq!(app.loaded_name.as_deref(), Some("renamed"));

        app.handle_key(key(KeyCode::Char('c')));
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(request_slot("renamed_copy"), RequestSlot::SavedRequest);
        assert_eq!(app.saved_requests.len(), 2);

        app.select_request_for_test("renamed");
        app.handle_key(key(KeyCode::Char('d')));
        app.handle_key(key(KeyCode::Char('y')));
        assert_eq!(request_slot("renamed"), RequestSlot::Free);
        assert_eq!(app.loaded_name, None);
        assert!(app.is_dirty(), "deleted request stays in the editor as unsaved");
    });
}

impl App {
    fn select_request_for_test(&mut self, name: &str) {
        self.selected_request = self
            .saved_requests
            .iter()
            .position(|request| request.name == name)
            .expect("request exists");
    }
}

#[test]
fn environments_can_be_managed_from_the_keyboard() {
    in_temp_project(|| {
        let mut app = loaded_app();
        app.handle_key(key(KeyCode::F(2)));
        assert_eq!(app.focus, Focus::EnvList);

        app.handle_key(key(KeyCode::Char('n')));
        type_text(&mut app, "local");
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.selected_env_row().name(), "local");

        app.handle_key(key(KeyCode::Char('a')));
        type_text(&mut app, "base_uri");
        app.handle_key(key(KeyCode::Enter));
        type_text(&mut app, "http://localhost:1");
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.focus, Focus::EnvVars);

        app.handle_key(key(KeyCode::Char('u')));
        type_text(&mut app, "Bearer local-token");
        app.handle_key(key(KeyCode::Enter));

        app.handle_key(key(KeyCode::Left));
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.project.selected_environment.as_deref(), Some("local"));
        assert_eq!(app.project.active_authorization(), Some("Bearer local-token"));
        assert_eq!(app.project.active_values().get("base_uri").map(String::as_str), Some("http://localhost:1"));

        // Renaming a variable keeps exactly one copy.
        app.set_focus(Focus::EnvVars);
        app.handle_key(key(KeyCode::Enter));
        app.handle_key(key(KeyCode::BackTab));
        app.handle_key(ctrl('u'));
        type_text(&mut app, "host");
        app.handle_key(key(KeyCode::Enter));
        app.handle_key(key(KeyCode::Enter));
        let keys = app
            .selected_env_values()
            .iter()
            .map(|pair| pair.key.as_str())
            .collect::<Vec<_>>();
        assert_eq!(keys, vec!["host"]);

        // The no-environment row deactivates and owns the default authorization.
        app.set_focus(Focus::EnvList);
        app.handle_key(key(KeyCode::Home));
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.project.selected_environment, None);
        app.handle_key(key(KeyCode::Char('u')));
        type_text(&mut app, "Bearer default");
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.project.active_authorization(), Some("Bearer default"));

        // Renaming moves the authorization; deleting removes it.
        app.handle_key(key(KeyCode::Down));
        app.handle_key(key(KeyCode::Char('r')));
        app.handle_key(ctrl('u'));
        type_text(&mut app, "dev");
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.project.environment("dev").and_then(|env| env.authorization.as_deref()), Some("Bearer local-token"));

        app.handle_key(key(KeyCode::Char('d')));
        app.handle_key(key(KeyCode::Enter));
        assert!(app.project.environments.is_empty());

        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.screen, Screen::Requests);
    });
}

#[test]
fn reloading_keeps_the_highlighted_request_and_environment() {
    in_temp_project(|| {
        let mut app = loaded_app();
        for name in ["m_request", "z_request"] {
            app.run_pending(PendingAction::New);
            app.draft.url.set_value(String::from("http://api.test"));
            app.perform(Action::Save);
            app.handle_key(ctrl('u'));
            type_text(&mut app, name);
            app.handle_key(key(KeyCode::Enter));
        }
        crate::project::project_handler::add_env("m_env").expect("env");
        app.refresh_workspace();
        app.select_request_for_test("m_request");
        app.select_env_row(1);

        // Another tool adds items that sort before the highlighted ones.
        fs::create_dir("a_request").expect("dir");
        for file in [".marker", "metadata.json", "body.json"] {
            fs::copy(PathBuf::from("m_request").join(file), PathBuf::from("a_request").join(file)).expect("copy");
        }
        crate::project::project_handler::add_env("a_env").expect("env");
        app.refresh_workspace();

        assert_eq!(app.highlighted_request().map(|r| r.name.as_str()), Some("m_request"));
        assert_eq!(app.selected_env_row().name(), "m_env");
    });
}

#[test]
fn environment_picker_switches_active_environment() {
    in_temp_project(|| {
        let mut app = loaded_app();
        crate::project::project_handler::add_env("staging").expect("env");
        app.refresh_workspace();

        app.handle_key(ctrl('g'));
        app.handle_key(key(KeyCode::Down));
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.project.selected_environment.as_deref(), Some("staging"));

        app.handle_key(ctrl('g'));
        app.handle_key(key(KeyCode::Char('1')));
        assert_eq!(app.project.selected_environment, None);
    });
}

#[test]
fn responses_can_be_saved_and_reopened() {
    in_temp_project(|| {
        let mut app = loaded_app();
        app.draft.url.set_value(String::from("http://api.test"));
        sent_request(app.perform(Action::Send));
        app.finish_request(Ok(response(200, "{\"ok\":true}")));

        app.handle_key(key(KeyCode::Char('s')));
        app.handle_key(ctrl('u'));
        type_text(&mut app, "out/last.json");
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(fs::read_to_string("out/last.json").expect("file"), "{\n  \"ok\": true\n}");
        assert_eq!(app.project.last_response_path.as_deref(), Some("out/last.json"));

        app.response = ResponseState::Empty;
        app.perform(Action::OpenLastResponse);
        assert_eq!(app.response_view().expect("view").body_lines.len(), 3);
    });
}

#[test]
fn create_project_prompt_creates_gemon_json() {
    let _guard = CWD_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let dir = std::env::temp_dir().join(format!("gemon_tui_new_{}", std::process::id()));
    fs::create_dir_all(&dir).expect("dir");
    let previous = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(&dir).expect("enter");

    let result = panic::catch_unwind(|| {
        let mut app = App::new();
        assert!(matches!(&app.overlay, Some(Overlay::Prompt(p)) if p.kind == PromptKind::CreateProject));
        app.handle_key(key(KeyCode::Enter));
        assert!(app.project.exists);
        assert!(PathBuf::from("gemon.json").exists());
    });

    std::env::set_current_dir(previous).expect("restore");
    let _ = fs::remove_dir_all(dir);
    result.unwrap_or_else(|err| panic::resume_unwind(err));
}

#[test]
fn environment_picker_selects_by_name_even_if_the_list_changed() {
    in_temp_project(|| {
        let mut app = loaded_app();
        crate::project::project_handler::add_env("staging").expect("env");
        app.refresh_workspace();
        app.handle_key(ctrl('g'));
        app.handle_key(key(KeyCode::Down));

        // Another tool adds an environment that sorts first while the picker is open.
        crate::project::project_handler::add_env("alpha").expect("env");
        app.refresh_workspace();
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.project.selected_environment.as_deref(), Some("staging"));
    });
}

#[test]
fn late_responses_do_not_take_focus_from_typing() {
    let mut app = App::detached();
    app.set_focus(Focus::Url);
    type_text(&mut app, "http://api.test");
    sent_request(app.handle_key(key(KeyCode::Enter)));
    type_text(&mut app, "/more");

    app.finish_request(Ok(response(200, "{}")));

    assert_eq!(app.focus, Focus::Url);
    assert_eq!(app.draft.url.value(), "http://api.test/more");
}

#[test]
fn loading_and_saving_keeps_bodies_byte_for_byte() {
    in_temp_project(|| {
        let mut app = loaded_app();
        app.draft.url.set_value(String::from("http://api.test"));
        app.draft.body.set_value(String::from("{\r\n\t\"a\": 1\r\n}"));
        app.perform(Action::Save);
        app.handle_key(ctrl('u'));
        type_text(&mut app, "tabs");
        app.handle_key(key(KeyCode::Enter));

        app.run_pending(PendingAction::New);
        app.guard_unsaved(PendingAction::Load(String::from("tabs")));
        assert!(!app.is_dirty());
        app.draft.headers.push(KeyValue {
            key: String::from("X"),
            value: String::from("1"),
        });
        app.perform(Action::Save);

        assert_eq!(fs::read_to_string("tabs/body.json").expect("body"), "{\r\n\t\"a\": 1\r\n}");
    });
}

#[test]
fn case_only_renames_and_saves_work_on_case_insensitive_file_systems() {
    in_temp_project(|| {
        fs::create_dir("case_probe").expect("probe");
        let case_insensitive = PathBuf::from("CASE_PROBE").exists();
        fs::remove_dir("case_probe").expect("probe");
        if !case_insensitive {
            return;
        }

        let mut app = loaded_app();
        app.draft.url.set_value(String::from("http://one"));
        app.perform(Action::Save);
        app.handle_key(ctrl('u'));
        type_text(&mut app, "users");
        app.handle_key(key(KeyCode::Enter));

        // Saving a new request as "USERS" targets the existing "users" folder.
        app.run_pending(PendingAction::New);
        app.draft.url.set_value(String::from("http://two"));
        app.perform(Action::Save);
        app.handle_key(ctrl('u'));
        type_text(&mut app, "USERS");
        app.handle_key(key(KeyCode::Enter));
        app.handle_key(key(KeyCode::Char('y')));
        assert_eq!(app.loaded_name.as_deref(), Some("users"));
        assert_eq!(app.highlighted_request().map(|r| r.name.as_str()), Some("users"));

        // Renaming only the case is allowed.
        app.set_focus(Focus::Sidebar);
        app.handle_key(key(KeyCode::Char('r')));
        app.handle_key(ctrl('u'));
        type_text(&mut app, "Users");
        app.handle_key(key(KeyCode::Enter));
        assert!(app.overlay.is_none(), "{:?}", app.overlay);
        assert_eq!(app.loaded_name.as_deref(), Some("Users"));
        assert_eq!(app.saved_requests.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(), vec!["Users"]);
    });
}

#[test]
fn deleting_refuses_paths_that_are_not_saved_requests() {
    in_temp_project(|| {
        fs::create_dir("src").expect("dir");
        for name in ["src", "../outside", "."] {
            assert!(crate::project::project_handler::delete_request(&name.to_string()).is_err(), "{name}");
        }
        assert!(PathBuf::from("src").exists());
        assert!(PathBuf::from("gemon.json").exists());
    });
}

// Rendering -------------------------------------------------------------------------------------

#[test]
fn response_body_is_visible_on_a_standard_terminal() {
    let mut app = App::detached();
    app.draft.url.set_value(String::from("http://api.test"));
    sent_request(app.perform(Action::Send));
    app.finish_request(Ok(response(200, "{\"greeting\": \"hello\", \"count\": 3}")));

    for (width, height) in [(80, 24), (120, 40)] {
        let screen = render(&app, width, height);
        assert!(screen.contains("\"greeting\": \"hello\""), "{width}x{height}:\n{screen}");
        assert!(screen.contains("200 OK"), "{width}x{height}:\n{screen}");
    }
}

#[test]
fn rendering_survives_every_size_and_state() {
    let mut app = App::detached();
    app.saved_requests = (0..40)
        .map(|index| SavedRequestInfo {
            name: format!("request_{index}"),
            request_type: String::from("REST"),
            method: GemonMethodType::ALL.get(index % 5).copied(),
        })
        .collect();
    with_env(&mut app, "local", &[("base_uri", "http://localhost")]);
    app.draft.url.set_value(String::from("{base_uri}/a/very/long/path/{missing}?query=ünïcödé"));
    app.draft.body.set_value(String::from("{\n  \"emoji\": \"🙂\",\n  \"tab\":\t1\n}"));
    app.draft.headers.push(KeyValue {
        key: String::from("Authorization"),
        value: String::from("Bearer x"),
    });
    sent_request(app.perform(Action::Send));
    app.finish_request(Ok(response(500, "<html>\u{7}not json\r\n</html>")));

    let overlays: Vec<Option<Overlay>> = vec![
        None,
        Some(Overlay::Help { scroll: 3 }),
        Some(Overlay::Palette(Palette {
            input: TextInput::single("e"),
            selected: 2,
        })),
        Some(Overlay::Prompt(
            Prompt::new("Prompt", PromptKind::NewEnvironment)
                .field("Name", "x", "placeholder")
                .field("Value", "", "placeholder")
                .hint("A hint that is long enough to wrap onto another line in narrow terminals."),
        )),
        Some(Overlay::TextView(TextView {
            title: String::from("cURL"),
            text: String::from("curl -X GET 'http://x' \\\n  -H 'A: b'"),
            scroll: 0,
        })),
    ];

    let mut crowded = Prompt::new("Crowded", PromptKind::NewEnvironment)
        .field("Name", "x".repeat(200), "")
        .field("Value", "", "")
        .hint("hint ".repeat(40));
    crowded.error = Some("error ".repeat(40));
    let overlays = overlays
        .into_iter()
        .chain([Some(Overlay::Prompt(crowded))])
        .collect::<Vec<_>>();

    for focus in [Focus::Sidebar, Focus::Url, Focus::Body, Focus::Auth, Focus::Response, Focus::EnvList, Focus::EnvVars] {
        app.set_focus(focus);
        for overlay in &overlays {
            app.overlay = overlay.clone();
            for (width, height) in [(1, 1), (20, 5), (43, 11), (44, 12), (60, 20), (80, 24), (200, 60)] {
                render(&app, width, height);
            }
        }
    }
    app.overlay = None;
    app.response_zoomed = true;
    app.set_focus(Focus::Response);
    app.response_zoomed = true;
    render(&app, 80, 24);
}

#[test]
fn tiny_terminals_get_a_clear_message() {
    let mut app = App::detached();
    let screen = render(&app, 30, 8);
    assert!(screen.contains("Terminal too small"));

    // A dialog opened while the screen is too small is still explained.
    app.draft.url.set_value(String::from("http://api.test"));
    app.handle_key(ctrl('q'));
    let screen = render(&app, 40, 8);
    assert!(screen.contains("Discard"), "{screen}");
}

#[test]
fn long_confirm_messages_fit_small_terminals() {
    let mut app = App::detached();
    app.overlay = Some(Overlay::Confirm(Confirm {
        title: String::from("Delete request"),
        message: "averyveryverylongword ".repeat(20),
        kind: ConfirmKind::DeleteRequest(String::from("x")),
    }));
    for (width, height) in [(44, 12), (50, 14), (80, 24)] {
        let screen = render(&app, width, height);
        assert!(screen.contains("Cancel"), "{width}x{height}:\n{screen}");
    }
}

#[test]
fn command_view_scrolling_stops_at_the_end() {
    let mut app = App::detached();
    app.overlay = Some(Overlay::Help { scroll: 0 });
    render(&app, 120, 60);
    app.overlay = Some(Overlay::TextView(TextView {
        title: String::from("long"),
        text: (0..200).map(|n| n.to_string()).collect::<Vec<_>>().join("\n"),
        scroll: 0,
    }));
    render(&app, 80, 24);
    for _ in 0..300 {
        app.handle_key(key(KeyCode::Down));
    }
    let Some(Overlay::TextView(view)) = &app.overlay else {
        panic!("view open");
    };
    assert_eq!(view.scroll, app.overlay_scroll_limit.get());
    assert!(view.scroll > 150 && view.scroll < 200, "{}", view.scroll);
}

#[test]
fn footer_shows_hints_for_the_focused_panel() {
    let mut app = App::detached();
    app.set_focus(Focus::Url);
    let screen = render(&app, 120, 30);
    assert!(screen.contains("Enter send"), "{screen}");

    app.set_focus(Focus::Sidebar);
    let screen = render(&app, 120, 30);
    assert!(screen.contains("/ search"), "{screen}");
}

// Mouse -----------------------------------------------------------------------------------------

fn mouse(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

fn app_with_requests(count: usize) -> App {
    let mut app = App::detached();
    app.project.exists = true;
    app.saved_requests = (0..count)
        .map(|index| SavedRequestInfo {
            name: format!("request_{index}"),
            request_type: String::from("REST"),
            method: Some(GemonMethodType::Get),
        })
        .collect();
    app
}

#[test]
fn clicking_selects_requests_and_focuses_panels() {
    let mut app = app_with_requests(3);
    let area = Rect::new(0, 0, 120, 40);
    let body = layout::shell(area).body;
    let panels = app.requests_layout(body);
    let list = App::sidebar_list_area(panels.sidebar);

    app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), list.x + 2, list.y + 1), area);
    assert_eq!(app.focus, Focus::Sidebar);
    assert_eq!(app.selected_request, 1);

    let panels = app.requests_layout(body);
    app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), panels.url.x + 20, panels.url.y + 1), area);
    assert_eq!(app.focus, Focus::Url);

    let panels = app.requests_layout(body);
    let tabs = App::title_tab_rects(panels.editor, &app.request_tab_labels());
    app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), tabs[1].x + 1, tabs[1].y), area);
    assert_eq!(app.focus, Focus::Body);
}

#[test]
fn sidebar_fits_long_names_and_resizes_from_the_keyboard() {
    let mut app = app_with_requests(1);
    app.saved_requests[0].name = String::from("get_project_projectId_ai_settings");
    assert_eq!(app.effective_sidebar_width(), 33 + 11);

    app.set_focus(Focus::Sidebar);
    app.handle_key(key(KeyCode::Char('<')));
    assert_eq!(app.sidebar_width, Some(40));
    app.handle_key(key(KeyCode::Char('>')));
    app.handle_key(key(KeyCode::Char('>')));
    assert_eq!(app.sidebar_width, Some(48));
}

#[test]
fn clicking_header_tabs_switches_screens() {
    let mut app = App::detached();
    let area = Rect::new(0, 0, 120, 40);
    let header = layout::shell(area).header;
    let tabs = app.header_tab_rects(header);
    app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), tabs[1].x + 2, header.y), area);
    assert_eq!(app.screen, Screen::Environments);
}

#[test]
fn dragging_borders_resizes_panels() {
    let mut app = app_with_requests(1);
    let area = Rect::new(0, 0, 120, 40);
    let body = layout::shell(area).body;
    let panels = app.requests_layout(body);
    let border = panels.sidebar.right() - 1;
    let width = app.effective_sidebar_width();

    app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), border, body.y + 5), area);
    app.handle_mouse(mouse(MouseEventKind::Drag(MouseButton::Left), border + 8, body.y + 5), area);
    app.handle_mouse(mouse(MouseEventKind::Up(MouseButton::Left), border + 8, body.y + 5), area);
    assert!(app.sidebar_width.expect("set by dragging") > width);

    let panels = app.requests_layout(body);
    let percent = app.response_percent;
    app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), panels.response.right() - 3, panels.response.y), area);
    app.handle_mouse(mouse(MouseEventKind::Drag(MouseButton::Left), panels.response.right() - 3, panels.response.y - 4), area);
    assert!(app.response_percent > percent);
}

#[test]
fn wheel_scrolls_the_response() {
    let mut app = App::detached();
    app.draft.url.set_value(String::from("http://api.test"));
    sent_request(app.perform(Action::Send));
    let body = format!("[{}]", (0..100).map(|n| n.to_string()).collect::<Vec<_>>().join(","));
    app.finish_request(Ok(response(200, &body)));
    let area = Rect::new(0, 0, 120, 40);
    render(&app, area.width, area.height);
    let panels = app.requests_layout(layout::shell(area).body);

    app.handle_mouse(mouse(MouseEventKind::ScrollDown, panels.response.x + 5, panels.response.y + 3), area);
    assert_eq!(app.response_scroll, 3);
}
