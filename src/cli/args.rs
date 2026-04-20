use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "zoxi")]
#[command(about = "Transpile Zoxi sources into Rust and compile with rustc")]
pub struct Cli {
    #[arg(long, short, global = true)]
    pub path: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Build(RustcCommandArgs),
    Clean,
    Run(RunCommand),
    Test(RustcCommandArgs),
}

#[derive(Debug, Args, Clone, Default)]
pub struct RustcCommandArgs {
    #[arg(short = 'r', long = "release")]
    pub release: bool,
    #[arg(short = 'e', long = "env", value_name = "KEY=VALUE")]
    pub env: Vec<String>,
    #[arg(allow_hyphen_values = true)]
    pub rustc_args: Vec<String>,
}

#[derive(Debug, Args, Clone, Default)]
pub struct RunCommand {
    #[command(flatten)]
    pub rustc: RustcCommandArgs,
    #[arg(last = true)]
    pub app_args: Vec<String>,
}
