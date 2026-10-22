use std::collections::{HashMap, HashSet};
use std::path::Path;

use regex::Regex;

use crate::config::Rule;

/// The text between the last two lines matching `prompt`, i.e. the output of the last command
/// when the final prompt line is the one pk was run from. With a single prompt line, the text
/// before it; with none, all of `text`.
pub fn last_output<'a>(text: &'a str, prompt: &Regex) -> &'a str {
    let prompts: Vec<(usize, usize)> = line_spans(text)
        .filter(|&(start, end)| prompt.is_match(text[start..end].trim_end_matches(['\n', '\r'])))
        .collect();
    match prompts.as_slice() {
        [] => text,
        [only] => &text[..only.0],
        [.., previous, last] => &text[previous.1..last.0],
    }
}

/// Byte ranges of each line, including its line terminator.
fn line_spans(text: &str) -> impl Iterator<Item = (usize, usize)> + '_ {
    text.split_inclusive('\n').scan(0, |offset, line| {
        let start = *offset;
        *offset += line.len();
        Some((start, *offset))
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match {
    /// Index into the rules passed to [`extract`].
    pub rule: usize,
    pub text: String,
    /// Named capture groups that participated in the match.
    pub groups: HashMap<String, String>,
}

/// Non-empty matches of all rules in order of appearance, deduplicated per rule.
/// Relative paths checked by a rule's `exists` group resolve against `base`.
pub fn extract(text: &str, rules: &[Rule], base: &Path) -> Vec<Match> {
    let mut found: Vec<(usize, Match)> = rules
        .iter()
        .enumerate()
        .flat_map(|(index, rule)| rule_matches(text, index, rule))
        .filter(|(_, m)| !m.text.is_empty() && passes_exists(m, &rules[m.rule], base))
        .collect();
    found.sort_by_key(|(start, m)| (*start, m.rule));

    let mut seen = HashSet::new();
    found
        .into_iter()
        .map(|(_, m)| m)
        .filter(|m| seen.insert((m.rule, m.text.clone())))
        .collect()
}

fn rule_matches<'a>(
    text: &'a str,
    index: usize,
    rule: &'a Rule,
) -> impl Iterator<Item = (usize, Match)> + 'a {
    rule.regex.captures_iter(text).map(move |caps| {
        let whole = caps.get(0).expect("group 0 always participates");
        let groups = rule
            .regex
            .capture_names()
            .flatten()
            .filter_map(|name| {
                caps.name(name)
                    .map(|g| (name.to_owned(), g.as_str().to_owned()))
            })
            .collect();
        let m = Match {
            rule: index,
            text: whole.as_str().to_owned(),
            groups,
        };
        (whole.start(), m)
    })
}

fn passes_exists(m: &Match, rule: &Rule, base: &Path) -> bool {
    match &rule.exists {
        None => true,
        Some(group) => m
            .groups
            .get(group)
            .is_some_and(|path| base.join(path).is_file()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::parse;

    const RULES: &str = r#"
        [[rule]]
        name = "url"
        extract = '''https?://[^\s<>"'()]+'''
        invoke = "open {}"

        [[rule]]
        name = "path"
        extract = '''(?P<path>[\w./-]+)(?::(?P<line>\d+))?'''
        exists = "path"
        invoke = "nvim {path}"
    "#;

    #[test]
    fn last_output_is_between_the_last_two_prompts() {
        let prompt = Regex::new("^❯ ").unwrap();
        let screen = "❯ ls\nCargo.toml\nsrc\n\
                      pk on main\n❯ git status\n\tmodified: src/main.rs\n\
                      pk on main\n❯ pk --file ";

        assert_eq!(
            last_output(screen, &prompt),
            "\tmodified: src/main.rs\npk on main\n"
        );
        assert_eq!(last_output("out\n❯ pk", &prompt), "out\n");
        assert_eq!(last_output("no prompts\n", &prompt), "no prompts\n");
    }

    fn texts(matches: &[Match]) -> Vec<&str> {
        matches.iter().map(|m| m.text.as_str()).collect()
    }

    #[test]
    fn keeps_only_existing_files_from_git_status() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/main.rs"), "").unwrap();
        let status = "On branch main\n\
                      Changes not staged for commit:\n\
                      \tmodified:   src/main.rs\n\
                      \tdeleted:    gone.rs\n\
                      Untracked files: src/ . /\n";

        let matches = extract(status, &parse(RULES).unwrap().rules, dir.path());

        assert_eq!(texts(&matches), ["src/main.rs"]);
    }

    #[test]
    fn optional_line_group_is_absent_when_unmatched() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.rs"), "").unwrap();

        let matches = extract(
            "a.rs:12 then a.rs",
            &parse(RULES).unwrap().rules,
            dir.path(),
        );

        assert_eq!(texts(&matches), ["a.rs:12", "a.rs"]);
        assert_eq!(
            matches[0].groups.get("line").map(String::as_str),
            Some("12")
        );
        assert!(!matches[1].groups.contains_key("line"));
    }

    #[test]
    fn orders_by_appearance_and_dedupes_within_rule() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("foo.rs"), "").unwrap();

        let matches = extract(
            "see https://x.io then foo.rs, again https://x.io",
            &parse(RULES).unwrap().rules,
            dir.path(),
        );

        assert_eq!(texts(&matches), ["https://x.io", "foo.rs"]);
        assert_eq!(matches.iter().map(|m| m.rule).collect::<Vec<_>>(), [0, 1]);
    }
}
