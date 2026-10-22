use std::io::{IsTerminal, Read};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::Parser;

use pk::{config, extract, invoke, pick};

/// Pick a match (path, URL, ...) from text and run its rule's command on it.
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// Read text from this file instead of stdin.
    #[arg(long)]
    file: Option<PathBuf>,
    /// Config file [default: ~/.config/pk/config.toml].
    #[arg(long)]
    config: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config_path = cli
        .config
        .or_else(config::default_path)
        .context("cannot locate config: set HOME or pass --config")?;
    let rules = config::load(&config_path)?;
    let text = read_input(cli.file.as_deref())?;

    let matches = extract::extract(&text, &rules, &std::env::current_dir()?);
    if matches.is_empty() {
        bail!("no matches");
    }
    let Some(index) = pick::pick(&matches, &rules)? else {
        return Ok(());
    };
    let m = &matches[index];
    Err(invoke::exec(&rules[m.rule].invoke, m))
}

fn read_input(file: Option<&Path>) -> Result<String> {
    if let Some(path) = file {
        return std::fs::read_to_string(path)
            .with_context(|| format!("reading {}", path.display()));
    }
    let mut stdin = std::io::stdin();
    if stdin.is_terminal() {
        bail!("no input: pipe text into pk or pass --file");
    }
    let mut text = String::new();
    stdin.read_to_string(&mut text)?;
    Ok(text)
}
