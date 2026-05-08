# Settings Task Notes

## Scope

Implement editor configuration support exposed from the V1 `Options` menu.

In MS-DOS EDIT V1 the `Options` menu contains a `Display` item. That item opens
the editor configuration dialog. `redit` needs to replicate that flow and store
the selected settings persistently.

## Requirements

- Add the V1 `Options -> Display` menu entry.
- Open a configuration popup/dialog from `Options -> Display`.
- Support display/editor configuration values from that dialog.
- Include color configuration support.
- Include the non-color options exposed by the original dialog.
- Persist configuration globally for the current user.
- Store user configuration in `$HOME/.reditrc`.
- Use INI format for `$HOME/.reditrc`.
- Allow choosing the default visual style/theme, including at least `V1` and
  `V2`.
- Load `$HOME/.reditrc` on startup.
- Apply loaded settings before rendering the initial UI.
- Save changes back to `$HOME/.reditrc` when the user confirms the settings
  dialog.

## Proposed INI Shape

```ini
[editor]
style=V1

[colors]
menu_fg=black
menu_bg=gray
editor_fg=white
editor_bg=blue
status_fg=white
status_bg=cyan
dialog_fg=black
dialog_bg=cyan
title_fg=black
title_bg=gray
scrollbar_fg=black
scrollbar_bg=gray
```

Implemented first pass:

- `$HOME/.reditrc` is loaded on startup.
- `$HOME/.reditrc` is saved when `Options -> Display` is confirmed.
- Supported style values: `V1`, `V2`.
- Supported color values: `black`, `blue`, `green`, `cyan`, `red`, `magenta`,
  `yellow`, `gray`, `dark_gray`, `light_blue`, `light_green`, `light_cyan`,
  `light_red`, `light_magenta`, `light_yellow`, `white`.
- The current popup exposes style plus menu/editor/status/dialog/title/scrollbar
  foreground and background colors.

## Verification Plan

1. Open original V1 EDIT at 80x25.
2. Navigate to `Options -> Display`.
3. Capture text and ANSI attributes with `tmux capture-pane -e -p`.
4. Record all labels, controls, default values, buttons, accelerators, colors,
   and focus behavior.
5. Implement the matching `redit` dialog.
6. Verify against the same 80x25 capture.
7. Verify persistence by saving settings, restarting `redit`, and confirming the
   initial UI uses `$HOME/.reditrc`.

## Open Questions

- Exact list of original V1 display settings.
- Exact color palette names and available foreground/background combinations.
- Whether `.reditrc` should preserve unknown keys/comments.
- Whether command-line flags such as `--v1` and `--v2` should override
  `.reditrc`, or only provide defaults when the file is absent.

Current behavior: `--v1` and `--v2` override `.reditrc` for that launch and reset
the editable color draft to that style's defaults.
