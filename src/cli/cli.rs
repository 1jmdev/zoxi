use std::process::ExitCode;

use clap::Parser;

use crate::build::{
    BuildOptions, BuildSubcommand, CleanOptions, EnvVar, RunOptions, RustcOptions, TestOptions,
};
use super::args::{Cli, Command, RustcCommandArgs};
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
        Command::Build(command) => transpiler.execute(BuildSubcommand::Build(BuildOptions {
            rustc: rustc_options(command)?,
        })),
        Command::Clean => transpiler.execute(BuildSubcommand::Clean(CleanOptions)),
        Command::Run(command) => transpiler.execute(BuildSubcommand::Run(RunOptions {
            rustc: rustc_options(command.rustc)?,
            app_args: command.app_args,
        })),
        Command::Test(command) => transpiler.execute(BuildSubcommand::Test(TestOptions {
            rustc: rustc_options(command)?,
        })),
    }
}

fn rustc_options(command: RustcCommandArgs) -> anyhow::Result<RustcOptions> {
    rustc_options_from_parts(command.release, command.env, command.rustc_args)
}

fn rustc_options_from_parts(
    release: bool,
    env: Vec<String>,
    rustc_args: Vec<String>,
) -> anyhow::Result<RustcOptions> {
    let envs = env
        .into_iter()
        .map(|entry| EnvVar::parse(&entry))
        .collect::<anyhow::Result<Vec<_>>>()?;

    Ok(RustcOptions {
        release,
        envs,
        args: rustc_args,
    })
}
