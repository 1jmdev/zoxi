use std::path::Path;
use std::process::Command;

use crate::cargo::{CargoOptions, EnvVar};

pub struct CargoCommand {
    command: Command,
}

impl CargoCommand {
    pub fn new(workdir: &Path, subcommand: &str) -> Self {
        let mut command = Command::new("cargo");
        command.current_dir(workdir).arg(subcommand);
        Self { command }
    }

    pub fn apply_options(mut self, options: &CargoOptions) -> Self {
        self.apply_envs(&options.envs);
        self.command.args(options.command_args());
        self
    }

    pub fn apply_envs(&mut self, envs: &[EnvVar]) {
        envs.iter().for_each(|env_var| {
            self.command.env(env_var.key(), env_var.value());
        });
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.command
            .args(args.into_iter().map(|arg| arg.as_ref().to_string()));
        self
    }

    pub fn app_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut args = args.into_iter().peekable();
        if args.peek().is_some() {
            self.command.arg("--");
            self.command.args(args.map(|arg| arg.as_ref().to_string()));
        }
        self
    }

    pub fn into_inner(self) -> Command {
        self.command
    }
}
