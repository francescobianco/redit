
# ── Build & run ───────────────────────────────────────────────────────────────

build:
	cargo build

run:
	cargo run

open-v1:
	@old="$$(stty size)"; trap 'set -- $$old; stty rows "$$1" cols "$$2"' EXIT; \
	  stty rows 25 cols 80; dosemu -t -K "$$PWD/dos/EDIT/V1" -E EDIT.COM

open-v2:
	@old="$$(stty size)"; trap 'set -- $$old; stty rows "$$1" cols "$$2"' EXIT; \
	  stty rows 25 cols 80; dosemu -t -K "$$PWD/dos/EDIT/V2" -E EDIT.COM

# ── tmux session management ───────────────────────────────────────────────────

sessions:
	@tmux kill-session -t redit_dos   2>/dev/null || true
	@tmux kill-session -t redit_clone 2>/dev/null || true
	@tmux new-session -d -s redit_dos   -x 80 -y 25 \
	  "dosemu -t -K '$$PWD/dos/EDIT/V1' -E EDIT.COM 2>/dev/null"
	@tmux new-session -d -s redit_clone -x 80 -y 25 \
	  "cargo run -q 2>/dev/null"
	@echo "Sessions started: redit_dos  redit_clone"
	@echo "Attach: tmux attach -t redit_dos"

kill-sessions:
	@tmux kill-session -t redit_dos   2>/dev/null || true
	@tmux kill-session -t redit_clone 2>/dev/null || true
	@echo "Sessions stopped."

# Capture both screens as plain text
capture:
	@echo "=== original ===" && tmux capture-pane -t redit_dos   -p
	@echo "=== clone ==="    && tmux capture-pane -t redit_clone -p

# Capture both screens with ANSI color codes
capture-ansi:
	@echo "=== original (ansi) ===" && tmux capture-pane -t redit_dos   -e -p
	@echo "=== clone (ansi) ==="    && tmux capture-pane -t redit_clone -e -p

# Diff plain-text screens
diff-screens:
	@diff \
	  <(tmux capture-pane -t redit_dos   -p | sed 's/[[:space:]]*$$//') \
	  <(tmux capture-pane -t redit_clone -p | sed 's/[[:space:]]*$$//') \
	  --label original --label clone -u || true

# ── Gherkin test suite ────────────────────────────────────────────────────────

# Run all features against the clone, assert behavioral steps
test:
	@EDITOR_TARGET=clone bash tests/run.sh tests/features/*.feature

# Run a single feature  (usage: make test-feature FEATURE=04_file_menu)
test-feature:
	@EDITOR_TARGET=clone bash tests/run.sh tests/features/$(FEATURE).feature

# Run all features against the original MS-DOS EDIT
test-original:
	@EDITOR_TARGET=original bash tests/run.sh tests/features/*.feature

# Run both and diff captures: the full conformance check
compare:
	@bash tests/compare.sh tests/features/*.feature

# Compare a single feature
compare-feature:
	@bash tests/compare.sh tests/features/$(FEATURE).feature

# ── Misc ─────────────────────────────────────────────────────────────────────

install:
	cargo install --path .

push:
	@git add .
	@git commit -am update || true
	@git push
