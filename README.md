# pk

Extract matches (paths, URLs, ...) from terminal output with configurable regexes, pick one
with `fzf`, and run a command on it (e.g. open the file in Neovim or the URL in a browser).

Inspired by [exvoker](https://github.com/patrick-kidger/exvoker).

## Install

```sh
cargo install --path .
```

Requires [`fzf`](https://github.com/junegunn/fzf) on `PATH`.
