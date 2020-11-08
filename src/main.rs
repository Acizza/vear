mod ui;

use anyhow::Result;
use ui::{CycleResult, UI};

#[async_std::main]
async fn main() -> Result<()> {
    let mut ui = UI::init()?;

    loop {
        match ui.next_cycle().await {
            CycleResult::Ok => (),
            CycleResult::Exit => break,
            CycleResult::Error(err) => {
                ui.exit().ok();
                return Err(err);
            }
        }
    }

    ui.exit()
}
