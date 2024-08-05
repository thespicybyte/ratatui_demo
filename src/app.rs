use crate::tui::*;
use color_eyre::Result;
use crossterm::event::{Event, KeyCode, MouseButton, MouseEvent, MouseEventKind};
use log::{debug, error, info, trace, warn, LevelFilter};
use ratatui::prelude::*;
use ratatui::widgets::canvas::Rectangle;
use ratatui::widgets::{Block, Borders, Gauge, Paragraph, Tabs, Wrap};
use std::fmt::{Display, Formatter};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;
use tui_logger::*;

pub(crate) struct App {
    input: Input,
    mode: AppMode,
    states: Vec<TuiWidgetState>,
    selected_tab: usize,
    progress_counter: Option<u16>,
    input_rect: Rect,
    console_rect: Rect,
    focus_mode: FocusMode,
    scroll: usize,
    messages: Vec<String>,
    selection_start: Option<(usize, usize)>, // (line, column)
    selection_end: Option<(usize, usize)>,   // (line, column)
    dragging: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum FocusMode {
    #[default]
    Input,
    Console,
}

impl Display for FocusMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FocusMode::Input => write!(f, "Input"),
            FocusMode::Console => write!(f, "Console"),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum AppMode {
    #[default]
    Run,
    Quit,
}

#[derive(Debug)]
pub enum AppEvent {
    UiEvent(Event),
    CounterChanged(Option<u16>),
}

impl App {
    pub fn new() -> App {
        let states = vec![
            TuiWidgetState::new().set_default_display_level(LevelFilter::Info),
            TuiWidgetState::new().set_default_display_level(LevelFilter::Info),
            TuiWidgetState::new().set_default_display_level(LevelFilter::Info),
            TuiWidgetState::new().set_default_display_level(LevelFilter::Info),
        ];

        App {
            input: Input::default(),
            mode: AppMode::Run,
            states,
            selected_tab: 0,
            progress_counter: None,
            input_rect: Default::default(),
            console_rect: Default::default(),
            focus_mode: Default::default(),
            scroll: 0,
            messages: vec![],
            selection_start: None,
            selection_end: None,
            dragging: false,
        }
    }

    pub fn start(mut self, terminal: &mut Terminal<impl Backend>) -> Result<()> {
        // Use an mpsc::channel to combine stdin events with app events
        let (tx, rx) = mpsc::channel();
        let event_tx = tx.clone();
        let progress_tx = tx.clone();

        thread::spawn(move || input_thread(event_tx));
        thread::spawn(move || progress_task(progress_tx).unwrap());
        thread::spawn(move || background_task());

        self.run(terminal, rx)
    }

    /// Main application loop
    fn run(
        &mut self,
        terminal: &mut Terminal<impl Backend>,
        rx: mpsc::Receiver<AppEvent>,
    ) -> Result<()> {
        for event in rx {
            match event {
                AppEvent::UiEvent(event) => self.handle_ui_event(event),
                AppEvent::CounterChanged(value) => self.update_progress_bar(event, value),
            }
            if self.mode == AppMode::Quit {
                break;
            }
            self.draw(terminal)?;
        }
        Ok(())
    }

    fn update_progress_bar(&mut self, event: AppEvent, value: Option<u16>) {
        // trace!(target: "App", "Updating progress bar {:?}",event);
        self.progress_counter = value;
        if value.is_none() {
            info!(target: "App", "Background task finished");
        }
    }

    fn handle_ui_event(&mut self, event: Event) {
        trace!(target: "App", "Handling UI event: {:?}",event);

        if let Event::Mouse(mouse_event) = event {
            let mouse_row = mouse_event.row;
            let mouse_col = mouse_event.column;

            match mouse_event.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    if self.rect_contains(self.input_rect, mouse_row, mouse_col) {
                        self.focus_mode = FocusMode::Input;
                        self.dragging = false;
                    } else if self.rect_contains(self.console_rect, mouse_row, mouse_col) {
                        self.focus_mode = FocusMode::Console;
                        // Start selection
                        let relative_row = mouse_row - self.console_rect.y;
                        let relative_col = mouse_col - self.console_rect.x;
                        self.selection_start = Some((relative_row as usize, relative_col as usize));
                        self.selection_end = self.selection_start;
                        self.dragging = true;
                    } else {
                        self.selection_start = None;
                        self.selection_end = None;
                        self.dragging = false;
                    }
                }
                MouseEventKind::Drag(MouseButton::Left) => {
                    if self.dragging && self.focus_mode == FocusMode::Console {
                        let relative_row = mouse_row - self.console_rect.y;
                        let relative_col = mouse_col - self.console_rect.x;
                        self.selection_end = Some((relative_row as usize, relative_col as usize));
                    }
                }
                MouseEventKind::Up(MouseButton::Left) => {
                    self.dragging = false;
                }
                _ => {}
            }
        }

        if let Event::Key(key) = event {
            debug!(target: "App", "Handling Key event: {:?}",event);
            let code = key.code;

            if self.focus_mode == FocusMode::Console {
                match key.code {
                    KeyCode::Esc => {
                        self.selection_start = None;
                        self.selection_end = None;
                    }
                    KeyCode::Tab => self.focus_mode = FocusMode::Input,
                    _ => {}
                }
            }
            if self.focus_mode == FocusMode::Input {
                match code.into() {
                    KeyCode::Enter => {
                        self.messages.push(self.input.value().into());
                        self.input.reset();
                        debug!("{:?}", self.messages);
                    }
                    KeyCode::Esc => self.mode = AppMode::Quit,
                    _ => (),
                }
                self.input.handle_event(&event);
            }
        }
    }

    fn rect_contains(&self, rect: Rect, row: u16, col: u16) -> bool {
        row >= rect.y && row < rect.y + rect.height && col >= rect.x && col < rect.x + rect.width
    }
    fn selected_state(&mut self) -> &mut TuiWidgetState {
        &mut self.states[self.selected_tab]
    }

    // fn next_tab(&mut self) {
    //     self.selected_tab = (self.selected_tab + 1) % self.tab_names.len();
    // }

    fn draw(&mut self, terminal: &mut Terminal<impl Backend>) -> Result<()> {
        terminal.draw(|frame| {
            let input_rect = self.input_rect.clone();
            let scroll = self.scroll;
            let input = self.input.clone();
            let focus_mode = self.focus_mode;
            frame.render_widget(self, frame.size());
            if focus_mode == FocusMode::Input {
                frame.set_cursor(
                    // Put cursor past the end of the input text
                    input_rect.x + (input.visual_cursor().max(scroll) - scroll) as u16 + 1,
                    // Move one line down, from the border to the input line
                    input_rect.y + 1,
                )
            }
        })?;

        Ok(())
    }
}

/// A simulated task that sends a counter value to the UI ranging from 0 to 100 every second.
fn progress_task(tx: mpsc::Sender<AppEvent>) -> anyhow::Result<()> {
    for progress in 0..100 {
        // debug!(target:"progress-task", "Send progress to UI thread. Value: {:?}", progress);
        tx.send(AppEvent::CounterChanged(Some(progress)))?;

        // trace!(target:"progress-task", "Sleep one second");
        thread::sleep(Duration::from_millis(1000));
    }
    // info!(target:"progress-task", "Progress task finished");
    tx.send(AppEvent::CounterChanged(None))?;
    Ok(())
}

/// A background task that logs a log entry for each log level every second.
fn background_task() {
    loop {
        // error!(target:"background-task", "an error");
        // warn!(target:"background-task", "a warning");
        // info!(target:"background-task", "an info");
        // debug!(target:"background-task", "a debug");
        // trace!(target:"background-task", "a trace");
        thread::sleep(Duration::from_millis(1000));
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let text = vec![
            Line::from(vec![
                Span::raw("First"),
                Span::styled("line", Style::new().green().italic()),
                ".".into(),
            ]),
            Line::from("Second line".red()),
            "Third line".into(),
        ];

        let [left_col, right_col] =
            Layout::horizontal([Constraint::Percentage(25), Constraint::Percentage(75)])
                .areas(area);

        let left_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(75),
            ])
            .split(left_col);

        let right_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Min(1), Constraint::Length(3)])
            .split(right_col);

        self.console_rect = right_rows[0];
        self.input_rect = right_rows[1];

        Paragraph::new(text.clone())
            .block(Block::bordered().title("Logo"))
            .style(Style::new().white().on_black())
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .render(left_rows[0], buf);

        Paragraph::new(text.clone())
            .block(Block::bordered().title("Session Info"))
            .style(Style::new().white().on_black())
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false })
            .render(left_rows[1], buf);

        Paragraph::new(text.clone())
            .block(Block::bordered().title("Items"))
            .style(Style::new().white().on_black())
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .render(left_rows[2], buf);

        let highlighted_content: Vec<Line> = self
            .messages
            .iter()
            .enumerate()
            .flat_map(|(line_index, message)| {
                let mut spans = Vec::new();
                let chars: Vec<char> = message.chars().collect();
                let mut in_selection = false;

                for (char_index, &ch) in chars.iter().enumerate() {
                    if let Some((start_line, start_col)) = self.selection_start {
                        if let Some((end_line, end_col)) = self.selection_end {
                            if (line_index == start_line && char_index >= start_col)
                                || (line_index == end_line && char_index <= end_col)
                                || (line_index > start_line && line_index < end_line)
                            {
                                in_selection = true;
                            } else {
                                in_selection = false;
                            }
                        }
                    }

                    let span = if in_selection {
                        Span::styled(
                            ch.to_string(),
                            Style::default().fg(Color::Yellow).bg(Color::Blue),
                        )
                    } else {
                        Span::raw(ch.to_string())
                    };
                    spans.push(span);
                }

                vec![Line::from(spans)]
            })
            .collect();

        Paragraph::new(highlighted_content)
            .block(
                Block::bordered()
                    .title("Console")
                    .style(match self.focus_mode {
                        FocusMode::Input => Style::default().fg(Color::White),
                        FocusMode::Console => Style::default().fg(Color::Yellow),
                    }),
            )
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: false })
            .render(self.console_rect, buf);

        let width = self.input_rect.width.max(3) - 3; // keep 2 for borders and 1 for cursor
        self.scroll = self.input.visual_scroll(width as usize);
        Paragraph::new(self.input.value())
            .style(Style::default().fg(Color::White))
            .scroll((0, self.scroll as u16))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(match self.focus_mode {
                        FocusMode::Input => Style::default().fg(Color::Yellow),
                        FocusMode::Console => Style::default().fg(Color::White),
                    })
                    .title("Input"),
            )
            .render(self.input_rect, buf);
    }
}
