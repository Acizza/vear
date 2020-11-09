mod event;

use anyhow::{Context, Result};
use crossterm::event::KeyCode;
use crossterm::terminal;
use event::{EventKind, Events};
use std::io;
use tui::backend::CrosstermBackend;
use tui::Terminal;

pub enum CycleResult {
    Ok,
    Exit,
    Error(anyhow::Error),
}

pub struct UI {
    events: Events,
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl UI {
    pub fn init() -> Result<Self> {
        terminal::enable_raw_mode().context("failed to enable raw mode")?;

        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).context("terminal creation failed")?;

        terminal.clear().context("failed to clear terminal")?;

        terminal
            .hide_cursor()
            .context("failed to hide mouse cursor")?;

        Ok(Self {
            events: Events::new(),
            terminal,
        })
    }

    pub async fn next_cycle(&mut self) -> CycleResult {
        if let Err(err) = self.draw() {
            return CycleResult::Error(err);
        }

        let event = match self.events.next().await {
            Ok(Some(event)) => event,
            Ok(None) => return CycleResult::Ok,
            Err(event::ErrorKind::ExitRequest) => return CycleResult::Exit,
            Err(event::ErrorKind::Other(err)) => return CycleResult::Error(err),
        };

        match event {
            EventKind::Key(key) => self.process_key(key),
            EventKind::Tick => self.tick(),
        }
    }

    fn draw(&mut self) -> Result<()> {
        self.terminal.draw(|mut frame| {}).map_err(Into::into)
    }

    fn process_key(&mut self, key: KeyCode) -> CycleResult {
        if key == KeyCode::Char('q') {
            CycleResult::Exit
        } else {
            CycleResult::Ok
        }
    }

    fn tick(&mut self) -> CycleResult {
        CycleResult::Ok
    }

    pub fn exit(mut self) -> Result<()> {
        self.terminal.clear().ok();
        terminal::disable_raw_mode().map_err(Into::into)
    }
}
