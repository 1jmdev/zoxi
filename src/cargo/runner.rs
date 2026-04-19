use std::path::Path;
use std::process::ExitCode;

use crate::cargo::{
    AddOptions, BuildOptions, CargoCommand, CargoOptions, CargoSubcommand, CleanOptions,
    RemoveOptions, RunOptions, TestOptions,
};

pub struct CargoRunner<'a> {
    workdir: &'a Path,
}

impl<'a> CargoRunner<'a> {
    pub fn new(workdir: &'a Path) -> Self {
        Self { workdir }
    }

    pub fn execute(&self, subcommand: &CargoSubcommand) -> anyhow::Result<ExitCode> {
        match subcommand {
            CargoSubcommand::Add(options) => self.add(options),
            CargoSubcommand::Build(options) => self.build(options),
            CargoSubcommand::Clean(options) => self.clean(options),
            CargoSubcommand::Remove(options) => self.remove(options),
            CargoSubcommand::Run(options) => self.run(options),
            CargoSubcommand::Test(options) => self.test(options),
            CargoSubcommand::Custom { name, cargo } => self.custom(name, cargo),
        }
    }

    pub fn add(&self, options: &AddOptions) -> anyhow::Result<ExitCode> {
        if options.packages.is_empty() {
            anyhow::bail!("cargo add requires at least one package")
        }

        self.run_command(
            "add",
            CargoCommand::new(self.workdir, "add")
                .apply_options(&options.cargo)
                .args(&options.packages),
        )
    }

    pub fn build(&self, options: &BuildOptions) -> anyhow::Result<ExitCode> {
        self.run_command(
            "build",
            CargoCommand::new(self.workdir, "build").apply_options(&options.cargo),
        )
    }

    pub fn clean(&self, options: &CleanOptions) -> anyhow::Result<ExitCode> {
        self.run_command(
            "clean",
            CargoCommand::new(self.workdir, "clean").apply_options(&options.cargo),
        )
    }

    pub fn remove(&self, options: &RemoveOptions) -> anyhow::Result<ExitCode> {
        if options.packages.is_empty() {
            anyhow::bail!("cargo remove requires at least one package")
        }

        self.run_command(
            "remove",
            CargoCommand::new(self.workdir, "remove")
                .apply_options(&options.cargo)
                .args(&options.packages),
        )
    }

    pub fn run(&self, options: &RunOptions) -> anyhow::Result<ExitCode> {
        self.run_command(
            "run",
            CargoCommand::new(self.workdir, "run")
                .apply_options(&options.cargo)
                .app_args(&options.app_args),
        )
    }

    pub fn test(&self, options: &TestOptions) -> anyhow::Result<ExitCode> {
        self.run_command(
            "test",
            CargoCommand::new(self.workdir, "test").apply_options(&options.cargo),
        )
    }

    pub fn custom(&self, name: &str, cargo: &CargoOptions) -> anyhow::Result<ExitCode> {
        self.run_command(name, CargoCommand::new(self.workdir, name).apply_options(cargo))
    }

    fn run_command(&self, name: &str, command: CargoCommand) -> anyhow::Result<ExitCode> {
        let status = command.into_inner().status()?;
        if status.success() {
            Ok(ExitCode::SUCCESS)
        } else {
            anyhow::bail!("cargo {name} failed with status {status}")
        }
    }
}
