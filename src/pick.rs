use std::io::Write;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

use crate::config::Rule;
use crate::extract::Match;

/// Lets the user choose a match with fzf. Returns its index, or `None` if they cancelled.
pub fn pick(matches: &[Match], rules: &[Rule]) -> Result<Option<usize>> {
    let mut fzf = Command::new("fzf")
        .args(["--delimiter=\t", "--with-nth=2.."])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("running fzf")?;

    let mut stdin = fzf.stdin.take().expect("stdin is piped");
    for (index, m) in matches.iter().enumerate() {
        let text = m.text.replace(['\t', '\n'], " ");
        writeln!(stdin, "{index}\t{}\t{text}", rules[m.rule].name)?;
    }
    drop(stdin);

    let output = fzf.wait_with_output()?;
    match output.status.code() {
        Some(0) => {}
        // 1: nothing matched the query, 130: cancelled.
        Some(1 | 130) => return Ok(None),
        _ => bail!("fzf failed with {}", output.status),
    }
    let line = String::from_utf8(output.stdout)?;
    let index = line.split('\t').next().unwrap_or_default().parse()?;
    Ok(Some(index))
}
