use std::process::ExitCode;

use clap::Parser;

use crate::cargo::{
    AddOptions, BuildOptions, CargoOptions, CargoSubcommand, CleanOptions, EnvVar, RemoveOptions,
    RunOptions, TestOptions,
};
use crate::cli::{CargoCommandArgs, Cli, Command};
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
        Command::Add(command) => transpiler.execute(CargoSubcommand::Add(AddOptions {
            cargo: cargo_options_from_parts(false, command.env, command.cargo_args)?,
            packages: command.packages,
        })),
        Command::Build(command) => transpiler.execute(CargoSubcommand::Build(BuildOptions {
            cargo: cargo_options(command)?,
        })),
        Command::Clean(command) => transpiler.execute(CargoSubcommand::Clean(CleanOptions {
            cargo: cargo_options(command)?,
        })),
        Command::Remove(command) => transpiler.execute(CargoSubcommand::Remove(RemoveOptions {
            cargo: cargo_options_from_parts(false, command.env, command.cargo_args)?,
            packages: command.packages,
        })),
        Command::Run(command) => transpiler.execute(CargoSubcommand::Run(RunOptions {
            cargo: cargo_options(command.cargo)?,
            app_args: command.app_args,
        })),
        Command::Test(command) => transpiler.execute(CargoSubcommand::Test(TestOptions {
            cargo: cargo_options(command)?,
        })),
        Command::Cargo(command) => transpiler.execute(CargoSubcommand::Custom {
            name: command.name,
            cargo: cargo_options(command.cargo)?,
        }),
    }
}

fn cargo_options(command: CargoCommandArgs) -> anyhow::Result<CargoOptions> {
    cargo_options_from_parts(command.release, command.env, command.cargo_args)
}

fn cargo_options_from_parts(
    release: bool,
    env: Vec<String>,
    cargo_args: Vec<String>,
) -> anyhow::Result<CargoOptions> {
    let envs = env
        .into_iter()
        .map(|entry| EnvVar::parse(&entry))
        .collect::<anyhow::Result<Vec<_>>>()?;

    Ok(CargoOptions {
        release,
        envs,
        args: cargo_args,
    })
}
