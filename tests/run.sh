#!/usr/bin/env bash
# tests/run.sh — redit Gherkin test runner
#
# Usage:
#   bash tests/run.sh                          # run all features (compare mode)
#   bash tests/run.sh tests/features/01_*.feature  # run specific feature
#   SNAPSHOT_MODE=record bash tests/run.sh     # capture golden snapshots from dosemu
#   SNAPSHOT_MODE=compare bash tests/run.sh    # compare clone against golden (default)
#
# Environment variables:
#   SNAPSHOT_MODE   record|compare (default: compare)
#   TERM_W          terminal width  (default: 80)
#   TERM_H          terminal height (default: 25)
#   DOS_VERSION     V1|V2 (default: V1)

set -euo pipefail
REPO=$(cd "$(dirname "$0")/.." && pwd)

DOS_SESSION="redit_dos"
CLONE_SESSION="redit_clone"
SNAPS_DIR="$REPO/tests/snapshots"
MODE="${SNAPSHOT_MODE:-compare}"
TERM_W="${TERM_W:-80}"
TERM_H="${TERM_H:-25}"
DOS_VER="${DOS_VERSION:-V1}"
PASS=0; FAIL=0; SKIP=0
_SCENARIO_FAILED=0

# ── Terminal colors ────────────────────────────────────────────────────────────
G='\e[0;32m'; R='\e[0;31m'; Y='\e[0;33m'; C='\e[0;36m'
B='\e[1m'; DIM='\e[2m'; N='\e[0m'

# ── Session helpers ────────────────────────────────────────────────────────────

_start_dos() {
    local extra="${1:-}"
    tmux kill-session -t "$DOS_SESSION" 2>/dev/null || true
    tmux new-session -d -s "$DOS_SESSION" -x "$TERM_W" -y "$TERM_H" \
        "dosemu -t -K '$REPO/dos/EDIT/$DOS_VER' -E '${extra:-EDIT.COM}' 2>/dev/null"
}

_start_clone() {
    local extra="${1:-}"
    tmux kill-session -t "$CLONE_SESSION" 2>/dev/null || true
    tmux new-session -d -s "$CLONE_SESSION" -x "$TERM_W" -y "$TERM_H" \
        "cd '$REPO'; cargo run -q -- $extra 2>/dev/null"
}

_send() {
    local target="$1"; shift
    # Each token is a separate tmux key argument
    eval "tmux send-keys -t '$target' $*"
}

_send_dos()   { _send "$DOS_SESSION"   "$@"; }
_send_clone() { _send "$CLONE_SESSION" "$@"; }
_send_both()  { _send_dos "$@"; _send_clone "$@"; }

# ── Capture helpers ────────────────────────────────────────────────────────────

# Plain text, trailing whitespace stripped from each line
_cap_text() {
    tmux capture-pane -t "$1" -p | sed 's/[[:space:]]*$//'
}

# Full ANSI escape sequence output
_cap_ansi() {
    tmux capture-pane -t "$1" -e -p
}

# Normalize ANSI: keep SGR color codes, drop positioning/erase codes
_norm_ansi() {
    sed 's/\x1b\[[0-9;]*[HKJsu]//g; s/\r//g'
}

# ── Snapshot helpers ───────────────────────────────────────────────────────────

_snap_path() {
    local feature="$1" scenario="$2" step="$3"
    local feat; feat=$(basename "$feature" .feature)
    local scen; scen=$(printf '%s' "$scenario" | tr ' /' '__' | tr -cd '[:alnum:]_-')
    printf '%s/%s/%s/%03d' "$SNAPS_DIR" "$feat" "$scen" "$step"
}

_save_snap() {
    local path="$1"; shift
    mkdir -p "$(dirname "$path")"
    "$@" > "$path"
}

# ── Assertion helpers ──────────────────────────────────────────────────────────

_assert_text() {
    local snap="$1"
    local actual; actual=$(_cap_text "$CLONE_SESSION")
    if [[ "$MODE" == record ]]; then
        _save_snap "${snap}.txt" _cap_text "$DOS_SESSION"
        return 0
    fi
    if [[ ! -f "${snap}.txt" ]]; then
        echo "  MISSING golden: ${snap}.txt  →  run with SNAPSHOT_MODE=record first"
        return 1
    fi
    diff --strip-trailing-cr -u "${snap}.txt" <(echo "$actual") \
        --label "golden" --label "clone" || return 1
}

_assert_ansi() {
    local snap="$1"
    local actual; actual=$(_cap_ansi "$CLONE_SESSION" | _norm_ansi)
    if [[ "$MODE" == record ]]; then
        _save_snap "${snap}.ansi" bash -c "_cap_ansi '$DOS_SESSION' | _norm_ansi"
        return 0
    fi
    if [[ ! -f "${snap}.ansi" ]]; then
        echo "  MISSING golden: ${snap}.ansi  →  run with SNAPSHOT_MODE=record first"
        return 1
    fi
    diff -u "${snap}.ansi" <(echo "$actual") --label "golden" --label "clone" || return 1
}

_assert_contains() {
    local text="$1"
    _cap_text "$CLONE_SESSION" | grep -qF "$text" || {
        echo "  Screen does not contain: $text"
        return 1
    }
}

_assert_line() {
    local n="$1" text="$2"
    local line; line=$(_cap_text "$CLONE_SESSION" | sed -n "${n}p")
    [[ "$line" == *"$text"* ]] || {
        echo "  Line $n expected to contain: $text"
        echo "  Actual: $line"
        return 1
    }
}

_assert_not_contains() {
    local text="$1"
    if _cap_text "$CLONE_SESSION" | grep -qF "$text"; then
        echo "  Screen should NOT contain: $text"
        return 1
    fi
}

# ── Step dispatch ──────────────────────────────────────────────────────────────
# Patterns are matched top-to-bottom; first match wins.

_dispatch() {
    local kw="$1" text="$2" snap="$3"

    case "$text" in

      # ── Setup steps ──────────────────────────────────────────────────────────
      "both editors are started fresh")
          _start_dos; _start_clone; sleep 3 ;;

      "both editors are started fresh at ${TERM_W}x${TERM_H}")
          _start_dos; _start_clone; sleep 3 ;;

      "both editors are started with file \""*"\"")
          local f; f=$(sed 's/.*file "\(.*\)"/\1/' <<<"$text")
          _start_dos "EDIT.COM $f"
          _start_clone "$f"
          sleep 3 ;;

      "the original editor is started")
          _start_dos; sleep 3 ;;

      "the clone is started")
          _start_clone; sleep 3 ;;

      # ── Key input steps ───────────────────────────────────────────────────────
      "I send \""*"\" to both")
          local k; k=$(sed 's/I send "\(.*\)" to both/\1/' <<<"$text")
          _send_both $k; sleep 0.5 ;;

      "I send \""*"\" to the clone")
          local k; k=$(sed 's/I send "\(.*\)" to the clone/\1/' <<<"$text")
          _send_clone $k; sleep 0.5 ;;

      "I send \""*"\" to the original")
          local k; k=$(sed 's/I send "\(.*\)" to the original/\1/' <<<"$text")
          _send_dos $k; sleep 0.5 ;;

      # ── Timing ────────────────────────────────────────────────────────────────
      "I wait "*)
          local n; n=$(grep -oE '[0-9]+(\.[0-9]+)?' <<<"$text" | head -1)
          sleep "$n" ;;

      "I wait for the editors to settle")
          sleep 1 ;;

      # ── Assertions ────────────────────────────────────────────────────────────
      "the screen text matches")
          _assert_text "$snap" ;;

      "the screen colors match")
          _assert_ansi "$snap" ;;

      "the screen matches")
          _assert_text "$snap" && _assert_ansi "$snap" ;;

      "the clone screen contains \""*"\"")
          local s; s=$(sed 's/.*contains "\(.*\)"/\1/' <<<"$text")
          _assert_contains "$s" ;;

      "the clone screen does not contain \""*"\"")
          local s; s=$(sed 's/.*contain "\(.*\)"/\1/' <<<"$text")
          _assert_not_contains "$s" ;;

      "the clone shows \""*"\" on line "*)
          local s n
          s=$(sed 's/the clone shows "\(.*\)" on line.*/\1/' <<<"$text")
          n=$(grep -oE '[0-9]+$' <<<"$text")
          _assert_line "$n" "$s" ;;

      *)
          echo "  UNDEFINED step: $kw $text"
          return 1 ;;
    esac
}

# ── Gherkin parser ─────────────────────────────────────────────────────────────

_run_feature() {
    local file="$1"
    local feat_label; feat_label=$(basename "$file" .feature | sed 's/^[0-9]*_//' | tr '_' ' ')
    echo -e "\n${B}${C}Feature: $feat_label${N}"

    local scenario="" step_n=0
    local bg_steps=(); local in_bg=0

    while IFS= read -r raw || [[ -n "$raw" ]]; do
        local line; line=$(sed 's/^[[:space:]]*//' <<<"$raw")
        [[ -z "$line" || "$line" == \#* ]] && continue

        if [[ "$line" == Feature:* ]]; then continue; fi

        if [[ "$line" == Background:* ]]; then
            in_bg=1; bg_steps=(); continue
        fi

        if [[ "$line" == Scenario:* || "$line" == "Scenario Outline:"* ]]; then
            in_bg=0
            scenario="${line#*: }"
            step_n=0
            _SCENARIO_FAILED=0
            echo -e "  ${B}Scenario: $scenario${N}"
            continue
        fi

        if [[ "$line" == Given\ * || "$line" == When\ *  || "$line" == Then\ * || \
              "$line" == And\ *   || "$line" == But\ * ]]; then

            local kw txt
            kw=$(awk '{print $1}' <<<"$line")
            txt="${line#"$kw "}"

            if [[ $in_bg -eq 1 ]]; then
                bg_steps+=("$kw|$txt"); continue
            fi

            # Run background steps before first step of scenario
            if [[ $step_n -eq 0 ]]; then
                for bg in "${bg_steps[@]:-}"; do
                    [[ -z "$bg" ]] && continue
                    local bg_kw="${bg%%|*}" bg_txt="${bg#*|}"
                    local bg_snap; bg_snap=$(_snap_path "$file" "$scenario" 0)
                    _dispatch "$bg_kw" "$bg_txt" "$bg_snap" 2>/dev/null || true
                done
            fi

            step_n=$((step_n+1))
            local snap; snap=$(_snap_path "$file" "$scenario" "$step_n")

            local out rc
            out=$(_dispatch "$kw" "$txt" "$snap" 2>&1); rc=$?

            if [[ $rc -eq 0 ]]; then
                echo -e "    ${G}✓${N} ${DIM}$kw${N} $txt"
                PASS=$((PASS+1))
            else
                echo -e "    ${R}✗${N} ${DIM}$kw${N} $txt"
                [[ -n "$out" ]] && printf '%s\n' "$out" | head -15 | sed 's/^/      /'
                FAIL=$((FAIL+1))
                _SCENARIO_FAILED=1
            fi
        fi
    done < "$file"
}

# ── Main ───────────────────────────────────────────────────────────────────────
main() {
    local features=("$@")
    [[ ${#features[@]} -eq 0 ]] && features=("$REPO/tests/features/"*.feature)

    echo -e "${B}redit Gherkin test suite${N}"
    echo -e "  mode=${Y}$MODE${N}  terminal=${TERM_W}x${TERM_H}  dos=${DOS_VER}"

    for f in "${features[@]}"; do
        [[ -f "$f" ]] && _run_feature "$f"
    done

    echo ""
    echo -e "${B}Results:${N}  ${G}$PASS passed${N}  ${R}$FAIL failed${N}"
    [[ $FAIL -eq 0 ]]
}

main "$@"