#!/usr/bin/env python3
import re
import sys


SGR_EQUIV = {
    "38;5;0": "30",
    "38;5;4": "34",
    "38;5;7": "37",
    "38;5;8": "90",
    "38;5;15": "97",
    "48;5;0": "40",
    "48;5;4": "44",
    "48;5;6": "46",
    "48;5;7": "47",
    "48;5;15": "107",
}


def parse_ranges(specs):
    ranges = []
    for spec in specs:
        m = re.fullmatch(r"rows=(\d+)-(\d+),cols=(\d+)-(\d+)", spec)
        if not m:
            raise SystemExit(f"invalid ignore spec: {spec}")
        r1, r2, c1, c2 = map(int, m.groups())
        ranges.append((r1, r2, c1, c2))
    return ranges


def ignored(ranges, row, col):
    return any(r1 <= row <= r2 and c1 <= col <= c2 for r1, r2, c1, c2 in ranges)


def normalize_sgr(params):
    if not params:
        params = "0"
    return SGR_EQUIV.get(params, params)


def normalize_text(text, ranges):
    out = []
    for row, line in enumerate(text.splitlines(), 1):
        chars = []
        for col, ch in enumerate(line, 1):
            chars.append(" " if ignored(ranges, row, col) else ch)
        out.append("".join(chars).rstrip())
    return "\n".join(out) + "\n"


def normalize_ansi(text, ranges):
    out = []
    row = 1
    col = 1
    i = 0
    style = ""
    emitted_style = None
    line = []
    while i < len(text):
        if text[i] == "\x1b":
            m = re.match(r"\x1b\[([0-9;]*)m", text[i:])
            if m:
                style = normalize_sgr(m.group(1))
                i += len(m.group(0))
                continue
        ch = text[i]
        i += 1
        if ch == "\r":
            continue
        if ch == "\n":
            out.append("".join(line).rstrip())
            line = []
            row += 1
            col = 1
            emitted_style = None
            continue
        if ignored(ranges, row, col):
            ch = " "
            cell_style = ""
        else:
            cell_style = style
        if cell_style != emitted_style:
            if cell_style:
                line.append(f"<SGR:{cell_style}>")
            emitted_style = cell_style
        line.append(ch)
        col += 1
    if line:
        out.append("".join(line).rstrip())
    return "\n".join(out) + "\n"


def main():
    if len(sys.argv) < 3:
        raise SystemExit("usage: normalize_capture.py txt|ansi FILE [rows=A-B,cols=C-D ...]")
    kind = sys.argv[1]
    path = sys.argv[2]
    ranges = parse_ranges(sys.argv[3:])
    text = open(path, "r", encoding="utf-8", errors="replace").read()
    if kind == "txt":
        sys.stdout.write(normalize_text(text, ranges))
    elif kind == "ansi":
        sys.stdout.write(normalize_ansi(text, ranges))
    else:
        raise SystemExit(f"unknown kind: {kind}")


if __name__ == "__main__":
    main()
