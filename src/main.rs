#![warn(clippy::pedantic)]
#![allow(clippy::clippy::cast_possible_truncation)]
#![allow(clippy::inline_always)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::shadow_unrelated)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::default_trait_access)]
#![allow(clippy::cast_sign_loss)]

mod archive;
mod ui;
mod util;

use anyhow::{anyhow, Context, Result};
use archive::Archive;
use argh::FromArgs;
use ui::{CycleResult, UI};

#[derive(FromArgs)]
/// View, extract, and mount archives in the terminal.
struct Args {
    /// the path of the archive to open
    #[argh(positional)]
    path: String,
}

#[async_std::main]
async fn main() -> Result<()> {
    let args: Args = argh::from_env();

    let archive = Archive::read(&args.path)
        .with_context(|| anyhow!("failed to read files from {}", args.path))?;

    let mut ui = UI::init(archive)?;

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
