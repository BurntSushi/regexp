RUSTC ?= rustc
RUSTDOC ?= rustdoc
BUILD_DIR ?= ./build
RUST_PATH ?= $(BUILD_DIR)
RUSTFLAGS ?= --opt-level=3
RUSTTESTFLAGS ?= 
REGEXP_LIB ?= $(BUILD_DIR)/.libregex.timestamp
REGEXP_LIB_FILES = src/compile.rs src/lib.rs src/parse.rs src/re.rs \
									 src/unicode.rs src/vm.rs
REGEXP_MACRO_LIB ?= $(BUILD_DIR)/.libregex_macros.timestamp
REGEXP_MACRO_LIB_FILES = src/macro.rs
REGEXP_TEST_FILES = src/test/bench.rs src/test/matches.rs \
									  src/test/mod.rs src/test/tests.rs
MOZILLA_RUST ?= $(HOME)/clones/rust
REGEXP_DYN_FLAGS =

ifdef REGEXP_DYNAMIC
	REGEXP_DYN_FLAGS = --cfg dynamic
endif

all: $(REGEXP_LIB) $(REGEXP_MACRO_LIB)

install:
	cargo-lite install

$(REGEXP_LIB): $(REGEXP_LIB_FILES)
	@mkdir -p $(BUILD_DIR)
	$(RUSTC) $(RUSTFLAGS) ./src/lib.rs --out-dir=$(BUILD_DIR)
	@touch $(REGEXP_LIB)

$(REGEXP_MACRO_LIB): $(REGEXP_LIB) $(REGEXP_MACRO_LIB_FILES)
	@mkdir -p $(BUILD_DIR)
	$(RUSTC) -L $(BUILD_DIR) $(RUSTFLAGS) ./src/macro.rs --out-dir=$(BUILD_DIR)
	@touch $(REGEXP_MACRO_LIB)

match-tests:
	./regex-match-tests.py ./src/testdata/*.dat > ./src/test/matches.rs

unicode-tables:
	./regex-unicode-tables.py > ./src/unicode.rs

docs: $(REGEXP_LIB_FILES) $(REGEXP_MACRO_LIB_FILES)
	rm -rf doc
	$(RUSTDOC) -L $(RUST_PATH) --test ./src/lib.rs
	$(RUSTDOC) -L $(RUST_PATH) ./src/lib.rs
	$(RUSTDOC) -L $(RUST_PATH) ./src/macro.rs
	# WTF is rustdoc doing?
	chmod 755 doc
	in-dir doc fix-perms
	rscp ./doc/* gopher:~/www/burntsushi.net/rustdoc/

test: build/tests
	RUST_TEST_TASKS=1 RUST_LOG=regex ./build/tests

build/tests: $(REGEXP_LIB) $(REGEXP_MACRO_LIB) $(REGEXP_TEST_FILES)
	$(RUSTC) $(RUSTTESTFLAGS) -L $(RUST_PATH) --test $(REGEXP_DYN_FLAGS) src/lib.rs -o ./build/tests

bench: build/bench
	RUST_TEST_TASKS=1 RUST_LOG=regex ./build/bench --bench

bench-perf: build/bench
	RUST_TEST_TASKS=1 RUST_LOG=regex perf record -g --call-graph dwarf -s ./build/bench --bench

build/bench: $(REGEXP_LIB) $(REGEXP_MACRO_LIB) $(REGEXP_TEST_FILES)
	$(RUSTC) $(RUSTFLAGS) -g -Z lto -L $(RUST_PATH) --test --cfg bench $(REGEXP_DYN_FLAGS) src/lib.rs -o ./build/bench

scratch: build/scratch
	RUST_TEST_TASKS=1 RUST_LOG=regex ./build/scratch

build/scratch: $(REGEXP_MACRO_LIB) scratch.rs
	$(RUSTC) -L $(BUILD_DIR) $(RUSTTESTFLAGS) scratch.rs -o ./build/scratch

ctags:
	ctags --recurse --options=ctags.rust --languages=Rust

clean:
	rm -f $(BUILD_DIR)/.*.timestamp $(BUILD_DIR)/*

push:
	git push origin master
	git push github master

mozilla:
	mkdir -p $(MOZILLA_RUST)/src/libregex
	mkdir -p $(MOZILLA_RUST)/src/libregex_macros
	rm -rf $(MOZILLA_RUST)/src/libregex/*
	cp -a ./src/* $(MOZILLA_RUST)/src/libregex/
	rm $(MOZILLA_RUST)/src/libregex/macro.rs
	cp ./src/macro.rs $(MOZILLA_RUST)/src/libregex_macros/lib.rs
	cp *.py $(MOZILLA_RUST)/src/etc/
	cp ./benchmark/regex-dna/regex-dna.rs $(MOZILLA_RUST)/src/test/bench/shootout-regex-dna.rs

