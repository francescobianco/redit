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

    if diff -q "$orig_file" "$clone_file" >/dev/null 2>&1; then
        MATCHES=$((MATCHES+1))
    elif [[ "$ext" == "txt" ]]; then
        echo -e "  ${R}TEXT DIFF${N}  ${DIM}${rel}${N}"
        diff "$orig_file" "$clone_file" \
            --label original-v1 --label clone-v1 -u | head -30 | sed 's/^/    /'
        TEXT_DIFFS=$((TEXT_DIFFS+1))
    elif [[ "$ext" == "ansi" ]]; then
        orig_colors=$(grep -oP '\x1b\[[0-9;]*m' "$orig_file" 2>/dev/null | sort | uniq -c | sort -rn | head -10 || true)
        clone_colors=$(grep -oP '\x1b\[[0-9;]*m' "$clone_file" 2>/dev/null | sort | uniq -c | sort -rn | head -10 || true)
        if [[ "$orig_colors" != "$clone_colors" ]]; then
            echo -e "  ${Y}COLOR DIFF${N}  ${DIM}${rel%.ansi}.txt${N}"
            echo "    original-v1 top SGR codes:"
            (grep -oP '\x1b\[[0-9;]*m' "$orig_file" 2>/dev/null || true) \
                | sort | uniq -c | sort -rn | head -5 \
                | sed "s/^ */    /" | sed 's/\x1b\[/ESC[/g'
            echo "    clone-v1 top SGR codes:"
            (grep -oP '\x1b\[[0-9;]*m' "$clone_file" 2>/dev/null || true) \
                | sort | uniq -c | sort -rn | head -5 \
                | sed "s/^ */    /" | sed 's/\x1b\[/ESC[/g'
            COLOR_DIFFS=$((COLOR_DIFFS+1))
        else
            MATCHES=$((MATCHES+1))
        fi
    fi
done < <(find "${diff_roots[@]}" -type f 2>/dev/null | sort)

echo ""
TOTAL_DIFFS=$((TEXT_DIFFS + COLOR_DIFFS + MISSING))
echo -e "${B}Comparison:${N}  ${G}$MATCHES identical${N}  ${R}$TEXT_DIFFS text-diff${N}  ${Y}$COLOR_DIFFS color-diff${N}  ${R}$MISSING missing${N}"
[[ $TOTAL_DIFFS -eq 0 ]]
