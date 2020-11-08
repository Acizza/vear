use crossterm::event::{Event, EventStream, KeyCode};
use futures::{future::FutureExt, select, StreamExt};
use futures_timer::Delay;
use std::time::Duration;

#[derive(Debug)]
pub enum EventKind {
    Key(KeyCode),
    Tick,
}

pub enum ErrorKind {
    ExitRequest,
    Other(anyhow::Error),
}

type EventError<T> = std::result::Result<T, ErrorKind>;

pub struct Events {
    reader: EventStream,
}

impl Events {
    const TICK_DURATION_MS: u64 = 1_000;

    pub fn new() -> Self {
        Self {
            reader: EventStream::new(),
        }
    }

    pub async fn next(&mut self) -> EventError<Option<EventKind>> {
        let mut tick = Delay::new(Duration::from_millis(Self::TICK_DURATION_MS)).fuse();
        let mut next_event = self.reader.next().fuse();

        select! {
            _ = tick => Ok(Some(EventKind::Tick)),
            event = next_event => match event {
                Some(Ok(Event::Key(key))) => Ok(Some(EventKind::Key(key.code))),
                Some(Ok(_)) => Ok(None),
                Some(Err(err)) => Err(ErrorKind::Other(err.into())),
                None => Err(ErrorKind::ExitRequest),
            }
        }
    }
}
