use crate::build::EnvVar;

#[derive(Clone, Debug, Default)]
pub struct RustcOptions {
    pub release: bool,
    pub envs: Vec<EnvVar>,
    pub args: Vec<String>,
}

impl RustcOptions {
    pub fn command_args(&self) -> impl Iterator<Item = &str> {
        self.args.iter().map(String::as_str)
    }
}

#[derive(Clone, Debug, Default)]
pub struct BuildOptions {
    pub rustc: RustcOptions,
}

#[derive(Clone, Debug, Default)]
pub struct RunOptions {
    pub rustc: RustcOptions,
    pub app_args: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct TestOptions {
    pub rustc: RustcOptions,
}

#[derive(Clone, Debug, Default)]
pub struct CleanOptions;

#[derive(Clone, Debug)]
pub enum BuildSubcommand {
    Build(BuildOptions),
    Clean(CleanOptions),
    Run(RunOptions),
    Test(TestOptions),
}
