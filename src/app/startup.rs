use std::process::ExitCode;

use clap::Parser;

use crate::cli::{Cli, Command};
use crate::project::ProjectPaths;
use crate::transpiler::Transpiler;

pub fn run() {
    if let Err(error) = try_run() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}

fn try_run() -> anyhow::Result<ExitCode> {
    let cli = Cli::parse();
    let paths = ProjectPaths::new(cli.path)?;
    let transpiler = Transpiler::new(paths);

    match cli.command {
        Command::Build { cargo_args } => transpiler.build(&cargo_args),
        Command::Run { cargo_args } => transpiler.run(&cargo_args),
    }
}
