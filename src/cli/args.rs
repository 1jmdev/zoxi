use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "zoxi")]
#[command(about = "Transpile Zoxi sources into Rust and run cargo")]
pub struct Cli {
    #[arg(long, short, global = true)]
    pub path: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Add(AddCommand),
    Build(CargoCommandArgs),
    Clean(CargoCommandArgs),
    Remove(RemoveCommand),
    Run(RunCommand),
    Test(CargoCommandArgs),
    Cargo(CustomCargoCommand),
}

#[derive(Debug, Args, Clone, Default)]
pub struct CargoCommandArgs {
    #[arg(short = 'r', long = "release")]
    pub release: bool,
    #[arg(short = 'e', long = "env", value_name = "KEY=VALUE")]
    pub env: Vec<String>,
    #[arg(allow_hyphen_values = true)]
    pub cargo_args: Vec<String>,
}

#[derive(Debug, Args, Clone, Default)]
pub struct RunCommand {
    #[command(flatten)]
    pub cargo: CargoCommandArgs,
    #[arg(last = true)]
    pub app_args: Vec<String>,
}

#[derive(Debug, Args, Clone, Default)]
pub struct AddCommand {
    #[arg(short = 'e', long = "env", value_name = "KEY=VALUE")]
    pub env: Vec<String>,
    #[arg(required = true)]
    pub packages: Vec<String>,
    #[arg(last = true)]
    pub cargo_args: Vec<String>,
}

#[derive(Debug, Args, Clone, Default)]
pub struct RemoveCommand {
    #[arg(short = 'e', long = "env", value_name = "KEY=VALUE")]
    pub env: Vec<String>,
    #[arg(required = true)]
    pub packages: Vec<String>,
    #[arg(last = true)]
    pub cargo_args: Vec<String>,
}

#[derive(Debug, Args, Clone, Default)]
pub struct CustomCargoCommand {
    pub name: String,
    #[command(flatten)]
    pub cargo: CargoCommandArgs,
}
