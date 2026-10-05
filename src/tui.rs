use crate::{
    request::{request_builder::GemonRequest, request_builder::GemonResponse},
    EmptyResult,
};
use app::{App, AppCommand};
use crossterm::{
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event, KeyEventKind, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
        PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, supports_keyboard_enhancement, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    error::Error,
    future::Future,
    io::{self, Stdout},
    panic,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};
use tokio::sync::mpsc::{self, UnboundedReceiver};

mod actions;
mod app;
mod clipboard;
mod export;
mod input;
mod layout;
mod theme;
mod ui;
mod viewer;

type PendingResponse = Pin<Box<dyn Future<Output = Result<GemonResponse, String>>>>;

/// Whether keyboard enhancement flags were pushed and must be popped on exit.
static KEYBOARD_ENHANCED: AtomicBool = AtomicBool::new(false);

const TICK: Duration = Duration::from_millis(100);
const INPUT_POLL: Duration = Duration::from_millis(50);

pub async fn run() -> EmptyResult {
    let mut terminal = TerminalSession::new()?;
    let mut app = App::new();
    let (mut events, input) = spawn_input_reader();
    let mut pending: Option<PendingResponse> = None;
    let mut ticker = tokio::time::interval(TICK);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let mut disk_check = tokio::time::interval(Duration::from_secs(2));
    disk_check.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    while !app.should_quit {
        terminal.draw(&app)?;

        tokio::select! {
            event = events.recv() => {
                let Some(event) = event else { break };
                handle_event(&mut app, &mut terminal, &mut pending, event)?;
                // Drain queued input before redrawing so fast typing and key repeat stay smooth.
                while let Ok(event) = events.try_recv() {
                    handle_event(&mut app, &mut terminal, &mut pending, event)?;
                }
                app.sync_with_disk();
            }
            result = async { pending.as_mut().expect("guarded by select condition").await }, if pending.is_some() => {
                pending = None;
                app.finish_request(result);
            }
            _ = ticker.tick(), if app.is_busy() => app.on_tick(),
            _ = disk_check.tick() => app.sync_with_disk(),
        }
    }

    input.stop();
    Ok(())
}

fn handle_event(
    app: &mut App,
    terminal: &mut TerminalSession,
    pending: &mut Option<PendingResponse>,
    event: Event,
) -> EmptyResult {
    let command = match event {
        Event::Key(key) if key.kind != KeyEventKind::Release => app.handle_key(key),
        Event::Mouse(mouse) => app.handle_mouse(mouse, terminal.area()?),
        Event::Paste(text) => {
            app.handle_paste(&text);
            AppCommand::None
        }
        _ => AppCommand::None,
    };

    match command {
        AppCommand::None => {}
        AppCommand::Send(request) => {
            // Replacing an in-flight future drops it, which cancels that request.
            *pending = Some(Box::pin(async move {
                request
                    .execute()
                    .await
                    .map_err(|err| describe_error(err.as_ref()))
            }));
        }
        AppCommand::Cancel => *pending = None,
    }
    Ok(())
}

/// Joins an error with its causes; HTTP client errors keep the useful part (e.g.
/// "Connection refused") in their source chain.
fn describe_error(err: &(dyn Error + 'static)) -> String {
    let mut message = err.to_string();
    let mut source = err.source();
    while let Some(cause) = source {
        let text = cause.to_string();
        if !message.contains(&text) {
            message.push_str(": ");
            message.push_str(&text);
        }
        source = cause.source();
    }
    message
}

/// Reads terminal input on a dedicated thread so a slow request never blocks the UI.
fn spawn_input_reader() -> (UnboundedReceiver<Event>, InputReader) {
    let (sender, receiver) = mpsc::unbounded_channel();
    let stop = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&stop);
    thread::spawn(move || {
        while !flag.load(Ordering::Relaxed) {
            match event::poll(INPUT_POLL) {
                Ok(true) => match event::read() {
                    Ok(event) => {
                        if sender.send(event).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                },
                Ok(false) => {}
                Err(_) => break,
            }
        }
    });
    (receiver, InputReader { stop })
}

struct InputReader {
    stop: Arc<AtomicBool>,
}

impl InputReader {
    fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

struct TerminalSession {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalSession {
    fn new() -> Result<TerminalSession, Box<dyn Error>> {
        install_panic_hook();
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(
            stdout,
            EnterAlternateScreen,
            EnableMouseCapture,
            EnableBracketedPaste
        )?;
        // Only disambiguation is requested: it lets terminals with the kitty protocol report
        // keys like Ctrl+Enter, while ordinary and shifted characters still arrive as text.
        if supports_keyboard_enhancement().unwrap_or(false)
            && execute!(
                stdout,
                PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
            )
            .is_ok()
        {
            KEYBOARD_ENHANCED.store(true, Ordering::Relaxed);
        }

        let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;
        terminal.clear()?;
        Ok(TerminalSession { terminal })
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
        restore_terminal();
        let _ = self.terminal.show_cursor();
    }
}

fn restore_terminal() {
    let _ = disable_raw_mode();
    let mut stdout = io::stdout();
    if KEYBOARD_ENHANCED.swap(false, Ordering::Relaxed) {
        let _ = execute!(stdout, PopKeyboardEnhancementFlags);
    }
    let _ = execute!(
        stdout,
        DisableBracketedPaste,
        DisableMouseCapture,
        LeaveAlternateScreen,
        crossterm::cursor::Show
    );
}

/// Restores the terminal before a panic message is printed, so it stays readable.
fn install_panic_hook() {
    let previous = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        restore_terminal();
        previous(info);
    }));
}
