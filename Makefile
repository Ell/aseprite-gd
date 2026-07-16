# Developer conveniences. `make` lists the targets.

GODOT ?= godot
DEMO := godot

.PHONY: help build build-release test check verify editor run preview corpus fuzz clean

help:
	@echo "aseprite-gd targets:"
	@echo "  make editor        build the extension, open the demo project in the Godot editor"
	@echo "  make run           build, then run an example scene (SCENE=examples/animated_character.tscn)"
	@echo "  make build         debug build of the GDExtension"
	@echo "  make build-release release build"
	@echo "  make test          fast Rust tests (parser, compositor, goldens, oracles)"
	@echo "  make check         everything CI's lint job runs: fmt, clippy, tests"
	@echo "  make verify        headless import + full resource verification of the demo project"
	@echo "  make corpus        regenerate fixtures + goldens (needs Aseprite installed)"
	@echo "  make fuzz          short fuzz pass over parse and render (needs cargo-fuzz)"
	@echo "  make clean         remove build artifacts and the demo project's import cache"

build:
	cargo build -p aseprite-gd

build-release:
	cargo build -p aseprite-gd --release

test:
	cargo test

check:
	cargo fmt --all --check
	cargo clippy --workspace --all-targets -- -D warnings
	cargo test

editor: build
	$(GODOT) --path $(DEMO) -e

SCENE ?= examples/animated_character.tscn
run: build
	$(GODOT) --path $(DEMO) res://$(SCENE)

verify: build
	@# cold first scans can exit nonzero after importing (godot#111645);
	@# the warm run and the verification script are the real gates
	-$(GODOT) --headless --path $(DEMO) --import >/dev/null 2>&1
	$(GODOT) --headless --path $(DEMO) --import >/dev/null 2>&1
	$(GODOT) --headless --path $(DEMO) --script verify_import.gd

corpus:
	./tools/corpus/generate.sh

fuzz:
	cd crates/ase-core && cargo fuzz run parse -- -max_total_time=60
	cd crates/ase-core && cargo fuzz run render -- -max_total_time=60

clean:
	cargo clean
	rm -rf $(DEMO)/.godot
