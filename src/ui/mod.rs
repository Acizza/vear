mod event;
mod panel;

use crate::archive::ArchiveEntries;
use anyhow::{Context, Result};
use crossterm::event::KeyCode;
use crossterm::terminal;
use event::{EventKind, Events};
use panel::{Draw, MainPanel, Panel};
use std::io;
use tui::backend::CrosstermBackend;
use tui::Terminal;

pub enum CycleResult {
    Ok,
    Exit,
    Error(anyhow::Error),
}

pub struct UI<'a> {
    events: Events,
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    main_panel: MainPanel<'a>,
}

impl<'a> UI<'a> {
    pub fn init(archive_entries: ArchiveEntries) -> Result<Self> {
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
            main_panel: MainPanel::new(archive_entries),
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
        // We need to remove the mutable borrow on self so we can call other mutable methods on it during our draw call.
        // This *should* be completely safe as long as nothing in the draw closure can access the terminal.
        let terminal: *mut _ = &mut self.terminal;
        let terminal: &mut _ = unsafe { &mut *terminal };

        terminal
            .draw(|frame| self.main_panel.draw(frame.size(), frame))
            .map_err(Into::into)
    }

    fn process_key(&mut self, key: KeyCode) -> CycleResult {
        if key == KeyCode::Char('q') {
            return CycleResult::Exit;
        }

        self.main_panel.process_key(key);
        CycleResult::Ok
    }

    fn tick(&mut self) -> CycleResult {
        CycleResult::Ok
    }

    pub fn exit(mut self) -> Result<()> {
        self.terminal.clear().ok();
        terminal::disable_raw_mode().map_err(Into::into)
    }
}
