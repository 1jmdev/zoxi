#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnvVar {
    key: String,
    value: String,
}

impl EnvVar {
    pub fn parse(input: &str) -> anyhow::Result<Self> {
        let Some((key, value)) = input.split_once('=') else {
            anyhow::bail!("invalid env assignment `{input}`, expected KEY=VALUE")
        };

        let key = key.trim();
        if key.is_empty() {
            anyhow::bail!("invalid env assignment `{input}`, key cannot be empty")
        }

        Ok(Self {
            key: key.to_string(),
            value: value.to_string(),
        })
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}

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

#[derive(Clone, Debug, Default)]
pub struct AddOptions {
    pub packages: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct RemoveOptions {
    pub packages: Vec<String>,
}

#[derive(Clone, Debug)]
pub enum BuildSubcommand {
    Add(AddOptions),
    Build(BuildOptions),
    Clean(CleanOptions),
    Remove(RemoveOptions),
    Run(RunOptions),
    Test(TestOptions),
}
