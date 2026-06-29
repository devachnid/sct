# sct completions

Install or print shell completion scripts. Supports bash, zsh, fish, PowerShell, and elvish.

---

## Usage

```
sct completions <SHELL>
sct completions --dir <PATH> <SHELL>
sct completions install [--shell <SHELL>] [--dir <PATH>]
```

## Arguments

| Argument | Description |
|---|---|
| `<SHELL>` | One of: `bash`, `zsh`, `fish`, `powershell`, `elvish` |

---

## Installation

For the current user, prefer the installer:

```bash
sct completions install
```

It detects your shell, writes the correctly named completion file, and prints any one-time shell setup still needed. Re-run it after upgrading `sct` if your package manager or installer did not refresh completions for you.

### bash

```bash
mkdir -p ~/.local/share/bash-completion/completions
sct completions --dir ~/.local/share/bash-completion/completions bash
```

Or system-wide:

```bash
sct completions --dir /etc/bash_completion.d bash
```

Reload with `source ~/.bashrc` or open a new shell.

### zsh

```zsh
mkdir -p ~/.zfunc
sct completions --dir ~/.zfunc zsh
```

Ensure `~/.zfunc` is on `$fpath` - add this to `~/.zshrc` **before** `compinit`:

```zsh
fpath=(~/.zfunc $fpath)
autoload -Uz compinit && compinit
```

Then open a new shell or run `exec zsh`.

### fish

```fish
sct completions --dir ~/.config/fish/completions fish
```

Takes effect immediately in new fish sessions.

### PowerShell

```powershell
sct completions --dir ~/.config/powershell/completions powershell
```

Reload with `. $PROFILE` or open a new PowerShell session.

### elvish

```elvish
sct completions --dir ~/.elvish/lib elvish
```

---

## Example

```bash
$ sct completions install --shell zsh
$ exec zsh
$ sct <TAB>
codelist     completions  diff         embed        gui          info
lexical      markdown     mcp          ndjson       parquet      semantic
sqlite       tui
```
