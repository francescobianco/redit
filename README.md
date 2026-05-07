# redit

`redit` is a Rust clone of the MS-DOS EDIT text editor. The original DOS
binary used as reference is kept in `dos/EDIT/V1/EDIT.COM` together with
`QBASIC.EXE` and `EDIT.HLP`.

## Comparing redit with MS-DOS EDIT

Use `tmux` as a deterministic terminal harness and `dosemu` to run the original
binary. Keep both panes at the same size, then capture their text and ANSI
attributes.

### Start both editors

From the repository root:

```sh
tmux kill-session -t redit_dos
tmux kill-session -t redit_clone

tmux new-session -d -s redit_dos -x 80 -y 25 \
  dosemu -t -K "$PWD/dos/EDIT/V1" -E EDIT.COM

tmux new-session -d -s redit_clone -x 80 -y 25 \
  cargo run
```

If `tmux kill-session` reports that the session does not exist, ignore it.

### Capture layout

Plain text capture:

```sh
tmux capture-pane -t redit_dos -p
tmux capture-pane -t redit_clone -p
```

Capture with terminal attributes and colors:

```sh
tmux capture-pane -t redit_dos -e -p
tmux capture-pane -t redit_clone -e -p
```

The `-e` form is the important one for palette work. In the original V1 EDIT
capture, the relevant SGR codes observed at 80x25 are:

```text
30;47  black on light gray: menu bar, scroll arrows/thumb, title highlight
37;44  white on blue: editor background and most frame text
34;47  blue on light gray: filename in the top border
37;40  white on black: selected menu title and selected menu row
97;46  bright white on cyan: status bar help text
90;40  dark gray on black: drop shadow / disabled-looking menu text
```

When comparing colors, do not rely on screenshots first. Compare the ANSI
attribute stream from `tmux capture-pane -e -p`, then use screenshots only as a
visual sanity check.

### Send keys and verify states

Use `tmux send-keys` to drive both programs through the same interaction:

```sh
tmux send-keys -t redit_dos C-[
tmux send-keys -t redit_clone Escape

tmux send-keys -t redit_dos M-f
tmux send-keys -t redit_clone M-f

tmux send-keys -t redit_dos Right
tmux send-keys -t redit_clone Right

tmux send-keys -t redit_dos abc Enter def Left Left
tmux send-keys -t redit_clone abc Enter def Left Left
```

Known key transport notes:

- `M-f` opens the File menu in the original and in `redit`.
- `F10` opens the menu in `redit`, but did not activate the V1 DOS EDIT menu in
  the observed DOSEMU terminal run. The original status bar says `Press ALT to
  activate menus`.
- `C-[` cleared the original welcome dialog in DOSEMU. `Escape` cleared the
  `redit` welcome dialog.
- Validate special tmux key names before trusting a test. For example,
  `tmux send-keys Backspace` was delivered as literal text `Backspace` in the
  observed run. Prefer testing candidates such as `BSpace`, `C-h`, and `C-?`
  against the captured editor contents before recording behavior.

Always capture after each key step:

```sh
tmux capture-pane -t redit_dos -e -p
tmux capture-pane -t redit_clone -e -p
```

### Observed V1 differences to replicate

Initial welcome state:

- Original filename title is `Untitled`; `redit` currently uses `UNTITLED1`.
- Original menu bar spacing places `Help` at the far right. `redit` renders all
  menu names consecutively.
- Original welcome dialog text is:
  `Welcome to the MS-DOS Editor`,
  `Copyright (C) Microsoft Corporation, 1987-1992.`,
  `All rights reserved.`,
  and `< Press Enter to see the Survival Guide >`.
- Original welcome dialog is 58 columns wide inside the border and starts lower
  than the current `redit` dialog.
- Original status bar in welcome mode is:
  `F1=Help   Enter=Execute   Esc=Cancel   Tab=Next Field   Arrow=Next Item`.
  `redit` omits the Tab and Arrow hints.

Normal editor state:

- Original status bar left side is
  `MS-DOS Editor  <F1=Help> Press ALT to activate menus`.
- Original cursor position format is `00001:001`; `redit` currently uses
  `Ln:   1  Col:  1`.
- Original uses bright white on cyan for most status help and black on gray for
  the right cursor-position field separator/area.
- Original horizontal scrollbar has a blank light-gray cell after the left
  arrow: `|<- space -░...->|`. `redit` currently starts the track immediately
  after `←`.
- Original scrollbar arrows/thumb are black on light gray, not white on blue.

File menu:

- Original File menu contains `New`, `Open...`, `Save`, `Save As...`,
  separator, `Print...`, separator, `Exit`.
- `redit` currently omits `Print...`.
- Original has no `Ctrl+S` shortcut text in this menu.
- Original menu item accelerator letters are bright white; the rest is black on
  light gray.
- Original selected menu row is white on black and updates status help. For
  `New`, the status help reads `Removes currently loaded file from memory`.

Edit menu:

- Original shortcuts are `Shift+Del`, `Ctrl+Ins`, `Shift+Ins`, and `Del`.
- `redit` currently shows `Ctrl+X`, `Ctrl+C`, and `Ctrl+V`.
- Original disabled-looking unavailable items render dark gray where
  appropriate.
- Original selected `Cut` status help reads
  `Deletes selected text and copies it to buffer`.

Search menu:

- Original V1 Search menu contains only `Find...`, `Repeat Last Find`, and
  `Change...`.
- `redit` currently also includes `Go To Line...`.
- Original `Repeat Last Find` shows only `F3`.
- Original selected `Find...` status help reads `Finds specified text`.

Workflow for adding more comparisons:

1. Start fresh `redit_dos` and `redit_clone` sessions at 80x25.
2. Drive the same state with `tmux send-keys`.
3. Capture both with `tmux capture-pane -e -p`.
4. Record the exact key sequence, the visible text difference, and the ANSI
   color/attribute difference.
5. Update `redit` only after the original state is captured, so behavior changes
   can be verified against the same script.
