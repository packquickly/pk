use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use regex::Regex;
use serde::Deserialize;

use crate::invoke::PLACEHOLDER;

#[derive(Debug)]
pub struct Rule {
    pub name: String,
    pub regex: Regex,
    /// Capture group whose value must be an existing path for a match to count.
    pub exists: Option<String>,
    /// Command words, each possibly containing placeholders (see [`crate::invoke`]).
    pub invoke: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    #[serde(rename = "rule", default)]
    rules: Vec<RawRule>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRule {
    name: String,
    extract: String,
    exists: Option<String>,
    invoke: String,
}

/// `$XDG_CONFIG_HOME/pk/config.toml`, falling back to `~/.config/pk/config.toml`.
pub fn default_path() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .map(|dir| dir.join("pk").join("config.toml"))
}

pub fn load(path: &Path) -> Result<Vec<Rule>> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    parse(&text).with_context(|| format!("parsing {}", path.display()))
}

pub fn parse(text: &str) -> Result<Vec<Rule>> {
    let raw: RawConfig = toml::from_str(text)?;
    raw.rules.into_iter().map(compile).collect()
}

fn compile(raw: RawRule) -> Result<Rule> {
    let regex =
        Regex::new(&raw.extract).with_context(|| format!("rule {:?}: invalid regex", raw.name))?;
    if let Some(group) = &raw.exists
        && !has_group(&regex, group)
    {
        bail!(
            "rule {:?}: `exists` names unknown capture group {group:?}",
            raw.name
        );
    }
    let invoke = parse_invoke(&raw.invoke, &regex)
        .with_context(|| format!("rule {:?}: invalid `invoke`", raw.name))?;
    Ok(Rule {
        name: raw.name,
        regex,
        exists: raw.exists,
        invoke,
    })
}

fn parse_invoke(template: &str, regex: &Regex) -> Result<Vec<String>> {
    let words = shlex::split(template).context("unbalanced quotes")?;
    if words.is_empty() {
        bail!("empty command");
    }
    let unknown = words
        .iter()
        .flat_map(|word| PLACEHOLDER.captures_iter(word))
        .map(|caps| caps["name"].to_owned())
        .find(|name| !name.is_empty() && !has_group(regex, name));
    if let Some(name) = unknown {
        bail!("placeholder {{{name}}} names unknown capture group");
    }
    Ok(words)
}

fn has_group(regex: &Regex, group: &str) -> bool {
    regex.capture_names().flatten().any(|name| name == group)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exists_must_name_a_capture_group() {
        let err = parse(
            r#"
            [[rule]]
            name = "path"
            extract = '(?P<file>\S+)'
            exists = "path"
            invoke = "nvim {file}"
            "#,
        )
        .unwrap_err();
        assert!(format!("{err:#}").contains("unknown capture group"));
    }
}
