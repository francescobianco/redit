#!/usr/bin/env bash
# tests/run.sh — run Gherkin feature files against ONE editor target
#
# Usage:
#   EDITOR_TARGET=clone    bash tests/run.sh [features...]
#   EDITOR_TARGET=original bash tests/run.sh [features...]
#
# EDITOR_TARGET=clone    → runs redit (cargo run)
# EDITOR_TARGET=original → runs EDIT.COM via dosemu
#
# Each "Then the screen …" step saves a capture under:
#   tests/captures/<target>/<feature>/<scenario>/<NNN>.txt
#   tests/captures/<target>/<feature>/<scenario>/<NNN>.ansi
#
# The compare.sh script runs this twice (original + clone) then diffs.

set -euo pipefail
REPO=$(cd "$(dirname "$0")/.." && pwd)

TARGET="${EDITOR_TARGET:-clone}"   # clone | original
SESSION="redit_test"
TERM_W="${TERM_W:-80}"
TERM_H="${TERM_H:-25}"
CAPTURES="$REPO/tests/captures/$TARGET"
PASS=0; FAIL=0

# ── Terminal output ────────────────────────────────────────────────────────────
G='\e[0;32m'; R='\e[0;31m'; Y='\e[0;33m'; C='\e[0;36m'
B='\e[1m'; DIM='\e[2m'; N='\e[0m'

# ── Start the editor under test ────────────────────────────────────────────────
_start_editor() {
    local arg="${1:-}"
    tmux kill-session -t "$SESSION" 2>/dev/null || true
    case "$TARGET" in
      original)
        tmux new-session -d -s "$SESSION" -x "$TERM_W" -y "$TERM_H" \
          "dosemu -t -K '$REPO/dos/EDIT/V1' -E '${arg:-EDIT.COM}' 2>/dev/null" ;;
      clone)
        tmux new-session -d -s "$SESSION" -x "$TERM_W" -y "$TERM_H" \
          "cd '$REPO'; cargo run -q -- $arg 2>/dev/null" ;;
      *)
        echo "Unknown EDITOR_TARGET: $TARGET"; exit 1 ;;
    esac
}

_stop_editor() {
    tmux kill-session -t "$SESSION" 2>/dev/null || true
}

# ── Input helpers ──────────────────────────────────────────────────────────────
_send() {
    eval "tmux send-keys -t '$SESSION' $*"
}

# ── Capture helpers ────────────────────────────────────────────────────────────
_cap_text() {
    tmux capture-pane -t "$SESSION" -p | sed 's/[[:space:]]*$//'
}

_cap_ansi() {
    tmux capture-pane -t "$SESSION" -e -p \
      | sed 's/\x1b\[[0-9;]*[HKJsu]//g; s/\r//g'
}

_save_capture() {
    local snap="$1"
    mkdir -p "$(dirname "$snap")"
    _cap_text > "${snap}.txt"
    _cap_ansi > "${snap}.ansi"
}

# ── Assertion helpers ──────────────────────────────────────────────────────────
_assert_screen_contains() {
    _cap_text | grep -qF "$1" || { echo "  NOT FOUND: $1"; return 1; }
}

_assert_screen_not_contains() {
    if _cap_text | grep -qF "$1"; then
        echo "  SHOULD NOT CONTAIN: $1"; return 1
    fi
}

_assert_line_contains() {
    local n="$1" txt="$2"
    local line; line=$(_cap_text | sed -n "${n}p")
    [[ "$line" == *"$txt"* ]] || { echo "  LINE $n: expected '$txt'"; echo "  GOT: $line"; return 1; }
}

# ── Step dispatch ──────────────────────────────────────────────────────────────
_step() {
    local kw="$1" text="$2" snap="$3"
    case "$text" in

      # ── Setup ────────────────────────────────────────────────────────────────
      "the editor is open")
          _start_editor; sleep 3 ;;

      "the editor is open with file \""*"\"")
          local f; f=$(sed 's/.*file "\(.*\)"/\1/' <<<"$text")
          _start_editor "$f"; sleep 3 ;;

      "the welcome dialog is dismissed")
          _send Escape; sleep 0.5 ;;

      # ── Input ────────────────────────────────────────────────────────────────
      "I type \""*"\"")
          local t; t=$(sed 's/I type "\(.*\)"/\1/' <<<"$text")
          _send "\"$t\""; sleep 0.3 ;;

      "I press "*)
          local k="${text#I press }"
          _send "$k"; sleep 0.5 ;;

      "I wait "*)
          local n; n=$(grep -oE '[0-9]+(\.[0-9]+)?' <<<"$text" | head -1)
          sleep "$n" ;;

      "I wait for the editor to settle")
          sleep 1 ;;

      # ── Screen assertions ─────────────────────────────────────────────────────
      "the screen is captured")
          _save_capture "$snap" ;;

      "the screen shows \""*"\"")
          local s; s=$(sed 's/the screen shows "\(.*\)"/\1/' <<<"$text")
          _assert_screen_contains "$s" ;;

      "the screen does not show \""*"\"")
          local s; s=$(sed 's/the screen does not show "\(.*\)"/\1/' <<<"$text")
          _assert_screen_not_contains "$s" ;;

      "line "*)
          local n s
          n=$(grep -oE '^line [0-9]+' <<<"$text" | grep -oE '[0-9]+')
          s=$(sed 's/.*shows "\(.*\)"/\1/' <<<"$text")
          _assert_line_contains "$n" "$s" ;;

      "the status bar shows \""*"\"")
          local s; s=$(sed 's/the status bar shows "\(.*\)"/\1/' <<<"$text")
          _assert_line_contains 25 "$s" ;;

      "the cursor is at line "*)
          local ln col
          ln=$(grep -oE 'line [0-9]+' <<<"$text" | grep -oE '[0-9]+')
          col=$(grep -oE 'column [0-9]+' <<<"$text" | grep -oE '[0-9]+')
          local pos; pos=$(printf '%05d:%03d' "$ln" "$col")
          _assert_line_contains 25 "$pos" ;;

      *)
          echo "  UNDEFINED: $kw $text"; return 1 ;;
    esac
}

# ── Gherkin parser ─────────────────────────────────────────────────────────────
_snap_path() {
    local feat="$1" scen="$2" step="$3"
    local f; f=$(basename "$feat" .feature)
    local s; s=$(printf '%s' "$scen" | tr ' /' '__' | tr -cd '[:alnum:]_-')
    printf '%s/%s/%s/%03d' "$CAPTURES" "$f" "$s" "$step"
}

_run_feature() {
    local file="$1"
    local label; label=$(basename "$file" .feature | sed 's/^[0-9]*_//' | tr '_' ' ')
    echo -e "\n${B}${C}Feature: $label${N}"

    local scenario="" step_n=0 bg_steps=() in_bg=0

    while IFS= read -r raw || [[ -n "$raw" ]]; do
        local line; line=$(sed 's/^[[:space:]]*//' <<<"$raw")
        [[ -z "$line" || "$line" == \#* || "$line" == Feature:* ]] && continue

        if [[ "$line" == Background:* ]]; then
            in_bg=1; bg_steps=(); continue
        fi

        if [[ "$line" == Scenario:* || "$line" == "Scenario Outline:"* ]]; then
            in_bg=0; scenario="${line#*: }"; step_n=0
            echo -e "  ${B}Scenario: $scenario${N}"
            continue
        fi

        if [[ "$line" == Given\ * || "$line" == When\ * || "$line" == Then\ * || \
              "$line" == And\ *   || "$line" == But\ * ]]; then
            local kw txt
            kw=$(awk '{print $1}' <<<"$line"); txt="${line#"$kw "}"

            if [[ $in_bg -eq 1 ]]; then bg_steps+=("$kw|$txt"); continue; fi

            if [[ $step_n -eq 0 ]]; then
                for bg in "${bg_steps[@]:-}"; do
                    [[ -z "$bg" ]] && continue
                    _step "${bg%%|*}" "${bg#*|}" "$(_snap_path "$file" "$scenario" 0)" 2>/dev/null || true
                done
            fi

            step_n=$((step_n+1))
            local snap; snap=$(_snap_path "$file" "$scenario" "$step_n")
            local out rc
            out=$(_step "$kw" "$txt" "$snap" 2>&1); rc=$?

            if [[ $rc -eq 0 ]]; then
                echo -e "    ${G}✓${N} ${DIM}$kw${N} $txt"; PASS=$((PASS+1))
            else
                echo -e "    ${R}✗${N} ${DIM}$kw${N} $txt"
                [[ -n "$out" ]] && printf '%s\n' "$out" | head -10 | sed 's/^/      /'
                FAIL=$((FAIL+1))
            fi
        fi
    done < "$file"
}

# ── Main ───────────────────────────────────────────────────────────────────────
main() {
    local features=("$@")
    [[ ${#features[@]} -eq 0 ]] && features=("$REPO/tests/features/"*.feature)

    echo -e "${B}redit test runner${N}  target=${Y}$TARGET${N}  ${TERM_W}x${TERM_H}"
    trap _stop_editor EXIT
    for f in "${features[@]}"; do [[ -f "$f" ]] && _run_feature "$f"; done
    echo -e "\n${B}Results:${N} ${G}$PASS passed${N}  ${R}$FAIL failed${N}"
    [[ $FAIL -eq 0 ]]
}

main "$@"
