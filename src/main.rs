mod archive;
mod ui;
mod util;

use anyhow::{anyhow, Context, Result};
use archive::ArchiveEntries;
use gumdrop::Options;
use std::path::PathBuf;
use ui::{CycleResult, UI};

#[derive(Debug, Options)]
struct CmdArgs {
    #[options(help = "print help message")]
    help: bool,
    #[options(free)]
    path: Vec<String>,
}

#[async_std::main]
async fn main() -> Result<()> {
    let args = CmdArgs::parse_args_default_or_exit();
    let path = PathBuf::from(args.path.join(" "));

    let archive_entries = ArchiveEntries::read(&path)
        .with_context(|| anyhow!("failed to read files from {}", path.display()))?;

    let mut ui = UI::init(archive_entries)?;

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
