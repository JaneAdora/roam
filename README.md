# roam

Mobile-friendly file browser TUI. Third widget in the `wt` / `recall` / `roam` suite.

The value isn't browsing — it's the single-key exit that drops you into Claude Code, `$EDITOR`, or a fresh shell rooted in the focused directory. Designed for Termux on a phone, SSH from Blink/Termius, and tiled dashboards of terminals.

## Build

```
cargo build --release
install -m 0755 target/release/roam ~/.local/bin/roam
```

## Usage

```
roam                # open at $ROAM_ROOT or $HOME
roam /some/path     # open at PATH
roam --resume       # open at the last-visited dir
eval "$(roam)"      # parent shell evaluates the exit command (cd / shell / claude)
```

## Keys

Footer advertises five essentials only. Press `?` inside `roam` for the full keymap.

```
Enter  action menu      j/k or arrows  move
/      filter           ?              help
q      quit             Ctrl-C         quit
```

Direct-key shortcuts (also in `?`):

```
o   cd-and-exit            c   launch claude
D   claude --danger (2x)   s   new shell here
t   new terminal tab tmux  y   copy path (OSC 52, 4 KiB cap)
e   open in $EDITOR        Y   copy file contents (4 KiB cap)
.   toggle hidden          p   toggle preview pane
r   refresh                1-9 jump bookmark
```

## Bookmarks

Edit `~/.config/roam/bookmarks.toml`:

```toml
[[bookmark]]
key = "1"
label = "projects"
path = "~/projects"

[[bookmark]]
key = "2"
label = "vaults"
path = "~/vaults"
```

Number keys 1-9 jump to the matching slot. The pinned section is visible above the entry list when width ≥ 40.

## Mobile / SSH notes

- **OSC 52 clipboard.** Works in Termux, Blink, kitty, iTerm2, alacritty, and tmux ≥ 3.2 (`set -g set-clipboard on`). Hard-capped at 4 KiB raw bytes per Termux's pty buffer limit; copying larger files shows a "truncated N/total" toast.
- **`t` (new terminal tab).** Requires tmux. Outside tmux, falls back to `s` (print `cd <path> && exec $SHELL`). The Termux `am start` approach doesn't honor cwd, so we don't bother.
- **Soft keyboard ergonomics.** All bindings are single keys, no chords or backslash. The footer hint shows only five keys to keep visual noise low.
- **Vi keys and arrows both work** — important for speech-to-text input.

## Shell integration

Drop the binary's stdout into `eval`:

```
# in ~/.bashrc or ~/.zshrc
roam() {
    local cmd
    cmd=$(command roam "$@") || return
    [[ -n "$cmd" ]] && eval "$cmd"
}
```

Or just run standalone and copy/paste the printed command.

## Config files

- `~/.config/roam/bookmarks.toml` — bookmark slots
- `~/.config/roam/state.json` — persisted UI state (last dir, hidden toggle, preview on/off)

## Suite

- `wt` — worktree wizard (sibling)
- `recall` — session browser (sibling)
- More planned: `gst`, `ssh`, `note`, `clip`, `net`, `op`, `gh`, `proc`, `port`. See `~/.claude/plans/jolly-crunching-teacup.md`.

## License

Private.
