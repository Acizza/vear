mod event;

use anyhow::Result;
use crossterm::event::KeyCode;
use crossterm::terminal;
use event::{EventKind, Events};

pub enum CycleResult {
    Ok,
    Exit,
    Error(anyhow::Error),
}

pub struct UI {
    events: Events,
}

impl UI {
    pub fn init() -> Result<Self> {
        terminal::enable_raw_mode()?;

        Ok(Self {
            events: Events::new(),
        })
    }

    pub async fn next_cycle(&mut self) -> CycleResult {
        let event = match self.events.next().await {
            Ok(Some(event)) => event,
            Ok(None) => return CycleResult::Ok,
            Err(event::ErrorKind::ExitRequest) => return CycleResult::Exit,
            Err(event::ErrorKind::Other(err)) => return CycleResult::Error(err),
        };

        println!("event: {:?}", event);

        if let EventKind::Key(KeyCode::Char('q')) = event {
            return CycleResult::Exit;
        }

        CycleResult::Ok
    }

    pub fn exit(self) -> Result<()> {
        terminal::disable_raw_mode().map_err(Into::into)
    }
}
