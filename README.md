# pk

Extract matches (paths, URLs, ...) from terminal output with configurable regexes, pick one
with `fzf`, and run a command on it (e.g. open the file in Neovim or the URL in a browser).

Inspired by [exvoker](https://github.com/patrick-kidger/exvoker).

## Install

```sh
cargo install --path .
```

Requires [`fzf`](https://github.com/junegunn/fzf) on `PATH`.

## Usage

```sh
git status | pk             # pick a changed file, open it
rg -n TODO | pk             # jump to file:line
pk --file screen.txt        # read from a file instead of stdin
```

## Config

Rules live in `~/.config/pk/config.toml` (or `$XDG_CONFIG_HOME/pk/config.toml`, or
`--config <path>`). Every rule runs over the input, and the picker shows each match labelled
with its rule's name.

```toml
# Optional: only search the last command's output (the default starship prompt starts with ❯).
prompt = '^❯ '

# Files, optionally with :line (e.g. from git status, rg -n, compiler errors).
[[rule]]
name = "path"
extract = '''(?P<path>[\w./-]+)(?::(?P<line>\d+))?'''
exists = "path"
invoke = "nvim +{line:1} {path}"

# URLs, without trailing punctuation or closing brackets.
[[rule]]
name = "url"
extract = '''https?://[^\s<>"'()\[\]]*[^\s<>"'()\[\].,;:!?]'''
invoke = "open {}"
```

- `prompt` (optional, top level): a regex matching your shell's prompt line. When the input
  contains prompt lines, only the text between the last two is searched: the output of the
  command you just ran. Input without prompt lines (e.g. piped) is searched in full.
- `extract`: a [Rust regex](https://docs.rs/regex/latest/regex/#syntax) (no lookaround).
- `exists` (optional): a named capture group whose value must be an existing file (not a
  directory), relative to the current directory.
- `invoke`: the command to run. `{}` is the whole match, `{name}` a named group, and
  `{name:default}` falls back to `default` when the group didn't match.

`invoke` is split into words like a shell would, but no shell runs it: each placeholder is
filled into its word as a single argument, so matched text is never interpreted. For pipes,
call a shell explicitly: `sh -c 'printf %s "$1" | pbcopy' _ {}`.

## Ghostty: run on what's on screen

Bind a key that dumps the screen to a temp file and runs `pk` on it, so you can run a command
and then pick from its output without piping. Add to `~/.config/ghostty/config`:

```ini
# Visible screen.
keybind = cmd+o=text:pk --file\x20
keybind = chain=write_screen_file:paste
keybind = chain=text:\r

# Entire scrollback.
keybind = cmd+shift+o=text:pk --file\x20
keybind = chain=write_scrollback_file:paste
keybind = chain=text:\r
```

This types into your prompt, so use it from an empty prompt. Requires Ghostty 1.3+.
