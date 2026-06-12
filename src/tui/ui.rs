use super::{
    app::{App, EnvField, Focus, KeyValue, Modal, PairField, RequestDraft, StatusKind, Tab},
    input::TextInput,
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row, Table, Tabs, Wrap,
    },
    Frame,
};

pub fn draw(frame: &mut Frame<'_>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(frame.area());

    draw_header(frame, app, chunks[0]);
    match app.active_tab {
        Tab::Requests => draw_requests(frame, app, chunks[1]),
        Tab::Environments => draw_environments(frame, app, chunks[1]),
        Tab::Help => draw_help(frame, chunks[1]),
    }
    draw_footer(frame, app, chunks[2]);

    if let Some(modal) = &app.modal {
        draw_modal(frame, app, modal);
    }
}

fn draw_header(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let titles = [Tab::Requests, Tab::Environments, Tab::Help]
        .iter()
        .map(|tab| Line::from(Span::styled(tab.title(), Style::default().fg(Color::White))))
        .collect::<Vec<_>>();

    let selected = match app.active_tab {
        Tab::Requests => 0,
        Tab::Environments => 1,
        Tab::Help => 2,
    };

    let project = app
        .project
        .name
        .as_deref()
        .map(|name| format!("Project: {name}"))
        .unwrap_or_else(|| String::from("Project: not initialized"));
    let env = app
        .project
        .selected_environment
        .as_deref()
        .map(|name| format!("Env: {name}"))
        .unwrap_or_else(|| String::from("Env: default"));
    let auth = if app.project.default_authorization_set {
        "Default auth: set"
    } else {
        "Default auth: empty"
    };

    let header = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    let tabs = Tabs::new(titles)
        .select(selected)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Gemon  F1 Requests | F2 Envs | F3 Help"),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(tabs, header[0]);

    let context = Paragraph::new(Line::from(vec![
        Span::styled(project, Style::default().fg(Color::White)),
        Span::raw("  "),
        Span::styled(env, Style::default().fg(Color::Cyan)),
        Span::raw("  "),
        Span::styled(auth, Style::default().fg(Color::Yellow)),
    ]))
    .alignment(Alignment::Right)
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(context, header[1]);
}

fn draw_requests(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(32), Constraint::Min(0)])
        .split(area);

    draw_saved_requests(frame, app, chunks[0]);
    draw_request_workspace(frame, app, chunks[1]);
}

fn draw_saved_requests(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let items = if app.saved_requests.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "No saved requests",
            Style::default().fg(Color::DarkGray),
        )))]
    } else {
        app.saved_requests
            .iter()
            .map(|request| {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        request.name.clone(),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        request.request_type.clone(),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]))
            })
            .collect()
    };

    let mut state = ListState::default();
    if !app.saved_requests.is_empty() {
        state.select(Some(app.selected_request));
    }

    let list = List::new(items)
        .block(focused_block(
            "Saved Requests  Ctrl-1",
            app.active_tab == Tab::Requests && app.focus == Focus::SavedRequests,
        ))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_request_workspace(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),
            Constraint::Length(9),
            Constraint::Percentage(42),
            Constraint::Percentage(58),
        ])
        .split(area);

    draw_composer(frame, app, chunks[0]);

    let pairs = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);
    draw_pair_table(
        frame,
        "Headers  Ctrl-5",
        &app.draft.headers,
        app.draft.selected_header,
        app.focus == Focus::Headers,
        pairs[0],
    );
    draw_pair_table(
        frame,
        "Form Data  Ctrl-6",
        &app.draft.form_data,
        app.draft.selected_form_data,
        app.focus == Focus::FormData,
        pairs[1],
    );

    draw_body(frame, &app.draft, app.focus == Focus::Body, chunks[2]);
    draw_response(frame, app, chunks[3]);
}

fn draw_composer(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let draft = &app.draft;
    let secure = if draft.secure { "on" } else { "off" };
    let lines = vec![
        Line::from(vec![
            Span::styled("Method ", label_style(app.focus == Focus::Method)),
            Span::styled(
                format!("[{}]", draft.method),
                value_style(app.focus == Focus::Method),
            ),
            Span::raw("  "),
            Span::styled("Secure ", label_style(app.focus == Focus::Secure)),
            Span::styled(
                format!("[{secure}]"),
                value_style(app.focus == Focus::Secure),
            ),
        ]),
        Line::from(vec![
            Span::styled("URI  ", label_style(app.focus == Focus::Url)),
            Span::styled(
                display_single_input(&draft.url, app.focus == Focus::Url),
                value_style(app.focus == Focus::Url),
            ),
        ]),
        Line::from(vec![
            Span::styled("Name ", label_style(app.focus == Focus::RequestName)),
            Span::styled(
                display_single_input(&draft.name, app.focus == Focus::RequestName),
                value_style(app.focus == Focus::RequestName),
            ),
        ]),
        Line::from(vec![
            Span::styled("CLI  ", Style::default().fg(Color::DarkGray)),
            Span::styled(draft.command_preview(), Style::default().fg(Color::Gray)),
        ]),
    ];

    let composer = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .block(focused_block(
            "Composer  Ctrl-2",
            matches!(
                app.focus,
                Focus::Method | Focus::Url | Focus::RequestName | Focus::Secure
            ),
        ));
    frame.render_widget(composer, area);
}

fn draw_pair_table(
    frame: &mut Frame<'_>,
    title: &str,
    pairs: &[KeyValue],
    selected: usize,
    focused: bool,
    area: Rect,
) {
    if pairs.is_empty() {
        let empty = Paragraph::new("No values. Press a to add.")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(focused_block(title, focused));
        frame.render_widget(empty, area);
        return;
    }

    let rows = pairs.iter().enumerate().map(|(index, pair)| {
        let style = if focused && selected == index {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else {
            Style::default()
        };
        Row::new(vec![
            Cell::from(pair.key.clone()),
            Cell::from(pair.value.clone()),
        ])
        .style(style)
    });

    let table = Table::new(
        rows,
        [Constraint::Percentage(35), Constraint::Percentage(65)],
    )
    .header(
        Row::new(vec![Cell::from("Key"), Cell::from("Value")])
            .style(Style::default().fg(Color::Yellow)),
    )
    .block(focused_block(title, focused))
    .column_spacing(1);

    frame.render_widget(table, area);
}

fn draw_body(frame: &mut Frame<'_>, draft: &RequestDraft, focused: bool, area: Rect) {
    let body = Paragraph::new(display_multiline_input(&draft.body, focused))
        .wrap(Wrap { trim: false })
        .block(focused_block("Body  Ctrl-7", focused));
    frame.render_widget(body, area);
}

fn draw_response(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let focused = app.active_tab == Tab::Requests && app.focus == Focus::Response;
    let Some(response) = &app.response else {
        let empty = Paragraph::new("No response yet. Press Ctrl-R to send the current request.")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(focused_block("Response  Ctrl-8", focused));
        frame.render_widget(empty, area);
        return;
    };

    let response_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let status_style = if (200..300).contains(&response.status) {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else if response.status >= 400 {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    };

    let metadata = Paragraph::new(Line::from(vec![
        Span::styled(format!("HTTP {}", response.status), status_style),
        Span::raw("  "),
        Span::styled(
            format!("{} ms", response.elapsed_ms),
            Style::default().fg(Color::Cyan),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{} bytes", response.size_bytes),
            Style::default().fg(Color::Gray),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{} headers", response.headers.len()),
            Style::default().fg(Color::Gray),
        ),
    ]))
    .block(focused_block("Response  Ctrl-8", focused));
    frame.render_widget(metadata, response_chunks[0]);

    let body = Paragraph::new(response.body.clone())
        .wrap(Wrap { trim: false })
        .scroll((app.response_scroll, 0))
        .block(Block::default().borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM));
    frame.render_widget(body, response_chunks[1]);
}

fn draw_environments(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(34), Constraint::Min(0)])
        .split(area);

    draw_env_list(frame, app, chunks[0]);
    draw_env_values(frame, app, chunks[1]);
}

fn draw_env_list(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let items = if app.project.environments.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "No environments",
            Style::default().fg(Color::DarkGray),
        )))]
    } else {
        app.project
            .environments
            .iter()
            .map(|env| {
                let selected = if env.selected { "selected" } else { "" };
                let auth = if env.authorization_set { "auth" } else { "" };
                ListItem::new(Line::from(vec![
                    Span::styled(
                        env.name.clone(),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    Span::styled(selected, Style::default().fg(Color::Cyan)),
                    Span::raw(" "),
                    Span::styled(auth, Style::default().fg(Color::Yellow)),
                ]))
            })
            .collect()
    };

    let mut state = ListState::default();
    if !app.project.environments.is_empty() {
        state.select(Some(app.selected_env));
    }

    let list = List::new(items)
        .block(focused_block(
            "Environments  Ctrl-9",
            app.active_tab == Tab::Environments && app.focus == Focus::EnvList,
        ))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_env_values(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let focused = app.active_tab == Tab::Environments && app.focus == Focus::EnvValues;
    let Some(env) = app.project.environments.get(app.selected_env) else {
        let empty = Paragraph::new("Press a to create an environment value.")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(focused_block("Environment Values  Ctrl-0", focused));
        frame.render_widget(empty, area);
        return;
    };

    draw_pair_table(
        frame,
        "Environment Values  Ctrl-0",
        &env.values,
        app.selected_env_value,
        focused,
        area,
    );
}

fn draw_help(frame: &mut Frame<'_>, area: Rect) {
    let help = vec![
        Line::from(Span::styled(
            "Navigation",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from("F1 requests | F2 environments | F3 help | Tab next field | Shift-Tab previous field"),
        Line::from("Direct focus: Ctrl-1 saved | Ctrl-2 composer | Ctrl-5 headers | Ctrl-6 form data"),
        Line::from("Direct focus: Ctrl-7 body | Ctrl-8 response | Ctrl-9 environments | Ctrl-0 env values"),
        Line::from(""),
        Line::from(Span::styled(
            "Requests",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from("Ctrl-R send | Ctrl-S save | Ctrl-N new draft | Ctrl-D delete saved request"),
        Line::from("Saved list: Enter load | Method/Secure: Enter or Space changes value"),
        Line::from("Headers/Form Data: a add | e or Enter edit | x remove"),
        Line::from("Response: Up/Down/PageUp/PageDown scroll"),
        Line::from(""),
        Line::from(Span::styled(
            "Environments",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from("Enter selects environment | a adds value | e edits value | x removes | u sets authorization"),
        Line::from("Environment placeholders such as {base_uri} are applied when requests run."),
        Line::from(""),
        Line::from("Ctrl-L reload workspace | Ctrl-C or Ctrl-Q quit | Esc backs up a tab or closes a modal"),
    ];

    let paragraph = Paragraph::new(help)
        .wrap(Wrap { trim: false })
        .block(Block::default().borders(Borders::ALL).title("Help"));
    frame.render_widget(paragraph, area);
}

fn draw_footer(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let status_style = match app.status.kind {
        StatusKind::Info => Style::default().fg(Color::White),
        StatusKind::Success => Style::default().fg(Color::Green),
        StatusKind::Error => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
    };

    let footer = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::DarkGray)),
            Span::styled(app.status.message.clone(), status_style),
        ]),
        Line::from(vec![
            Span::styled("Ctrl-R", Style::default().fg(Color::Cyan)),
            Span::raw(" send  "),
            Span::styled("Ctrl-S", Style::default().fg(Color::Cyan)),
            Span::raw(" save  "),
            Span::styled("Ctrl-L", Style::default().fg(Color::Cyan)),
            Span::raw(" reload  "),
            Span::styled("Ctrl-C/Q", Style::default().fg(Color::Cyan)),
            Span::raw(" quit"),
        ]),
    ])
    .block(Block::default().borders(Borders::LEFT | Borders::RIGHT));

    frame.render_widget(footer, area);
}

fn draw_modal(frame: &mut Frame<'_>, app: &App, modal: &Modal) {
    let area = centered_rect(62, 34, frame.area());
    frame.render_widget(Clear, area);

    let lines = match modal {
        Modal::ProjectName { name } => vec![
            field_line("Name", name, true),
            Line::from(""),
            Line::from("Enter creates the project. Esc cancels."),
        ],
        Modal::SaveRequest { name } => vec![
            field_line("Name", name, true),
            Line::from(""),
            Line::from("Enter saves the request. Esc cancels."),
        ],
        Modal::Header {
            key, value, active, ..
        }
        | Modal::FormData {
            key, value, active, ..
        } => vec![
            field_line("Key", key, *active == PairField::Key),
            field_line("Value", value, *active == PairField::Value),
            Line::from(""),
            Line::from("Tab changes field. Enter saves. Esc cancels."),
        ],
        Modal::EnvValue {
            env,
            key,
            value,
            active,
            ..
        } => vec![
            field_line("Environment", env, *active == EnvField::Environment),
            field_line("Key", key, *active == EnvField::Key),
            field_line("Value", value, *active == EnvField::Value),
            Line::from(""),
            Line::from("Tab changes field. Enter saves. Esc cancels."),
        ],
        Modal::Authorization { value } => {
            let target = app
                .project
                .selected_environment
                .as_deref()
                .unwrap_or("default environment");
            vec![
                Line::from(format!("Target: {target}")),
                field_line("Value", value, true),
                Line::from(""),
                Line::from("Leave empty and press Enter to remove authorization."),
            ]
        }
        Modal::ConfirmDeleteRequest { name } => vec![
            Line::from(format!("Delete saved request '{name}'?")),
            Line::from(""),
            Line::from("Press y or Enter to confirm. Press n or Esc to cancel."),
        ],
        Modal::ConfirmDeleteEnv { name } => vec![
            Line::from(format!("Delete environment '{name}'?")),
            Line::from(""),
            Line::from("Press y or Enter to confirm. Press n or Esc to cancel."),
        ],
        Modal::ConfirmDeleteEnvValue { env, key } => vec![
            Line::from(format!("Delete '{key}' from '{env}'?")),
            Line::from(""),
            Line::from("Press y or Enter to confirm. Press n or Esc to cancel."),
        ],
    };

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false }).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(modal.title()),
    );
    frame.render_widget(paragraph, area);
}

fn field_line(label: &'static str, input: &TextInput, focused: bool) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label:<12}"), label_style(focused)),
        Span::styled(display_single_input(input, focused), value_style(focused)),
    ])
}

fn display_single_input(input: &TextInput, focused: bool) -> String {
    let mut value = input.value();
    if focused {
        let (_, col) = input.cursor();
        insert_cursor(&mut value, col);
    }
    if value.is_empty() {
        String::from(" ")
    } else {
        value
    }
}

fn display_multiline_input(input: &TextInput, focused: bool) -> String {
    let mut lines = input.lines().to_vec();
    if focused {
        let (row, col) = input.cursor();
        if let Some(line) = lines.get_mut(row) {
            insert_cursor(line, col);
        }
    }

    if lines.is_empty() {
        String::from(" ")
    } else {
        lines.join("\n")
    }
}

fn insert_cursor(value: &mut String, col: usize) {
    let mut chars = value.chars().collect::<Vec<_>>();
    let col = col.min(chars.len());
    chars.insert(col, '|');
    *value = chars.into_iter().collect();
}

fn focused_block(title: impl AsRef<str>, focused: bool) -> Block<'static> {
    let style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    Block::default()
        .borders(Borders::ALL)
        .border_style(style)
        .title(title.as_ref().to_string())
}

fn label_style(focused: bool) -> Style {
    if focused {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

fn value_style(focused: bool) -> Style {
    if focused {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    }
}

fn centered_rect(width_percent: u16, height_percent: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - height_percent) / 2),
            Constraint::Percentage(height_percent),
            Constraint::Percentage((100 - height_percent) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - width_percent) / 2),
            Constraint::Percentage(width_percent),
            Constraint::Percentage((100 - width_percent) / 2),
        ])
        .split(vertical[1])[1]
}
