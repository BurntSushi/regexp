RUST_CFG=
RUSTC ?= rustc
RUSTDOC ?= rustdoc
RUST_PATH ?= $(HOME)/.rust/lib/x86_64-unknown-linux-gnu
RUSTFLAGS ?= --opt-level=3
RUSTTESTFLAGS ?= -L $(RUST_PATH)
SRC_FILES = src/lib.rs src/parse.rs src/compile.rs src/vm.rs \
						src/unicode.rs src/regexp.rs \
						src/test/mod.rs src/test/test.rs src/test/quick.rs

compile:
	$(RUSTC) $(RUSTFLAGS) ./src/lib.rs --out-dir=./build

install:
	cargo-lite install --debug

ctags:
	ctags --recurse --options=ctags.rust --languages=Rust

docs: doc/regexp/index.html

doc/regexp/index.html: $(SRCFILES)
	rm -rf doc
	$(RUSTDOC) -L $(RUST_PATH) --test ./src/lib.rs
	$(RUSTDOC) -L $(RUST_PATH) ./src/lib.rs
	# WTF is rustdoc doing?
	chmod 755 doc
	in-dir doc fix-perms
	rscp ./doc/* gopher:~/www/burntsushi.net/rustdoc/

test: build/tests
	RUST_TEST_TASKS=1 RUST_LOG=regexp ./build/tests

build/tests: $(SRC_FILES)
	rustc $(RUSTTESTFLAGS) --test $(RUST_CFG) src/lib.rs -o ./build/tests

largetest: build/largetests
	RUST_TEST_TASKS=1 RUST_LOG=regexp ./build/largetests

build/largetests: $(SRC_FILES)
	rustc $(RUSTTESTFLAGS) --test --cfg large src/lib.rs -o ./build/largetests

quicktest: build/quicktests
	RUST_TEST_TASKS=1 RUST_LOG=quickcheck,regexp ./build/quicktests

build/quicktests: $(SRC_FILES)
	rustc $(RUSTTESTFLAGS) --test --cfg quickcheck src/lib.rs -o ./build/quicktests

bench: build/bench
	RUST_TEST_TASKS=1 RUST_LOG=regexp ./build/bench --bench

bench-runner: $(SRC_FILES)
	rustc $(RUSTFLAGS) --test $(RUST_CFG) src/lib.rs -o ./build/bench

clean:
	rm -rf ./build/*

push:
	git push origin master
	git push github master

