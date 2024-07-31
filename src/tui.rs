use crate::app::AppEvent;
pub use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode as Key},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use ratatui::Terminal;
use std::io;
use std::sync::mpsc;
use tracing::trace;

pub fn init_terminal() -> io::Result<Terminal<impl Backend>> {
    trace!(target:"crossterm", "Initializing terminal");
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(io::stdout());
    Terminal::new(backend)
}

pub fn restore_terminal() -> io::Result<()> {
    trace!(target:"crossterm", "Restoring terminal");
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)
}

pub fn input_thread(tx_event: mpsc::Sender<AppEvent>) -> anyhow::Result<()> {
    trace!(target:"crossterm", "Starting input thread");
    while let Ok(event) = event::read() {
        trace!(target:"crossterm", "Stdin event received {:?}", event);
        tx_event.send(AppEvent::UiEvent(event))?;
    }
    Ok(())
}
