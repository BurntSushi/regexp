RUSTC ?= rustc
RUSTDOC ?= rustdoc
RUST_PATH ?= ./build/
RUSTFLAGS ?= --opt-level=3
RUSTTESTFLAGS ?= -L $(RUST_PATH)
SRC_FILES = src/lib.rs src/parse.rs src/compile.rs src/vm.rs \
						src/unicode.rs src/regexp.rs src/macro.rs \
						src/test/mod.rs src/test/matches.rs src/test/bench.rs \
						src/test/macro.rs

compile:
	make regexp
	make regexp-re

regexp:
	$(RUSTC) -L $(RUST_PATH) $(RUSTFLAGS) ./src/lib.rs --out-dir=./build

regexp-re:
	$(RUSTC) -L $(RUST_PATH) $(RUSTFLAGS) ./src/macro.rs --out-dir=./build

macros: macros.rs
	$(RUSTC) -L $(RUST_PATH) macros.rs -o macros

install:
	cargo-lite install --debug

ctags:
	ctags --recurse --options=ctags.rust --languages=Rust

match-tests:
	./make-match-tests ./src/testdata/*.dat > ./src/test/matches.rs

unicode-tables:
	./make-unicode-tables > ./src/unicode.rs

docs: $(SRCFILES)
	rm -rf doc
	$(RUSTDOC) -L $(RUST_PATH) --test ./src/lib.rs
	$(RUSTDOC) -L $(RUST_PATH) ./src/lib.rs
	$(RUSTDOC) -L $(RUST_PATH) ./src/macro.rs
	# WTF is rustdoc doing?
	chmod 755 doc
	in-dir doc fix-perms
	rscp ./doc/* gopher:~/www/burntsushi.net/rustdoc/

test: build/tests test-re
	RUST_TEST_TASKS=1 RUST_LOG=regexp ./build/tests

build/tests: $(SRC_FILES)
	rustc $(RUSTTESTFLAGS) --test src/lib.rs -o ./build/tests

test-re: build/tests-re
	RUST_TEST_TASKS=1 RUST_LOG=regexp,regex_re_test ./build/tests-re

build/tests-re: $(SRC_FILES)
	rustc $(RUSTTESTFLAGS) --test src/test/macro.rs -o ./build/tests-re

debug: build/debug
	RUST_TEST_TASKS=1 RUST_LOG=regexp ./build/debug

build/debug: $(SRC_FILES)
	rustc $(RUSTTESTFLAGS) --test --cfg debug src/lib.rs -o ./build/debug

bench: build/bench
	RUST_TEST_TASKS=1 RUST_LOG=regexp ./build/bench --bench

bench-perf: build/bench
	RUST_TEST_TASKS=1 RUST_LOG=regexp perf record -g -s ./build/bench --bench

build/bench: $(SRC_FILES)
	rustc $(RUSTFLAGS) -Z lto -g --test --cfg bench src/lib.rs -o ./build/bench

clean:
	rm -rf ./build/*

push:
	git push origin master
	git push github master

