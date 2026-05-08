#!/usr/bin/env bash
# tests/compare.sh — run features against both editors then diff captures
#
# Usage:
#   bash tests/compare.sh [features...]
#
# For each "the screen is captured" step, both editors produce:
#   tests/captures/original/<feature>/<scenario>/<NNN>.txt|.ansi
#   tests/captures/clone/<feature>/<scenario>/<NNN>.txt|.ansi
#
# This script then diffs each pair and reports mismatches.

set -euo pipefail
REPO=$(cd "$(dirname "$0")/.." && pwd)
CAPTURES="$REPO/tests/captures"

G='\e[0;32m'; R='\e[0;31m'; Y='\e[0;33m'; C='\e[0;36m'
B='\e[1m'; N='\e[0m'

features=("$@")
[[ ${#features[@]} -eq 0 ]] && features=("$REPO/tests/features/"*.feature)

echo -e "${B}redit comparison suite${N}"
echo -e "  Running features against ${Y}original${N} …"
EDITOR_TARGET=original bash "$REPO/tests/run.sh" "${features[@]}"

echo ""
echo -e "  Running features against ${Y}clone${N} …"
EDITOR_TARGET=clone bash "$REPO/tests/run.sh" "${features[@]}"

echo ""
echo -e "${B}Diffing captures …${N}"

DIFFS=0; MATCHES=0

while IFS= read -r orig_file; do
    rel="${orig_file#"$CAPTURES/original/"}"
    clone_file="$CAPTURES/clone/$rel"

    if [[ ! -f "$clone_file" ]]; then
        echo -e "  ${R}MISSING${N}  $rel  (no clone capture)"
        DIFFS=$((DIFFS+1))
        continue
    fi

    if diff -q "$orig_file" "$clone_file" >/dev/null 2>&1; then
        MATCHES=$((MATCHES+1))
    else
        echo -e "  ${R}DIFF${N}  $rel"
        diff "$orig_file" "$clone_file" \
            --label original --label clone -u | head -20 | sed 's/^/    /'
        DIFFS=$((DIFFS+1))
    fi
done < <(find "$CAPTURES/original" -type f | sort)

echo ""
echo -e "${B}Comparison:${N}  ${G}$MATCHES identical${N}  ${R}$DIFFS different${N}"
[[ $DIFFS -eq 0 ]]
