use std::path::PathBuf;

use clap::{Parser, Subcommand};

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
    Build {
        #[arg(trailing_var_arg = true)]
        cargo_args: Vec<String>,
    },
    Run {
        #[arg(trailing_var_arg = true)]
        cargo_args: Vec<String>,
    },
}
