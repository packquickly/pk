use std::fs::File;
use std::os::unix::process::CommandExt;
use std::process::Command;
use std::sync::LazyLock;

use regex::{Captures, Regex};

use crate::extract::Match;

/// `{}` is the whole match, `{name}` a named group, `{name:default}` a group with a fallback.
pub static PLACEHOLDER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{(?P<name>\w*)(?::(?P<default>[^}]*))?\}").unwrap());

/// Fills placeholders in each word of `template`. Each word stays a single argument, so match
/// text is never split or interpreted by a shell. A group that didn't participate and has no
/// default becomes an empty string.
pub fn argv(template: &[String], m: &Match) -> Vec<String> {
    template.iter().map(|word| fill(word, m)).collect()
}

fn fill(word: &str, m: &Match) -> String {
    PLACEHOLDER
        .replace_all(word, |caps: &Captures| {
            let name = &caps["name"];
            let value = if name.is_empty() {
                Some(&m.text)
            } else {
                m.groups.get(name)
            };
            value
                .map(String::as_str)
                .or(caps.name("default").map(|d| d.as_str()))
                .unwrap_or("")
                .to_owned()
        })
        .into_owned()
}

/// Replaces this process with the command. Its stdin is the controlling terminal when there is
/// one, so interactive programs work even when pk's stdin was a pipe. Returns only on failure.
pub fn exec(template: &[String], m: &Match) -> anyhow::Error {
    let argv = argv(template, m);
    let mut command = Command::new(&argv[0]);
    command.args(&argv[1..]);
    if let Ok(tty) = File::open("/dev/tty") {
        command.stdin(tty);
    }
    let err = command.exec();
    anyhow::Error::new(err).context(format!("running {}", argv[0]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn words(template: &str) -> Vec<String> {
        shlex::split(template).unwrap()
    }

    #[test]
    fn fills_groups_and_falls_back_to_defaults() {
        let m = Match {
            rule: 0,
            text: "src/a.rs".into(),
            groups: HashMap::from([("path".into(), "src/a.rs".into())]),
        };

        let argv = argv(&words("nvim +{line:1} {path} --title={}"), &m);

        assert_eq!(argv, ["nvim", "+1", "src/a.rs", "--title=src/a.rs"]);
    }

    #[test]
    fn match_text_stays_one_argument() {
        let m = Match {
            rule: 0,
            text: "https://x.io/a b; rm -rf ~".into(),
            groups: HashMap::new(),
        };

        let argv = argv(&words("open {}"), &m);

        assert_eq!(argv, ["open", "https://x.io/a b; rm -rf ~"]);
    }
}
