use crate::cargo::EnvVar;

#[derive(Clone, Debug, Default)]
pub struct CargoOptions {
    pub release: bool,
    pub envs: Vec<EnvVar>,
    pub args: Vec<String>,
}

impl CargoOptions {
    pub fn command_args(&self) -> impl Iterator<Item = &str> {
        self.release
            .then_some("--release")
            .into_iter()
            .chain(self.args.iter().map(String::as_str))
    }
}

#[derive(Clone, Debug, Default)]
pub struct BuildOptions {
    pub cargo: CargoOptions,
}

#[derive(Clone, Debug, Default)]
pub struct RunOptions {
    pub cargo: CargoOptions,
    pub app_args: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct AddOptions {
    pub cargo: CargoOptions,
    pub packages: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct RemoveOptions {
    pub cargo: CargoOptions,
    pub packages: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct TestOptions {
    pub cargo: CargoOptions,
}

#[derive(Clone, Debug, Default)]
pub struct CleanOptions {
    pub cargo: CargoOptions,
}

#[derive(Clone, Debug)]
pub enum CargoSubcommand {
    Add(AddOptions),
    Build(BuildOptions),
    Clean(CleanOptions),
    Remove(RemoveOptions),
    Run(RunOptions),
    Test(TestOptions),
    Custom { name: String, cargo: CargoOptions },
}
