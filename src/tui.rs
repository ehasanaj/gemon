use crate::EmptyResult;
use app::{App, AppCommand};
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind,
        KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    error::Error,
    io::{self, Stdout},
    time::Duration,
};

mod app;
mod input;
mod layout;
mod ui;

pub async fn run() -> EmptyResult {
    let mut terminal = TerminalSession::new()?;
    let mut app = App::new();

    loop {
        terminal.draw(&app)?;

        if app.should_quit {
            break;
        }

        if event::poll(Duration::from_millis(150))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => match app.handle_key(key) {
                    AppCommand::None => {}
                    AppCommand::SendRequest => app.send_request().await,
                },
                Event::Mouse(mouse) => {
                    let area = terminal.area()?;
                    match app.handle_mouse(mouse, area) {
                        AppCommand::None => {}
                        AppCommand::SendRequest => app.send_request().await,
                    }
                }
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    }

    Ok(())
}

struct TerminalSession {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    keyboard_enhancement_enabled: bool,
}

impl TerminalSession {
    fn new() -> Result<TerminalSession, Box<dyn Error>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let keyboard_enhancement_enabled = execute!(
            stdout,
            PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                    | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
            )
        )
        .is_ok();

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;

        Ok(TerminalSession {
            terminal,
            keyboard_enhancement_enabled,
        })
    }

    fn draw(&mut self, app: &App) -> Result<(), Box<dyn Error>> {
        self.terminal.draw(|frame| ui::draw(frame, app))?;
        Ok(())
    }

    fn area(&self) -> Result<ratatui::layout::Rect, Box<dyn Error>> {
        let size = self.terminal.size()?;
        Ok(ratatui::layout::Rect::new(0, 0, size.width, size.height))
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        if self.keyboard_enhancement_enabled {
            let _ = execute!(self.terminal.backend_mut(), PopKeyboardEnhancementFlags);
        }
        let _ = execute!(
            self.terminal.backend_mut(),
            DisableMouseCapture,
            LeaveAlternateScreen
        );
        let _ = self.terminal.show_cursor();
    }
}
