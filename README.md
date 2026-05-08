# redit

`redit` is a Rust clone of the MS-DOS EDIT text editor. The original DOS
binary used as reference is kept in `dos/EDIT/V1/EDIT.COM` together with
`QBASIC.EXE` and `EDIT.HLP`.

---

## Quick start

```sh
cargo run                      # open empty editor (V1 style)
cargo run -- myfile.txt        # open a file
cargo run -- --v2 myfile.txt   # open with V2 style
make open-v1                   # run original MS-DOS EDIT V1 via dosemu
```

---

## Gherkin test framework

The test suite compares `redit` against the original `EDIT.COM` running
inside `dosemu`. It uses `tmux` as a deterministic terminal harness.

### Directory layout

```
tests/
├── run.sh                  # Gherkin runner (bash)
├── features/               # .feature files — one per topic
│   ├── 01_welcome.feature
│   ├── 02_typing.feature
│   ├── 03_navigation.feature
│   ├── 04_file_menu.feature
│   ├── 05_edit_menu.feature
│   ├── 06_search_menu.feature
│   ├── 07_help.feature
│   ├── 08_save_exit.feature
│   ├── 09_status_bar.feature
│   └── 10_display_options.feature
└── snapshots/              # golden captures from dosemu (git-tracked)
    └── <feature>/<scenario>/<NNN>.txt|.ansi
```

### Workflow

**Step 1 — record golden snapshots from the original:**

```sh
make snapshot
```

This starts `dosemu` + `redit` side by side, drives both through every
feature file, and saves the dosemu output as `tests/snapshots/**/*.txt`
(plain text) and `tests/snapshots/**/*.ansi` (ANSI color stream).
Commit the snapshot files so CI can run without `dosemu`.

**Step 2 — run tests against the clone:**

```sh
make test
```

Starts `redit`, drives it through every feature file, and `diff`s the
output against the golden snapshots. A non-zero exit means at least one
step failed.

**Single feature:**

```sh
make test-feature FEATURE=04_file_menu
make snapshot-feature FEATURE=04_file_menu
```

### Writing a new feature

Create `tests/features/NN_topic.feature`. Use standard Gherkin syntax.
Available step vocabulary:

#### Given / setup

| Step | Effect |
|---|---|
| `both editors are started fresh` | kill + restart both tmux sessions (80×25) |
| `both editors are started with file "<name>"` | open a specific file in both |
| `the original editor is started` | start only dosemu |
| `the clone is started` | start only redit |

#### When / input

| Step | Effect |
|---|---|
| `I send "<keys>" to both` | `tmux send-keys` to both sessions |
| `I send "<keys>" to the clone` | `tmux send-keys` to redit only |
| `I send "<keys>" to the original` | `tmux send-keys` to dosemu only |
| `I wait <n> seconds` | `sleep n` |
| `I wait for the editors to settle` | `sleep 1` |

`<keys>` is passed verbatim to `tmux send-keys`. Use tmux key names:
`Enter`, `Escape`, `BSpace`, `DC` (Delete), `IC` (Insert), `Up`, `Down`,
`Left`, `Right`, `Home`, `End`, `PgUp`, `PgDn`, `F1`…`F12`,
`M-f` (Alt+F), `C-s` (Ctrl+S), `C-Home`, `C-End`, `C-Left`, `C-Right`.
Literal text is just quoted: `"hello world"`.

Multiple tokens in one step: `"abc Enter def Left"`.

#### Then / assertions

| Step | Effect |
|---|---|
| `the screen text matches` | diff clone text against golden `.txt` |
| `the screen colors match` | diff clone ANSI stream against golden `.ansi` |
| `the screen matches` | text + color diff |
| `the clone screen contains "<text>"` | grep for text in clone output |
| `the clone screen does not contain "<text>"` | inverse grep |
| `the clone shows "<text>" on line <N>` | check specific screen line |

---

## Comparing redit with MS-DOS EDIT manually

Use `tmux` as a deterministic terminal harness and `dosemu` to run the
original binary. Keep both panes at 80×25, then capture their text and ANSI
attributes.

### Start both editors

```sh
# Start the original (from repo root)
tmux kill-session -t redit_dos   2>/dev/null; \
tmux new-session  -d -s redit_dos -x 80 -y 25 \
  "dosemu -t -K '$PWD/dos/EDIT/V1' -E EDIT.COM 2>/dev/null"

# Start the clone
tmux kill-session -t redit_clone 2>/dev/null; \
tmux new-session  -d -s redit_clone -x 80 -y 25 \
  "cargo run -q 2>/dev/null"
```

Or use the Makefile shortcuts:

```sh
make sessions       # start both sessions
make kill-sessions  # stop both
```

### Capture screen content

Plain text (for layout comparison):

```sh
tmux capture-pane -t redit_dos   -p
tmux capture-pane -t redit_clone -p
```

Full ANSI escape sequences (for color comparison):

```sh
tmux capture-pane -t redit_dos   -e -p
tmux capture-pane -t redit_clone -e -p
```

Save and diff them:

```sh
tmux capture-pane -t redit_dos   -p > /tmp/orig.txt
tmux capture-pane -t redit_clone -p > /tmp/clone.txt
diff /tmp/orig.txt /tmp/clone.txt
```

### Send key input

```sh
# Send keys to both sessions simultaneously
tmux send-keys -t redit_dos   "M-f"   ; tmux send-keys -t redit_clone "M-f"
tmux send-keys -t redit_dos   Enter   ; tmux send-keys -t redit_clone Enter
tmux send-keys -t redit_dos   Escape  ; tmux send-keys -t redit_clone Escape

# Type literal text
tmux send-keys -t redit_dos   "hello world"
tmux send-keys -t redit_clone "hello world"

# Special keys
tmux send-keys -t redit_dos   Up Down Left Right Home End PgUp PgDn
tmux send-keys -t redit_clone Up Down Left Right Home End PgUp PgDn

tmux send-keys -t redit_dos   DC        # Delete key
tmux send-keys -t redit_clone DC

tmux send-keys -t redit_dos   IC        # Insert key
tmux send-keys -t redit_clone IC
```

Always capture after each step to record before/after state.

### Known tmux key transport notes

- `M-f` opens the File menu in both the original and `redit`.
- `Escape` closes dialogs in `redit`. Use `Escape` for both in tests.
- `BSpace` is the correct tmux name for the Backspace key.
- `DC` is the correct tmux name for the Delete key.
- `IC` is the correct tmux name for the Insert key.
- Literal key names like `Backspace` or `Delete` are sent as text, not
  as control keys. Always use the tmux canonical names.
- `C-[` (Ctrl+[) also sends ESC — useful when `Escape` is consumed by tmux.

### SGR color palette observed in the original V1

Decoded from `tmux capture-pane -e -p` on dosemu at 80×25:

```
[30m][47m]   Black fg / Light Gray bg   menu bar, all V1 dialog boxes, scroll arrows
[37m][44m]   White fg / Blue bg         editor text area and frame borders
[34m][47m]   Blue fg / Light Gray bg    filename in the top border
[37m][40m]   White fg / Black bg        selected menu title, selected menu row
[97m][46m]   Bright White / Cyan bg     status bar
[90m][40m]   Dark Gray / Black bg       drop shadow on all dialogs and menus
[97m]        Bright White fg (on Gray)  < > characters in V1 dialog buttons
```

When diagnosing a color difference: compare the SGR sequence per cell,
not just the visual screenshot. Run `capture-pane -e -p` and inspect the
stream directly.

---

## Observed V1 differences to replicate

### Initial welcome state

- Original filename title is `Untitled`; `redit` uses `Untitled`.
- Original menu bar places `Help` at the far right. `redit` matches this.
- Original welcome dialog body: Gray bg / Black fg. `redit` matches.
- Original `< Press Enter >` row: only `<` and `>` are Bright White;
  the rest of the row is Black on Gray. `redit` matches.
- Original status bar in welcome mode:
  `F1=Help   Enter=Execute   Esc=Cancel   Tab=Next Field   Arrow=Next Item`.

### Normal editor state

- Original status bar: `MS-DOS Editor  <F1=Help> Press ALT to activate menus`.
- Original cursor position format: `00001:001` (five digits colon three digits).
- Original horizontal scrollbar: blank cell after left arrow `← ░░░ →`.
- Original scrollbar arrows/thumb: Black on Light Gray.

### File menu

- Contains: `New`, `Open...`, `Save`, `Save As...`, separator, `Print...`,
  separator, `Exit`.
- No `Ctrl+S` shortcut text.
- Accelerator letters are Bright White; rest is Black on Light Gray.
- Selected row: White on Black.

### Edit menu

- Shortcuts: `Shift+Del`, `Ctrl+Ins`, `Shift+Ins`, `Del`.

### Search menu

- Items: `Find...`, `Repeat Last Find` (`F3`), `Change...`.
- No `Go To Line...` entry.

---

## Adding more comparisons

1. Start fresh sessions: `make sessions`
2. Drive the desired state with `tmux send-keys`.
3. Capture both with `tmux capture-pane -e -p`.
4. Record the key sequence, text difference, and SGR color difference.
5. Write a new `Scenario:` in the relevant `.feature` file.
6. Run `make snapshot-feature FEATURE=NN_topic` to save the golden file.
7. Run `make test-feature FEATURE=NN_topic` to verify the clone.
