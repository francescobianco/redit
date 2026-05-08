#!/usr/bin/env bash
# tests/compare.sh — run features against both editors then diff captures
#
# Compares clone-v1 against original-v1 (the faithful V1 comparison).
# clone-v2 is redit-only and has no original counterpart.
#
# Captures:
#   tests/captures/original-v1/<feature>/<scenario>/<NNN>.txt
#   tests/captures/clone-v1/<feature>/<scenario>/<NNN>.txt
#   (same structure for .ansi files)
#
# Diff strategy:
#   .txt  → text content comparison  (layout, characters)
#   .ansi → color comparison         (SGR codes stripped of cursor/erase sequences)
#
# Exit code: 0 = all captures identical, 1 = any diff found.

set -euo pipefail
REPO=$(cd "$(dirname "$0")/.." && pwd)
CAPTURES="$REPO/tests/captures"

G='\e[0;32m'; R='\e[0;31m'; Y='\e[0;33m'; C='\e[0;36m'
B='\e[1m'; DIM='\e[2m'; N='\e[0m'

features=("$@")
[[ ${#features[@]} -eq 0 ]] && features=("$REPO/tests/features/"*.feature)

echo -e "${B}redit comparison suite${N}  (original-v1 ↔ clone-v1)"
echo -e "  Running features against ${Y}original-v1${N} …"
EDITOR_TARGET=original EDITOR_VERSION=v1 bash "$REPO/tests/run.sh" "${features[@]}" || true

echo ""
echo -e "  Running features against ${Y}clone-v1${N} …"
EDITOR_TARGET=clone EDITOR_VERSION=v1 bash "$REPO/tests/run.sh" "${features[@]}" || true

echo ""
echo -e "${B}Diffing captures …${N}"

TEXT_DIFFS=0; COLOR_DIFFS=0; MATCHES=0; MISSING=0

_ignore_specs_for_rel() {
    local rel="$1" feature="${rel%%/*}" feature_file=""
    for feat in "${features[@]}"; do
        if [[ "$(basename "$feat" .feature)" == "$feature" ]]; then
            feature_file="$feat"
            break
        fi
    done
    [[ -z "$feature_file" ]] && return 0
    sed -n 's/^[[:space:]]*# compare-ignore:[[:space:]]*rows=\([0-9]\+-[0-9]\+\)[[:space:]]*cols=\([0-9]\+-[0-9]\+\)[[:space:]]*$/rows=\1,cols=\2/p' "$feature_file"
}

_normalize_capture() {
    local kind="$1" file="$2"; shift 2
    python3 "$REPO/tests/normalize_capture.py" "$kind" "$file" "$@"
}

if [[ $# -eq 0 ]]; then
    diff_roots=("$CAPTURES/original-v1")
else
    diff_roots=()
    for feat in "${features[@]}"; do
        name=$(basename "$feat" .feature)
        diff_roots+=("$CAPTURES/original-v1/$name")
    done
fi

while IFS= read -r orig_file; do
    rel="${orig_file#"$CAPTURES/original-v1/"}"
    clone_file="$CAPTURES/clone-v1/$rel"

    if [[ ! -f "$clone_file" ]]; then
        echo -e "  ${R}MISSING${N}  $rel"
        MISSING=$((MISSING+1))
        continue
    fi

    ext="${orig_file##*.}"
    mapfile -t ignore_specs < <(_ignore_specs_for_rel "$rel")

    norm_orig=$(mktemp)
    norm_clone=$(mktemp)
    norm_diff=$(mktemp)
    if [[ "$ext" == "ansi" ]]; then
        _normalize_capture ansi "$orig_file" "${ignore_specs[@]}" > "$norm_orig"
        _normalize_capture ansi "$clone_file" "${ignore_specs[@]}" > "$norm_clone"
    else
        _normalize_capture txt "$orig_file" "${ignore_specs[@]}" > "$norm_orig"
        _normalize_capture txt "$clone_file" "${ignore_specs[@]}" > "$norm_clone"
    fi

    if diff -q "$norm_orig" "$norm_clone" >/dev/null 2>&1; then
        rm -f "$norm_orig" "$norm_clone" "$norm_diff"
        MATCHES=$((MATCHES+1))
    elif [[ "$ext" == "txt" ]]; then
        echo -e "  ${R}TEXT DIFF${N}  ${DIM}${rel}${N}"
        diff "$norm_orig" "$norm_clone" \
            --label original-v1 --label clone-v1 -u > "$norm_diff" || true
        head -30 "$norm_diff" | sed 's/^/    /'
        rm -f "$norm_orig" "$norm_clone" "$norm_diff"
        TEXT_DIFFS=$((TEXT_DIFFS+1))
    elif [[ "$ext" == "ansi" ]]; then
        echo -e "  ${Y}COLOR DIFF${N}  ${DIM}${rel%.ansi}.txt${N}"
        diff "$norm_orig" "$norm_clone" \
            --label original-v1 --label clone-v1 -u > "$norm_diff" || true
        head -30 "$norm_diff" | sed 's/^/    /'
        rm -f "$norm_orig" "$norm_clone" "$norm_diff"
        COLOR_DIFFS=$((COLOR_DIFFS+1))
    else
        rm -f "$norm_orig" "$norm_clone" "$norm_diff"
    fi
done < <(find "${diff_roots[@]}" -type f 2>/dev/null | sort)

echo ""
TOTAL_DIFFS=$((TEXT_DIFFS + COLOR_DIFFS + MISSING))
echo -e "${B}Comparison:${N}  ${G}$MATCHES identical${N}  ${R}$TEXT_DIFFS text-diff${N}  ${Y}$COLOR_DIFFS color-diff${N}  ${R}$MISSING missing${N}"
[[ $TOTAL_DIFFS -eq 0 ]]
