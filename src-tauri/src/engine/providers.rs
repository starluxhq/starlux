use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Provider {
    pub id: &'static str,
    pub name: &'static str,
    pub binary: &'static str,
    pub available: bool,
    pub models: &'static [&'static str],
}

const CATALOG: &[(&str, &str, &str, &[&str])] = &[(
    "claude-cli",
    "Claude (subscription)",
    "claude",
    &["opus", "sonnet", "haiku"],
)];

pub fn detect() -> Vec<Provider> {
    CATALOG
        .iter()
        .map(|(id, name, binary, models)| Provider {
            id,
            name,
            binary,
            available: which::which(binary).is_ok(),
            models,
        })
        .collect()
}
