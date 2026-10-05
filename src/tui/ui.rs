use super::{
    actions::Action,
    app::{
        format_duration, format_size, mask_secret, status_text, App, Confirm, EnvRow, Focus, KeyValue,
        Overlay, Palette, PairTarget, Picker, Prompt, RequestTab, ResponseState, ResponseTab,
        Screen, StatusKind, TextView, BRAND,
    },
    input::{self, TextInput},
    layout::{self, keep_in_view},
    theme,
    viewer::{self, Search},
};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Cell, Clear, Paragraph, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Table, TableState, Wrap,
    },
    Frame,
};
use serde_json::Value;
use unicode_width::UnicodeWidthStr;

const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Where the terminal cursor goes this frame; only the focused text input sets it.
type Cursor = Option<Position>;

pub fn draw(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();
    if layout::too_small(area) {
        draw_too_small(frame, app, area);
        return;
    }

    let shell = layout::shell(area);
    let mut cursor = None;
    draw_header(frame, app, shell.header);
    match app.screen {
        Screen::Requests => draw_requests(frame, app, shell.body, &mut cursor),
        Screen::Environments => draw_environments(frame, app, shell.body),
    }
    draw_footer(frame, app, shell.footer);

    if let Some(overlay) = &app.overlay {
        dim(frame.buffer_mut(), area);
        cursor = None;
        draw_overlay(frame, app, overlay, area, &mut cursor);
    }

    if let Some(position) = cursor {
        frame.set_cursor_position(position);
    }
}

fn draw_too_small(frame: &mut Frame<'_>, app: &App, area: Rect) {
    // Dialogs still take keys while hidden, so say which keys they expect.
    let keys = match &app.overlay {
        Some(Overlay::Confirm(confirm)) => {
            let mut line = vec![Span::styled(format!("{}: ", confirm.title), theme::warning())];
            line.extend(keys_line(&confirm.kind.choices()).spans);
            Line::from(line)
        }
        Some(_) => Line::from(vec![
            Span::styled("Dialog open · ", theme::muted()),
            Span::styled("Esc", theme::key()),
            Span::styled(" closes it", theme::muted()),
        ]),
        None => Line::from(vec![
            Span::styled("Ctrl+Q", theme::key()),
            Span::styled(" quits", theme::muted()),
        ]),
    };
    let message = Paragraph::new(vec![
        Line::from(Span::styled("Terminal too small", theme::heading())),
        Line::from(format!(
            "{}×{} — gemon needs at least {}×{}",
            area.width,
            area.height,
            layout::MIN_WIDTH,
            layout::MIN_HEIGHT
        )),
        keys,
    ])
    .alignment(Alignment::Center)
    .wrap(Wrap { trim: true });
    frame.render_widget(message, area);
}

// Chrome ----------------------------------------------------------------------------------------

fn draw_header(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let mut left = vec![
        Span::styled(
            BRAND,
            Style::new()
                .fg(Color::Black)
                .bg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
    ];
    for (screen, label) in Screen::ALL.iter().zip(app.header_tab_labels()) {
        let style = if *screen == app.screen {
            theme::active_tab(false)
        } else {
            theme::muted()
        };
        left.push(Span::styled(label, style));
        left.push(Span::raw(" "));
    }
    let left = Line::from(left);
    let left_width = left.width() as u16;
    frame.render_widget(Paragraph::new(left), area);

    let available = usize::from(area.width.saturating_sub(left_width + 1));
    let right = header_context(app, available);
    let right_width = right.width() as u16;
    frame.render_widget(
        Paragraph::new(right),
        Rect::new(area.right().saturating_sub(right_width), area.y, right_width, 1),
    );
}

/// Project, environment and auth status, dropping the least important parts to fit.
fn header_context(app: &App, width: usize) -> Line<'static> {
    if app.project_error.is_some() {
        return fit(vec![Span::styled("gemon.json has errors ", theme::danger())], width);
    }
    let Some(name) = &app.project.name else {
        return fit(vec![Span::styled("no project · Ctrl+P → Create project ", theme::warning())], width);
    };

    let env = match &app.project.selected_environment {
        Some(env) => Span::styled(env.clone(), theme::key()),
        None => Span::styled("none", theme::muted()),
    };
    let auth = match app.project.active_authorization() {
        Some(_) => Span::styled("set", theme::success()),
        None => Span::styled("none", theme::muted()),
    };
    let candidates = [
        vec![
            Span::styled(name.clone(), theme::title(false)),
            Span::styled("  │  env ", theme::muted()),
            env.clone(),
            Span::styled("  │  auth ", theme::muted()),
            auth.clone(),
            Span::raw(" "),
        ],
        vec![
            Span::styled("env ", theme::muted()),
            env.clone(),
            Span::styled(" │ auth ", theme::muted()),
            auth,
            Span::raw(" "),
        ],
        vec![Span::styled("env ", theme::muted()), env, Span::raw(" ")],
    ];
    candidates
        .into_iter()
        .map(Line::from)
        .find(|line| line.width() <= width)
        .unwrap_or_default()
}

fn fit(spans: Vec<Span<'static>>, width: usize) -> Line<'static> {
    let line = Line::from(spans);
    if line.width() <= width {
        line
    } else {
        Line::default()
    }
}

fn draw_footer(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let (icon, style) = match app.status.kind {
        StatusKind::Info => ("•", theme::text()),
        StatusKind::Success => ("✓", theme::success()),
        StatusKind::Warning => ("!", theme::warning()),
        StatusKind::Error => ("✗", theme::danger()),
    };
    let max_status = (usize::from(area.width) * 11 / 20).saturating_sub(3);
    let status = vec![
        Span::styled(format!(" {icon} "), style),
        Span::styled(truncate(&app.status.message, max_status), style),
    ];
    let status_width = Line::from(status.clone()).width();

    let available = usize::from(area.width).saturating_sub(status_width + 3);
    let mut hints = Vec::new();
    let mut used = 0;
    for (key, label) in footer_hints(app) {
        let width = key.width() + label.width() + 3;
        if used + width > available {
            break;
        }
        used += width;
        hints.push(Span::styled(key, theme::key()));
        hints.push(Span::styled(format!(" {label}  "), theme::muted()));
    }

    frame.render_widget(
        Paragraph::new(Line::from(hints)).alignment(Alignment::Right),
        area,
    );
    frame.render_widget(
        Paragraph::new(Line::from(status)),
        Rect::new(
            area.x,
            area.y,
            (status_width as u16).min(area.width),
            area.height,
        ),
    );
}

fn footer_hints(app: &App) -> Vec<(&'static str, &'static str)> {
    if let Some(overlay) = &app.overlay {
        return match overlay {
            Overlay::Prompt(prompt) if prompt.fields.len() > 1 => {
                vec![("Enter", "next/confirm"), ("Tab", "switch field"), ("Esc", "cancel")]
            }
            Overlay::Prompt(_) => vec![("Enter", "confirm"), ("Esc", "cancel")],
            Overlay::Confirm(confirm) => confirm.kind.choices(),
            Overlay::Picker(_) => vec![("↑↓", "choose"), ("Enter", "select"), ("Esc", "cancel")],
            Overlay::Palette(_) => vec![("type", "filter"), ("↑↓", "choose"), ("Enter", "run"), ("Esc", "close")],
            Overlay::Help { .. } => vec![("↑↓", "scroll"), ("Esc", "close")],
            Overlay::TextView(_) => vec![("y", "copy"), ("↑↓", "scroll"), ("Esc", "close")],
        };
    }

    let mut hints = match app.focus {
        Focus::Sidebar if app.sidebar_searching => {
            vec![("type", "filter"), ("↑↓", "move"), ("Enter", "open"), ("Esc", "clear")]
        }
        Focus::Sidebar => vec![
            ("Enter", "open"),
            ("/", "search"),
            ("n", "new"),
            ("r", "rename"),
            ("c", "duplicate"),
            ("d", "delete"),
        ],
        Focus::Method => vec![("←→", "change"), ("Enter", "choose"), ("Tab", "URL")],
        Focus::Url => vec![("Enter", "send"), ("^S", "save"), ("^T", "method"), ("^G", "env")],
        Focus::Headers | Focus::Form => vec![("a", "add"), ("Enter", "edit"), ("d", "delete"), ("Tab", "next")],
        Focus::Body => vec![("^R", "send"), ("^S", "save"), ("Tab", "next")],
        Focus::Auth => vec![("Space", "toggle"), ("e", "edit token"), ("^G", "env")],
        Focus::Response
            if app.response_search.as_ref().is_some_and(|search| search.editing) =>
        {
            vec![("type", "search"), ("↑↓", "prev/next"), ("Enter", "done"), ("Esc", "clear")]
        }
        Focus::Response => vec![
            ("↑↓", "scroll"),
            ("←→", "body/headers"),
            ("/", "search"),
            ("y", "copy"),
            ("s", "save"),
            ("z", "zoom"),
            ("^L", "URL"),
        ],
        Focus::EnvList => vec![
            ("Enter", "activate"),
            ("n", "new"),
            ("a", "add var"),
            ("u", "auth"),
            ("r", "rename"),
            ("d", "delete"),
            ("Esc", "back"),
        ],
        Focus::EnvVars => vec![("a", "add"), ("Enter", "edit"), ("d", "delete"), ("u", "auth"), ("Esc", "back")],
    };
    if app.is_busy() {
        hints.insert(0, ("Esc", "cancel"));
    }
    hints.push(("^P", "commands"));
    hints.push(if app.is_typing() { ("F3", "help") } else { ("?", "help") });
    hints
}

// Requests screen -------------------------------------------------------------------------------

fn draw_requests(frame: &mut Frame<'_>, app: &App, area: Rect, cursor: &mut Cursor) {
    let panels = app.requests_layout(area);
    if !panels.sidebar.is_empty() {
        draw_sidebar(frame, app, panels.sidebar, cursor);
    }
    if !panels.url.is_empty() {
        draw_url_bar(frame, app, panels.url, cursor);
    }
    if !panels.editor.is_empty() {
        draw_editor(frame, app, panels.editor, cursor);
    }
    if !panels.response.is_empty() {
        draw_response(frame, app, panels.response, cursor);
    }
}

fn panel(title: Line<'static>, focused: bool) -> Block<'static> {
    Block::bordered()
        .border_type(if focused { BorderType::Thick } else { BorderType::Rounded })
        .border_style(theme::border(focused))
        .title(title)
}

fn draw_sidebar(frame: &mut Frame<'_>, app: &App, area: Rect, cursor: &mut Cursor) {
    let focused = app.focus == Focus::Sidebar;
    let visible = app.visible_request_indices();
    let total = app.saved_requests.len();
    let count = if visible.len() == total {
        format!(" {total} ")
    } else {
        format!(" {}/{total} ", visible.len())
    };
    let title = Line::from(vec![
        Span::styled(" Requests ", theme::title(focused)),
        Span::styled(count, theme::muted()),
    ]);
    frame.render_widget(panel(title, focused), area);

    let inner = layout::inner(area);
    if inner.is_empty() {
        return;
    }

    // Search line.
    let search_area = Rect::new(inner.x, inner.y, inner.width, 1);
    let searching = focused && app.sidebar_searching;
    if searching || !app.sidebar_filter.is_empty() {
        let prefix = Span::styled("/ ", theme::key());
        let field = Rect::new(search_area.x + 2, search_area.y, search_area.width.saturating_sub(2), 1);
        frame.render_widget(Paragraph::new(Line::from(prefix)), search_area);
        render_single_input(frame, field, &app.sidebar_filter, searching, "filter by name or method", cursor);
    } else {
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("/ ", theme::muted()),
                Span::styled("search", theme::placeholder()),
            ])),
            search_area,
        );
    }

    let list = App::sidebar_list_area(area);
    if list.is_empty() {
        return;
    }

    if visible.is_empty() {
        let lines = if app.project_error.is_some() {
            vec![
                Line::from(Span::styled("gemon.json could not be read.", theme::danger())),
                Line::from(""),
                Line::from(Span::styled("Fix it, then", theme::muted())),
                Line::from(vec![Span::styled("Ctrl+P", theme::key()), Span::styled(" → Reload", theme::muted())]),
            ]
        } else if !app.project.exists {
            vec![
                Line::from(Span::styled("No project in this folder.", theme::muted())),
                Line::from(""),
                Line::from(vec![Span::styled("Ctrl+P", theme::key()), Span::styled(" → Create project", theme::muted())]),
            ]
        } else if total == 0 {
            vec![
                Line::from(Span::styled("No saved requests yet.", theme::muted())),
                Line::from(""),
                Line::from(vec![Span::styled("Ctrl+S", theme::key()), Span::styled(" saves the current one", theme::muted())]),
                Line::from(vec![Span::styled("Ctrl+O", theme::key()), Span::styled(" imports openapi.yaml", theme::muted())]),
            ]
        } else {
            vec![
                Line::from(Span::styled("No matches.", theme::muted())),
                Line::from(vec![Span::styled("Esc", theme::key()), Span::styled(" clears the filter", theme::muted())]),
            ]
        };
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), list);
        return;
    }

    let rows = usize::from(list.height);
    let selected = app.selected_visible_position().unwrap_or_default();
    let offset = keep_in_view(app.sidebar_offset.get(), selected, rows).min(visible.len().saturating_sub(1));
    app.sidebar_offset.set(offset);

    let name_width = usize::from(list.width).saturating_sub(9);
    let lines = visible
        .iter()
        .enumerate()
        .skip(offset)
        .take(rows)
        .map(|(position, index)| {
            let request = &app.saved_requests[*index];
            let loaded = app.loaded_name.as_deref() == Some(request.name.as_str());
            let is_selected = position == selected;
            let marker = if loaded { "●" } else { " " };
            let method = App::method_label(request.method);
            let method_style = request.method.map(theme::method).unwrap_or_else(theme::muted);
            let name = truncate(&request.name, name_width);
            if is_selected {
                let style = theme::selected(focused);
                let padded = format!("{marker}{method:<7}{name}");
                let fill = usize::from(list.width).saturating_sub(padded.width());
                Line::from(Span::styled(format!("{padded}{}", " ".repeat(fill)), style))
            } else {
                Line::from(vec![
                    Span::styled(marker, theme::key()),
                    Span::styled(format!("{method:<7}"), method_style),
                    Span::styled(name, if loaded { theme::title(false) } else { theme::text() }),
                ])
            }
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines), list);

    if visible.len() > rows {
        render_scrollbar(frame, area, visible.len(), rows, offset);
    }
}

fn draw_url_bar(frame: &mut Frame<'_>, app: &App, area: Rect, cursor: &mut Cursor) {
    let focused = matches!(app.focus, Focus::Method | Focus::Url);
    let mut title = vec![Span::styled(format!(" {} ", app.draft_title()), theme::title(focused))];
    if app.is_dirty() {
        title.push(Span::styled("● unsaved ", theme::warning()));
    }
    let mut block = panel(Line::from(title), focused);

    let env = app.project.active_values();
    let unresolved = app.draft.unresolved_placeholders(&env);
    let url = app.draft.url.value();
    if !unresolved.is_empty() {
        let names = unresolved
            .iter()
            .map(|name| format!("{{{name}}}"))
            .collect::<Vec<_>>()
            .join(" ");
        block = block.title_bottom(Line::from(Span::styled(
            format!(" ! {names} not set in {} ", app.project.active_environment_label()),
            theme::warning(),
        )));
    } else if url.contains('{') {
        let resolved = super::app::substitute(url.trim(), &env);
        let width = usize::from(area.width).saturating_sub(6);
        block = block.title_bottom(Line::from(Span::styled(
            format!(" → {} ", truncate(&resolved, width)),
            theme::muted(),
        )));
    }
    frame.render_widget(block, area);

    let inner = layout::inner(area);
    if inner.is_empty() {
        return;
    }
    let method = app.draft.method;
    let method_focused = app.focus == Focus::Method;
    let mut badge_style = Style::new()
        .fg(Color::Black)
        .bg(theme::method_color(method))
        .add_modifier(Modifier::BOLD);
    if method_focused {
        badge_style = badge_style.add_modifier(Modifier::UNDERLINED);
    }
    let badge = format!(" {:<6}▾ ", method.as_str());
    let badge_width = badge.width() as u16;
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(badge, badge_style))),
        Rect::new(inner.x, inner.y, badge_width.min(inner.width), 1),
    );
    let field = Rect::new(
        inner.x + badge_width + 1,
        inner.y,
        inner.width.saturating_sub(badge_width + 1),
        1,
    );
    render_single_input(
        frame,
        field,
        &app.draft.url,
        app.focus == Focus::Url,
        "Enter a URL, e.g. {base_uri}/users — Enter sends",
        cursor,
    );
}

fn draw_editor(frame: &mut Frame<'_>, app: &App, area: Rect, cursor: &mut Cursor) {
    let focused = matches!(app.focus, Focus::Headers | Focus::Body | Focus::Form | Focus::Auth);
    let labels = app.request_tab_labels();
    let mut title = Vec::new();
    for (index, (tab, label)) in RequestTab::ALL.iter().zip(labels).enumerate() {
        if index > 0 {
            title.push(Span::styled("│", theme::muted()));
        }
        let style = if *tab == app.request_tab {
            theme::active_tab(focused)
        } else {
            theme::muted()
        };
        title.push(Span::styled(label, style));
    }
    let title = Line::from(title);
    let title_width = title.width();
    let mut block = panel(title, focused);
    if app.request_tab == RequestTab::Body {
        if let Some(note) = body_note(app).filter(|note| fits_beside(title_width, note, area)) {
            block = block.title(note.right_aligned());
        }
    }
    frame.render_widget(block, area);

    let inner = layout::inner(area);
    if inner.is_empty() {
        return;
    }
    match app.request_tab {
        RequestTab::Headers => draw_pairs(frame, app, PairTarget::Header, inner, app.focus == Focus::Headers),
        RequestTab::Form => draw_pairs(frame, app, PairTarget::Form, inner, app.focus == Focus::Form),
        RequestTab::Body => draw_body(frame, &app.draft.body, inner, app.focus == Focus::Body, cursor),
        RequestTab::Auth => draw_auth(frame, app, inner),
    }
}

fn body_note(app: &App) -> Option<Line<'static>> {
    let body = app.draft.body_text()?;
    let trimmed = body.trim_start();
    if !(trimmed.starts_with('{') || trimmed.starts_with('[')) {
        return None;
    }
    let resolved = super::app::substitute(&body, &app.project.active_values());
    Some(match serde_json::from_str::<Value>(&resolved) {
        Ok(_) => Line::from(Span::styled(" valid JSON ", theme::success())),
        Err(_) => Line::from(Span::styled(" invalid JSON ", theme::warning())),
    })
}

fn draw_pairs(frame: &mut Frame<'_>, app: &App, target: PairTarget, area: Rect, focused: bool) {
    let pairs = app.draft.pairs(target);
    if pairs.is_empty() {
        let noun = match target {
            PairTarget::Header => "headers",
            PairTarget::Form => "form fields",
        };
        let mut lines = vec![Line::from(Span::styled(format!("No {noun}."), theme::muted()))];
        if focused {
            lines.push(Line::from(vec![
                Span::styled("a", theme::key()),
                Span::styled(" or ", theme::muted()),
                Span::styled("Enter", theme::key()),
                Span::styled(" adds one.", theme::muted()),
            ]));
        } else {
            lines.push(Line::from(Span::styled(
                "Tab here, then press a to add one.",
                theme::muted(),
            )));
        }
        if target == PairTarget::Header {
            lines.push(Line::from(Span::styled(
                "gemon sends Content-Type and Accept: application/json by default.",
                theme::muted(),
            )));
        }
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), area);
        return;
    }
    draw_key_values(frame, pairs, app.draft.selected_pair(target), focused, area);
}

fn draw_key_values(frame: &mut Frame<'_>, pairs: &[KeyValue], selected: usize, focused: bool, area: Rect) {
    let rows = pairs.iter().map(|pair| {
        Row::new(vec![
            Cell::from(pair.key.clone()),
            Cell::from(pair.value.clone()),
        ])
    });
    let key_width = pairs
        .iter()
        .map(|pair| pair.key.width())
        .max()
        .unwrap_or_default()
        .clamp(8, usize::from(area.width) / 2) as u16;
    let table = Table::new(rows, [Constraint::Length(key_width), Constraint::Fill(1)])
        .header(Row::new(vec!["Name", "Value"]).style(theme::label()))
        .column_spacing(2)
        .highlight_style(theme::selected(focused));
    let visible_rows = usize::from(area.height.saturating_sub(1));
    let mut state = TableState::default()
        .with_offset(layout::follow_offset(selected, visible_rows))
        .with_selected(Some(selected));
    frame.render_stateful_widget(table, area, &mut state);
    if pairs.len() > visible_rows {
        render_scrollbar(frame, area, pairs.len(), visible_rows, state.offset());
    }
}

fn draw_body(frame: &mut Frame<'_>, input: &TextInput, area: Rect, focused: bool, cursor: &mut Cursor) {
    let line_count = input.lines().len();
    let gutter = (line_count.to_string().len() + 1).max(3) as u16;
    let text_area = Rect::new(area.x + gutter, area.y, area.width.saturating_sub(gutter), area.height);
    if text_area.is_empty() {
        return;
    }

    if input.is_empty() && !focused {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled("No body.", theme::muted())),
                Line::from(Span::styled(
                    "Tab here to write one; braces auto-indent. Ctrl+P → Format JSON body tidies it.",
                    theme::muted(),
                )),
            ])
            .wrap(Wrap { trim: true }),
            area,
        );
        return;
    }

    let (row_offset, col_offset) = input.viewport(usize::from(text_area.width), usize::from(text_area.height));
    let (cursor_row, cursor_col) = input.cursor();
    let mut gutter_lines = Vec::new();
    let mut text_lines = Vec::new();
    for (index, line) in input
        .lines()
        .iter()
        .enumerate()
        .skip(row_offset)
        .take(usize::from(text_area.height))
    {
        let number_style = if focused && index == cursor_row { theme::key() } else { theme::muted() };
        gutter_lines.push(Line::from(Span::styled(
            format!("{:>width$} ", index + 1, width = usize::from(gutter) - 1),
            number_style,
        )));
        text_lines.push(Line::from(input::slice_by_width(line, col_offset, usize::from(text_area.width))));
    }
    frame.render_widget(Paragraph::new(gutter_lines), Rect::new(area.x, area.y, gutter, area.height));
    frame.render_widget(Paragraph::new(text_lines), text_area);

    if focused && input.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled("{ \"name\": \"value\" }", theme::placeholder())),
            text_area,
        );
    }

    if focused && cursor_row >= row_offset {
        let line = &input.lines()[cursor_row];
        let x = input::width_of(line.chars().skip(col_offset).take(cursor_col.saturating_sub(col_offset)));
        *cursor = Some(Position::new(
            text_area.x + x as u16,
            text_area.y + (cursor_row - row_offset) as u16,
        ));
    }
}

fn draw_auth(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let focused = app.focus == Focus::Auth;
    let checkbox = if app.draft.secure { "[x]" } else { "[ ]" };
    let checkbox_style = if focused { theme::selected(true) } else { theme::key() };
    let env_label = app.project.active_environment_label().to_string();
    let mut lines = vec![
        Line::from(vec![
            Span::styled(format!(" {checkbox} "), checkbox_style),
            Span::styled(" Send project authorization", theme::title(false)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("     Environment    ", theme::label()),
            Span::raw(env_label.clone()),
        ]),
    ];
    let value_line = match app.project.active_authorization() {
        Some(value) => Line::from(vec![
            Span::styled("     Authorization  ", theme::label()),
            Span::raw(mask_secret(value)),
        ]),
        None => Line::from(vec![
            Span::styled("     Authorization  ", theme::label()),
            Span::styled(format!("not set for {env_label}"), theme::warning()),
        ]),
    };
    lines.push(value_line);
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Space", theme::key()),
        Span::styled(" toggles · ", theme::muted()),
        Span::styled("e", theme::key()),
        Span::styled(" edits the token · ", theme::muted()),
        Span::styled("Ctrl+G", theme::key()),
        Span::styled(" switches environment", theme::muted()),
    ]));
    lines.push(Line::from(Span::styled(
        "An Authorization header you add yourself always wins.",
        theme::muted(),
    )));
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

fn draw_response(frame: &mut Frame<'_>, app: &App, area: Rect, cursor: &mut Cursor) {
    let focused = app.focus == Focus::Response;
    let mut title = vec![];
    for (index, (tab, label)) in [ResponseTab::Body, ResponseTab::Headers]
        .iter()
        .zip(app.response_tab_labels())
        .enumerate()
    {
        if index > 0 {
            title.push(Span::styled("│", theme::muted()));
        }
        let style = if *tab == app.response_tab {
            theme::active_tab(focused)
        } else {
            theme::muted()
        };
        title.push(Span::styled(label, style));
    }
    let title = Line::from(title);
    let title_width = title.width();
    let mut block = panel(title, focused);
    if let Some(summary) = response_summary(app)
        .into_iter()
        .flatten()
        .find(|summary| fits_beside(title_width, summary, area))
    {
        block = block.title(summary.right_aligned());
    }

    let search_line = app.response_search.as_ref().map(|search| {
        let count = if search.matches.is_empty() {
            if search.input.is_empty() { String::new() } else { String::from(" no matches ") }
        } else {
            format!(" {}/{} ", search.current + 1, search.matches.len())
        };
        (search, count)
    });
    if search_line.is_none() {
        if let Some(origin) = response_origin(app) {
            let width = usize::from(area.width).saturating_sub(4);
            block = block.title_bottom(
                Line::from(Span::styled(format!(" {} ", truncate(&origin, width)), theme::muted())).right_aligned(),
            );
        }
    }
    frame.render_widget(block, area);

    let inner = layout::inner(area);
    if inner.is_empty() {
        return;
    }

    if let Some((search, count)) = search_line {
        let y = area.bottom().saturating_sub(1);
        let label = Span::styled(" / ", theme::key());
        frame.render_widget(Paragraph::new(Line::from(label)), Rect::new(area.x + 1, y, 3, 1));
        let count_width = count.width() as u16;
        let field_width = inner.width.saturating_sub(4 + count_width).min(40);
        let field = Rect::new(area.x + 4, y, field_width, 1);
        render_single_input(frame, field, &search.input, focused && search.editing, "search", cursor);
        frame.render_widget(
            Paragraph::new(Span::styled(count, theme::muted())),
            Rect::new(field.right(), y, count_width, 1),
        );
    }

    match &app.response {
        ResponseState::Empty => draw_centered(
            frame,
            inner,
            vec![
                Line::from(Span::styled("No response yet", theme::title(false))),
                Line::from(vec![
                    Span::styled("Enter", theme::key()),
                    Span::styled(" in the URL or ", theme::muted()),
                    Span::styled("Ctrl+R", theme::key()),
                    Span::styled(" anywhere sends the request.", theme::muted()),
                ]),
            ],
        ),
        ResponseState::Loading { started, summary } => draw_centered(
            frame,
            inner,
            vec![
                Line::from(vec![
                    Span::styled(format!("{} ", SPINNER[app.spinner % SPINNER.len()]), theme::key()),
                    Span::styled(format!("Waiting for response… {}", format_duration(started.elapsed())), theme::title(false)),
                ]),
                Line::from(Span::styled(truncate(summary, usize::from(inner.width)), theme::muted())),
                Line::from(vec![Span::styled("Esc", theme::key()), Span::styled(" cancels", theme::muted())]),
            ],
        ),
        ResponseState::Failed { summary, message } => {
            let lines = vec![
                Line::from(Span::styled("Request failed", theme::danger())),
                Line::from(Span::styled(summary.clone(), theme::muted())),
                Line::from(""),
                Line::from(message.clone()),
                Line::from(""),
                Line::from(Span::styled(
                    "Check the URL, the active environment, and that the server is reachable.",
                    theme::muted(),
                )),
            ];
            frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
        }
        ResponseState::Cancelled { summary } => draw_centered(
            frame,
            inner,
            vec![
                Line::from(Span::styled("Request cancelled", theme::title(false))),
                Line::from(Span::styled(truncate(summary, usize::from(inner.width)), theme::muted())),
            ],
        ),
        ResponseState::Ready(view) => {
            let lines = view.lines(app.response_tab);
            let text_area = Rect::new(inner.x, inner.y, inner.width.saturating_sub(1), inner.height);
            let width = usize::from(text_area.width);
            let height = usize::from(text_area.height);
            let total = viewer::total_height(lines, width);
            let scroll = app.response_scroll.min(total.saturating_sub(height));
            app.response_viewport.set(super::app::Viewport {
                total_rows: total,
                height,
                width,
            });

            if lines.iter().all(|line| line.is_empty()) {
                let message = match app.response_tab {
                    ResponseTab::Body => "Empty body",
                    ResponseTab::Headers => "No headers",
                };
                draw_centered(frame, inner, vec![Line::from(Span::styled(message, theme::muted()))]);
                return;
            }

            let query = app
                .response_search
                .as_ref()
                .map(|search| (search.input.value(), search.current_match()));
            let search = query.as_ref().map(|(query, current)| Search {
                query,
                current: *current,
            });
            let rendered = viewer::render_window(
                lines,
                view.content_kind(app.response_tab),
                width,
                scroll,
                height,
                search.as_ref(),
            );
            frame.render_widget(Paragraph::new(rendered), text_area);
            if total > height {
                render_scrollbar(frame, area, total, height, scroll);
            }
        }
    }
}

/// Status for the response title, longest first; the caller picks the first that fits.
fn response_summary(app: &App) -> Option<Vec<Line<'static>>> {
    let single = |line: Line<'static>| Some(vec![line]);
    match &app.response {
        ResponseState::Ready(view) => {
            let status = match view.status {
                Some(status) => Span::styled(format!(" {} ", status_text(status)), theme::status(status)),
                None => Span::styled(" saved file ", theme::muted()),
            };
            let elapsed = view
                .elapsed
                .map(|elapsed| Span::styled(format!("· {} ", format_duration(elapsed)), theme::muted()));
            let size = Span::styled(format!("· {} ", format_size(view.size_bytes)), theme::muted());
            let zoom = app
                .response_zoomed
                .then(|| Span::styled("· z restore ", theme::key()));

            let full = [Some(status.clone()), elapsed.clone(), Some(size), zoom]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            let medium = [Some(status.clone()), elapsed].into_iter().flatten().collect::<Vec<_>>();
            Some(vec![Line::from(full), Line::from(medium), Line::from(status)])
        }
        ResponseState::Loading { .. } => single(Line::from(Span::styled(
            format!(" {} sending ", SPINNER[app.spinner % SPINNER.len()]),
            theme::key(),
        ))),
        ResponseState::Failed { .. } => single(Line::from(Span::styled(" failed ", theme::danger()))),
        ResponseState::Cancelled { .. } => single(Line::from(Span::styled(" cancelled ", theme::muted()))),
        ResponseState::Empty => None,
    }
}

/// Whether a right-aligned title fits next to a left title of `left_width` in `area`.
fn fits_beside(left_width: usize, right: &Line<'_>, area: Rect) -> bool {
    left_width + right.width() + 3 <= usize::from(area.width)
}

fn response_origin(app: &App) -> Option<String> {
    match &app.response {
        ResponseState::Ready(view) => Some(view.origin.clone()),
        _ => None,
    }
}

// Environments screen ---------------------------------------------------------------------------

fn draw_environments(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let panels = layout::environments(area, app.env_list_width);
    draw_env_list(frame, app, panels.list);
    draw_env_vars(frame, app, panels.vars);
    draw_env_auth(frame, app, panels.auth);
}

fn draw_env_list(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let focused = app.focus == Focus::EnvList;
    let title = Line::from(vec![
        Span::styled(" Environments ", theme::title(focused)),
        Span::styled(format!(" {} ", app.project.environments.len()), theme::muted()),
    ]);
    frame.render_widget(panel(title, focused), area);
    let inner = layout::inner(area);
    if inner.is_empty() {
        return;
    }

    let rows = app.env_rows();
    let active = app.active_env_row();
    let height = usize::from(inner.height);
    let offset = layout::follow_offset(app.selected_env, height);
    let lines = rows
        .iter()
        .enumerate()
        .skip(offset)
        .take(height)
        .map(|(index, row)| {
            let marker = if index == active { "● " } else { "  " };
            let tags = {
                let mut tags = Vec::new();
                if app.row_authorization(*row).is_some() {
                    tags.push(String::from("auth"));
                }
                if let EnvRow::Environment(env) = row {
                    tags.push(match env.values.len() {
                        1 => String::from("1 var"),
                        count => format!("{count} vars"),
                    });
                }
                tags.join(" · ")
            };
            let name_width = usize::from(inner.width).saturating_sub(tags.width() + 4);
            let name = truncate(row.name(), name_width);
            let fill = usize::from(inner.width).saturating_sub(marker.width() + name.width() + tags.width() + 1);
            if index == app.selected_env {
                let style = theme::selected(focused);
                Line::from(Span::styled(format!("{marker}{name}{}{tags} ", " ".repeat(fill)), style))
            } else {
                let name_style = match row {
                    EnvRow::NoEnvironment => theme::placeholder(),
                    _ if index == active => theme::title(false),
                    _ => theme::text(),
                };
                Line::from(vec![
                    Span::styled(marker, theme::success()),
                    Span::styled(name, name_style),
                    Span::raw(" ".repeat(fill)),
                    Span::styled(format!("{tags} "), theme::muted()),
                ])
            }
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines), inner);
}

fn draw_env_vars(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let focused = app.focus == Focus::EnvVars;
    let row = app.selected_env_row();
    let mut title = vec![Span::styled(" Variables ", theme::title(focused))];
    if let EnvRow::Environment(env) = row {
        title.push(Span::styled(format!("· {} ", env.name), theme::key()));
        if app.project.selected_environment.as_deref() == Some(env.name.as_str()) {
            title.push(Span::styled("active ", theme::success()));
        }
    }
    frame.render_widget(panel(Line::from(title), focused), area);
    let inner = layout::inner(area);
    if inner.is_empty() {
        return;
    }

    let values = app.selected_env_values();
    if values.is_empty() {
        let lines = match row {
            EnvRow::NoEnvironment => vec![
                Line::from(Span::styled("Without an environment, {placeholders} are sent as typed.", theme::muted())),
                Line::from(""),
                Line::from(vec![
                    Span::styled("n", theme::key()),
                    Span::styled(" creates an environment; ", theme::muted()),
                    Span::styled("u", theme::key()),
                    Span::styled(" sets the authorization used without one.", theme::muted()),
                ]),
            ],
            EnvRow::Environment(_) => vec![
                Line::from(Span::styled("No variables yet.", theme::muted())),
                Line::from(""),
                Line::from(vec![
                    Span::styled("a", theme::key()),
                    Span::styled(" adds one, e.g. base_uri = http://localhost:8080, used as {base_uri}.", theme::muted()),
                ]),
            ],
        };
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);
        return;
    }
    draw_key_values(frame, values, app.selected_env_var, focused, inner);
}

fn draw_env_auth(frame: &mut Frame<'_>, app: &App, area: Rect) {
    if area.height < 3 {
        return;
    }
    let row = app.selected_env_row();
    let block = panel(Line::from(Span::styled(" Authorization ", theme::title(false))), false);
    frame.render_widget(block, area);
    let inner = layout::inner(area);
    let value = match app.row_authorization(row) {
        Some(value) => Line::from(vec![
            Span::raw(mask_secret(&value)),
            Span::styled("  sent with requests that have Auth enabled", theme::muted()),
        ]),
        None => Line::from(Span::styled("Not set", theme::muted())),
    };
    let hint = Line::from(vec![
        Span::styled("u", theme::key()),
        Span::styled(format!(" edits the authorization for {}", row.name()), theme::muted()),
    ]);
    frame.render_widget(Paragraph::new(vec![value, hint]), inner);
}

// Overlays --------------------------------------------------------------------------------------

fn draw_overlay(frame: &mut Frame<'_>, app: &App, overlay: &Overlay, area: Rect, cursor: &mut Cursor) {
    match overlay {
        Overlay::Prompt(prompt) => draw_prompt(frame, prompt, area, cursor),
        Overlay::Confirm(confirm) => draw_confirm(frame, confirm, area),
        Overlay::Picker(picker) => draw_picker(frame, picker, area),
        Overlay::Palette(palette) => draw_palette(frame, palette, area, cursor),
        Overlay::Help { scroll } => draw_help(frame, app, *scroll, area),
        Overlay::TextView(view) => draw_text_view(frame, app, view, area),
    }
}

fn modal(title: &str, border: Style) -> Block<'static> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(border)
        .title(Line::from(Span::styled(format!(" {title} "), border.add_modifier(Modifier::BOLD))))
}

fn draw_prompt(frame: &mut Frame<'_>, prompt: &Prompt, area: Rect, cursor: &mut Cursor) {
    let width = 72.min(area.width.saturating_sub(4));
    let text_width = usize::from(width.saturating_sub(4));
    let hint_lines = prompt
        .hint
        .as_ref()
        .map(|hint| wrapped_height(hint, text_width))
        .unwrap_or_default();
    let error_lines = prompt
        .error
        .as_ref()
        .map(|error| wrapped_height(error, text_width))
        .unwrap_or_default();
    let height = 2 + 3 * prompt.fields.len() as u16 + hint_lines + error_lines + 1;
    let rect = layout::centered(area, width, height);
    frame.render_widget(Clear, rect);
    frame.render_widget(modal(&prompt.title, Style::new().fg(theme::ACCENT)), rect);

    let inner = Rect::new(rect.x + 2, rect.y + 1, rect.width.saturating_sub(4), rect.height.saturating_sub(2));
    let clip = |area: Rect| area.intersection(inner);
    let mut y = inner.y;
    for (index, field) in prompt.fields.iter().enumerate() {
        let active = index == prompt.active;
        let field_rect = clip(Rect::new(inner.x, y, inner.width, 3));
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(theme::border(active))
            .title(Span::styled(format!(" {} ", field.label), theme::title(active)));
        frame.render_widget(block, field_rect);
        let input_rect = clip(Rect::new(field_rect.x + 1, field_rect.y + 1, field_rect.width.saturating_sub(2), 1));
        render_single_input(frame, input_rect, &field.input, active, field.placeholder, cursor);
        y += 3;
    }

    // The error matters more than the hint when space is short.
    if let Some(error) = &prompt.error {
        frame.render_widget(
            Paragraph::new(Span::styled(format!("✗ {error}"), theme::danger())).wrap(Wrap { trim: true }),
            clip(Rect::new(inner.x, y, inner.width, error_lines)),
        );
        y += error_lines;
    }
    if let Some(hint) = &prompt.hint {
        frame.render_widget(
            Paragraph::new(Span::styled(hint.clone(), theme::muted())).wrap(Wrap { trim: true }),
            clip(Rect::new(inner.x, y, inner.width, hint_lines)),
        );
        y += hint_lines;
    }
    let keys = if prompt.fields.len() > 1 {
        keys_line(&[("Enter", "next / confirm"), ("Tab", "switch field"), ("Esc", "cancel")])
    } else {
        keys_line(&[("Enter", "confirm"), ("Esc", "cancel")])
    };
    frame.render_widget(Paragraph::new(keys), clip(Rect::new(inner.x, y, inner.width, 1)));
}

fn draw_confirm(frame: &mut Frame<'_>, confirm: &Confirm, area: Rect) {
    let width = 64.min(area.width.saturating_sub(4));
    let text_width = usize::from(width.saturating_sub(4));
    let message_lines = wrapped_height(&confirm.message, text_width);
    let rect = layout::centered(area, width, message_lines + 5);
    let border = if confirm.kind.is_destructive() {
        Style::new().fg(theme::DANGER)
    } else {
        Style::new().fg(theme::ACCENT)
    };
    frame.render_widget(Clear, rect);
    frame.render_widget(modal(&confirm.title, border), rect);
    let inner = Rect::new(rect.x + 2, rect.y + 1, rect.width.saturating_sub(4), rect.height.saturating_sub(2));
    // Long names can need more lines than fit; the key line always stays visible.
    let message_area = Rect::new(inner.x, inner.y + 1, inner.width, message_lines)
        .intersection(Rect::new(inner.x, inner.y, inner.width, inner.height.saturating_sub(2)));
    frame.render_widget(
        Paragraph::new(confirm.message.clone()).wrap(Wrap { trim: true }),
        message_area,
    );
    frame.render_widget(
        Paragraph::new(keys_line(&confirm.kind.choices())),
        Rect::new(inner.x, inner.bottom().saturating_sub(1), inner.width, 1),
    );
}

fn draw_picker(frame: &mut Frame<'_>, picker: &Picker, area: Rect) {
    let width = 56.min(area.width.saturating_sub(4));
    let list_height = (picker.items.len() as u16).min(area.height.saturating_sub(8)).max(1);
    let rect = layout::centered(area, width, list_height + 4);
    frame.render_widget(Clear, rect);
    frame.render_widget(modal(&picker.title, Style::new().fg(theme::ACCENT)), rect);
    let inner = layout::inner(rect);
    let list = Rect::new(inner.x, inner.y, inner.width, list_height);
    let offset = layout::follow_offset(picker.selected, usize::from(list_height));

    let lines = picker
        .items
        .iter()
        .enumerate()
        .skip(offset)
        .take(usize::from(list_height))
        .map(|(index, item)| {
            let number = if index < 9 { format!(" {} ", index + 1) } else { String::from("   ") };
            let marker = if item.active { "● " } else { "  " };
            let label_width = usize::from(inner.width).saturating_sub(number.width() + marker.width() + item.detail.width() + 2);
            let label = truncate(&item.label, label_width);
            let fill = usize::from(inner.width)
                .saturating_sub(number.width() + marker.width() + label.width() + item.detail.width() + 1);
            if index == picker.selected {
                Line::from(Span::styled(
                    format!("{number}{marker}{label}{}{} ", " ".repeat(fill), item.detail),
                    theme::selected(true),
                ))
            } else {
                Line::from(vec![
                    Span::styled(number, theme::muted()),
                    Span::styled(marker, theme::success()),
                    Span::raw(label),
                    Span::raw(" ".repeat(fill)),
                    Span::styled(format!("{} ", item.detail), theme::muted()),
                ])
            }
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines), list);
    frame.render_widget(
        Paragraph::new(Span::styled(picker.hint.clone(), theme::muted())),
        Rect::new(inner.x + 1, inner.bottom().saturating_sub(1), inner.width.saturating_sub(1), 1),
    );
}

fn draw_palette(frame: &mut Frame<'_>, palette: &Palette, area: Rect, cursor: &mut Cursor) {
    let matches = palette.matches();
    let width = 72.min(area.width.saturating_sub(4));
    let list_height = (matches.len().max(1) as u16).min(area.height.saturating_sub(8)).min(16);
    let height = list_height + 4;
    let rect = Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height.saturating_sub(height) / 4).max(1),
        width,
        height.min(area.height),
    );
    frame.render_widget(Clear, rect);
    frame.render_widget(modal("Commands", Style::new().fg(theme::ACCENT)), rect);
    let inner = layout::inner(rect);

    frame.render_widget(
        Paragraph::new(Span::styled("› ", theme::key())),
        Rect::new(inner.x + 1, inner.y, 2, 1),
    );
    render_single_input(
        frame,
        Rect::new(inner.x + 3, inner.y, inner.width.saturating_sub(4), 1),
        &palette.input,
        true,
        "type to search commands",
        cursor,
    );
    frame.render_widget(
        Paragraph::new(Span::styled("─".repeat(usize::from(inner.width)), theme::muted())),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );

    let list = Rect::new(inner.x, inner.y + 2, inner.width, list_height);
    if matches.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled("  No matching command", theme::muted())),
            list,
        );
        return;
    }
    let offset = layout::follow_offset(palette.selected, usize::from(list_height));
    let lines = matches
        .iter()
        .enumerate()
        .skip(offset)
        .take(usize::from(list_height))
        .map(|(index, action)| palette_line(*action, index == palette.selected, usize::from(inner.width)))
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines), list);
}

fn palette_line(action: Action, selected: bool, width: usize) -> Line<'static> {
    let label = format!("  {}", action.label());
    let shortcut = format!("{}  ", action.shortcut());
    let fill = width.saturating_sub(label.width() + shortcut.width());
    if selected {
        Line::from(Span::styled(
            format!("{label}{}{shortcut}", " ".repeat(fill)),
            theme::selected(true),
        ))
    } else {
        Line::from(vec![
            Span::raw(label),
            Span::raw(" ".repeat(fill)),
            Span::styled(shortcut, theme::muted()),
        ])
    }
}

fn draw_help(frame: &mut Frame<'_>, app: &App, scroll: usize, area: Rect) {
    let width = 92.min(area.width.saturating_sub(2));
    let height = area.height.saturating_sub(2);
    let rect = layout::centered(area, width, height);
    frame.render_widget(Clear, rect);
    let title = format!("Keyboard shortcuts · gemon {}", env!("CARGO_PKG_VERSION"));
    frame.render_widget(modal(&title, Style::new().fg(theme::ACCENT)), rect);
    let inner = Rect::new(rect.x + 2, rect.y + 1, rect.width.saturating_sub(4), rect.height.saturating_sub(2));

    let lines = help_lines(usize::from(inner.width));
    let max_scroll = lines.len().saturating_sub(usize::from(inner.height));
    let scroll = scroll.min(max_scroll);
    app.overlay_scroll_limit.set(max_scroll);
    frame.render_widget(Paragraph::new(lines).scroll((scroll as u16, 0)), inner);
    if max_scroll > 0 {
        render_scrollbar(frame, rect, max_scroll + usize::from(inner.height), usize::from(inner.height), scroll);
    }
}

fn help_lines(width: usize) -> Vec<Line<'static>> {
    let sections: [(&str, &[(&str, &str)]); 7] = [
        (
            "Everywhere",
            &[
                ("Tab / Shift+Tab", "Move between panels: requests, method, URL, headers, body, form, auth, response"),
                ("Ctrl+P", "Command palette: every action, searchable"),
                ("Ctrl+R", "Send the request (Ctrl+Enter too where supported)"),
                ("Ctrl+S", "Save the request; asks for a name the first time"),
                ("Ctrl+N", "New request"),
                ("Ctrl+L", "Edit the URL"),
                ("Ctrl+F", "Search saved requests"),
                ("Ctrl+T", "Choose the HTTP method"),
                ("Ctrl+G", "Switch environment"),
                ("Ctrl+O", "Import openapi.yaml files"),
                ("F1 / F2", "Requests / Environments screen"),
                ("F3 or ?", "This help (? when not typing)"),
                ("Esc", "Close dialogs, clear searches, cancel a running request, go back"),
                ("Ctrl+Q / Ctrl+C", "Quit (asks about unsaved changes)"),
            ],
        ),
        (
            "Text fields",
            &[
                ("←→ Home End", "Move; Alt/Ctrl+←→ jump words"),
                ("Ctrl+A / Ctrl+E", "Start / end of line"),
                ("Ctrl+W / Alt+Backspace", "Delete previous word"),
                ("Ctrl+U / Ctrl+K", "Delete to start / end of line"),
                ("Paste", "Bracketed paste; newlines are kept in the body only"),
            ],
        ),
        (
            "Request list",
            &[
                ("↑↓ PgUp PgDn", "Move (j/k, g/G work too)"),
                ("Enter", "Open the highlighted request"),
                ("/", "Filter by name or method"),
                ("n r c d", "New, rename, duplicate, delete"),
                ("< >", "Narrow / widen the list"),
            ],
        ),
        (
            "Request editor",
            &[
                ("Enter (URL)", "Send the request"),
                ("←→ Space (method)", "Change method; Enter opens the list"),
                ("a Enter d (headers/form)", "Add, edit, delete a row"),
                ("Body", "Enter keeps indentation; braces indent automatically"),
                ("Space (auth)", "Toggle sending the environment's authorization; e edits it"),
                ("{name}", "Environment placeholders, resolved when sending"),
            ],
        ),
        (
            "Response",
            &[
                ("↑↓ PgUp PgDn g G", "Scroll"),
                ("←→", "Switch between body and headers"),
                ("/  n  N", "Search, next match, previous match"),
                ("y / s / S", "Copy to clipboard / save to file / save with timestamp"),
                ("z  + -", "Full screen, grow, shrink"),
            ],
        ),
        (
            "Environments (F2)",
            &[
                ("Enter", "Activate the highlighted environment"),
                ("n r d", "New, rename, delete environment"),
                ("a Enter d", "Add, edit, delete variables"),
                ("u", "Edit the authorization of the highlighted environment"),
            ],
        ),
        (
            "Mouse",
            &[
                ("Click / wheel", "Focus, select, scroll; click a selected request to open it"),
                ("Drag borders", "Resize the request list and the response panel"),
                ("Shift+drag", "Select text in most terminals (Option+drag in macOS Terminal)"),
            ],
        ),
    ];

    let key_width = 26.min(width / 2);
    let mut lines = Vec::new();
    for (index, (title, rows)) in sections.iter().enumerate() {
        if index > 0 {
            lines.push(Line::from(""));
        }
        lines.push(Line::from(Span::styled(title.to_string(), theme::heading())));
        for (keys, description) in rows.iter() {
            let description_width = width.saturating_sub(key_width + 1).max(10);
            let mut first = true;
            for chunk in wrap_words(description, description_width) {
                let key = if first { keys.to_string() } else { String::new() };
                first = false;
                lines.push(Line::from(vec![
                    Span::styled(format!("{key:<key_width$} "), theme::key()),
                    Span::raw(chunk),
                ]));
            }
        }
    }
    lines
}

fn draw_text_view(frame: &mut Frame<'_>, app: &App, view: &TextView, area: Rect) {
    let width = 100.min(area.width.saturating_sub(4));
    let text_width = usize::from(width.saturating_sub(4));
    let content_height = view
        .text
        .lines()
        .map(|line| viewer::visual_height(line, text_width))
        .sum::<usize>();
    let height = (content_height + 4).min(usize::from(area.height.saturating_sub(2))) as u16;
    let rect = layout::centered(area, width, height);
    frame.render_widget(Clear, rect);
    frame.render_widget(modal(&view.title, Style::new().fg(theme::ACCENT)), rect);
    let inner = Rect::new(rect.x + 2, rect.y + 1, rect.width.saturating_sub(4), rect.height.saturating_sub(2));
    let body_height = inner.height.saturating_sub(2);
    let max_scroll = content_height.saturating_sub(usize::from(body_height));
    app.overlay_scroll_limit.set(max_scroll);
    let scroll = view.scroll.min(max_scroll).min(usize::from(u16::MAX)) as u16;
    frame.render_widget(
        Paragraph::new(view.text.clone())
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0)),
        Rect::new(inner.x, inner.y, inner.width, body_height),
    );
    if max_scroll > 0 {
        render_scrollbar(frame, rect, content_height, usize::from(body_height), usize::from(scroll));
    }
    frame.render_widget(
        Paragraph::new(keys_line(&[("y", "copy"), ("↑↓", "scroll"), ("Esc", "close")])),
        Rect::new(inner.x, inner.bottom().saturating_sub(1), inner.width, 1),
    );
}

// Helpers ---------------------------------------------------------------------------------------

/// Renders a one-line input with horizontal scrolling; sets the cursor when focused.
fn render_single_input(
    frame: &mut Frame<'_>,
    area: Rect,
    input: &TextInput,
    focused: bool,
    placeholder: &str,
    cursor: &mut Cursor,
) {
    if area.is_empty() {
        return;
    }
    if input.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled(placeholder.to_string(), theme::placeholder())),
            area,
        );
        if focused {
            *cursor = Some(Position::new(area.x, area.y));
        }
        return;
    }
    let (visible, cursor_x) = input.single_line_view(usize::from(area.width));
    frame.render_widget(Paragraph::new(visible), area);
    if focused {
        *cursor = Some(Position::new(area.x + cursor_x as u16, area.y));
    }
}

fn render_scrollbar(frame: &mut Frame<'_>, area: Rect, total: usize, viewport: usize, position: usize) {
    let mut state = ScrollbarState::new(total.saturating_sub(viewport) + 1)
        .viewport_content_length(viewport)
        .position(position);
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(None)
        .end_symbol(None)
        .track_symbol(None)
        .thumb_symbol("█")
        .thumb_style(theme::muted());
    frame.render_stateful_widget(
        scrollbar,
        Rect::new(area.x, area.y + 1, area.width, area.height.saturating_sub(2)),
        &mut state,
    );
}

fn draw_centered(frame: &mut Frame<'_>, area: Rect, lines: Vec<Line<'static>>) {
    let height = (lines.len() as u16).min(area.height);
    let rect = Rect::new(area.x, area.y + (area.height - height) / 2, area.width, height);
    frame.render_widget(
        Paragraph::new(lines).alignment(Alignment::Center).wrap(Wrap { trim: true }),
        rect,
    );
}

fn keys_line(keys: &[(&str, &str)]) -> Line<'static> {
    let mut spans = Vec::new();
    for (index, (key, label)) in keys.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled("   ", theme::muted()));
        }
        spans.push(Span::styled(key.to_string(), theme::key()));
        spans.push(Span::styled(format!(" {label}"), theme::muted()));
    }
    Line::from(spans)
}

/// Fades everything already drawn, so an overlay stands out.
fn dim(buffer: &mut Buffer, area: Rect) {
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            if let Some(cell) = buffer.cell_mut((x, y)) {
                cell.set_style(Style::reset().fg(theme::MUTED));
            }
        }
    }
}

fn truncate(text: &str, width: usize) -> String {
    if text.width() <= width {
        return text.to_string();
    }
    if width == 0 {
        return String::new();
    }
    let mut result = String::new();
    let mut used = 0;
    for character in text.chars() {
        let character_width = input::char_width(character);
        if used + character_width + 1 > width {
            break;
        }
        used += character_width;
        result.push(character);
    }
    result.push('…');
    result
}

fn wrap_words(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if !current.is_empty() && current.width() + 1 + word.width() > width {
            lines.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() || lines.is_empty() {
        lines.push(current);
    }
    lines
}

fn wrapped_height(text: &str, width: usize) -> u16 {
    wrap_words(text, width.max(1)).len() as u16
}

#[cfg(test)]
mod tests {
    use super::{truncate, wrap_words};

    #[test]
    fn truncate_adds_ellipsis_within_width() {
        assert_eq!(truncate("abcdef", 10), "abcdef");
        assert_eq!(truncate("abcdef", 4), "abc…");
    }

    #[test]
    fn wrap_words_respects_width() {
        assert_eq!(wrap_words("one two three", 7), vec!["one two", "three"]);
    }
}
