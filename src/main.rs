#![windows_subsystem = "windows"]

// mod cli;
mod gui;
mod installer;

const ICON: &[u8] = include_bytes!("../quilt.png");

fn main() -> anyhow::Result<()> {
    // let args = cli::Args::parse();

    // if let Some(subcommand) = args.subcommand {
    //     match subcommand {
    //         cli::SubCommands::Client => todo!(),
    //         cli::SubCommands::Server => todo!(),
    //     }
    // } else {
        gui::run()?;
    // }

    Ok(())
}
