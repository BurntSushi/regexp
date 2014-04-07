RUST_CFG=
RUST_PATH ?= $(HOME)/.rust/lib/x86_64-unknown-linux-gnu
SRC_FILES = src/lib.rs src/parse.rs src/compile.rs src/vm.rs \
						src/unicode.rs src/regexp.rs src/test.rs

compile:
	rustc --opt-level=3 ./src/lib.rs

install:
	cargo-lite install --debug

ctags:
	ctags --recurse --options=ctags.rust --languages=Rust

docs:
	rm -rf doc
	rustdoc -L $(RUST_PATH) --test src/lib.rs
	rustdoc -L $(RUST_PATH) src/lib.rs
	# WTF is rustdoc doing?
	chmod 755 doc
	in-dir doc fix-perms
	rscp ./doc/* gopher:~/www/burntsushi.net/rustdoc/

test: test-runner
	RUST_TEST_TASKS=1 RUST_LOG=quickcheck,regexp ./test-runner

test-runner: $(SRC_FILES)
	rustc -L $(RUST_PATH) --test src/lib.rs -o test-runner

test-examples:
	(cd ./examples && ./test)

bench: bench-runner
	RUST_TEST_TASKS=1 RUST_LOG=quickcheck,regexp ./bench-runner --bench

bench-runner: $(SRC_FILES)
	rustc --opt-level=3 --test $(RUST_CFG) src/lib.rs -o bench-runner

test-clean:
	rm -rf ./test-runner ./bench-runner

clean: test-clean
	rm -f *.rlib

push:
	git push origin master
	git push github master

