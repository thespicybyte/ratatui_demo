mod action;
mod app;
mod config;
mod errors;
mod logging;
mod tui;

use crate::app::App;
use crate::tui::{init_terminal, restore_terminal};
use color_eyre::{eyre::WrapErr, Result};
use ratatui::{
    buffer::Buffer, crossterm::event::KeyCode, layout::Rect, style::Stylize, widgets::Widget,
};
use tracing::{debug, span, Level};
use tui_logger::{init_logger, set_default_level};

// use logging;

fn main() -> Result<()> {
    // init_logger(LevelFilter::Trace)?;
    // set_default_level(LevelFilter::Trace);

    logging::init()?;
    //
    // let h = std::thread::spawn(|| {
    //     let span = span!(Level::DEBUG, "foo", task = "footask");
    //     let f = span.enter();
    //     debug!("Logging initialized");
    // });
    let span = span!(Level::DEBUG, "foo", task = "initializing");
    let init_span = span.enter();
    debug!("Logging initialized");
    // h.join();

    //
    let mut terminal = init_terminal()?;
    terminal.clear()?;
    drop(init_span);
    // terminal.hide_cursor()?;
    //
    App::new().start(&mut terminal)?;

    let span = span!(Level::DEBUG, "foo", task = "restoring");
    let _restore_span = span.enter();
    restore_terminal()?;
    terminal.clear()?;

    Ok(())
}
