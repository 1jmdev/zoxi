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
