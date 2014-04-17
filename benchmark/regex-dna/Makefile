bench-rust: run-rust big.fasta
	time ./run-rust < big.fasta

bench-rust-perf: run-rust big.fasta
	time perf record -g -s ./run-rust < big.fasta

bench-golang: run-golang big.fasta
	time ./run-golang < big.fasta

bench-python: regex-dna.py big.fasta
	time python3 ./regex-dna.py < big.fasta

bench-c: run-c big.fasta
	time ./run-c < big.fasta

big.fasta: generator
	./generator 5000000 > big.fasta

generator: shootout-fasta.rs
	rustc --opt-level=3 shootout-fasta.rs -o generator

run-rust: regex-dna.rs
	(cd ../.. && make)
	rustc --opt-level=3 -Z lto -L ../../build regex-dna.rs -o run-rust

run-golang: regex-dna.go
	go build -o run-golang regex-dna.go

run-c: regex-dna.c
	gcc -pipe -Wall -O3 -fomit-frame-pointer -march=native -pthread `pkg-config --cflags --libs glib-2.0` regex-dna.c -o run-c -ltcl -lglib-2.0

check: check.fasta check.output run-rust run-golang run-c
	bash -c 'diff check.output <(./run-golang < check.fasta)'
	bash -c 'diff check.output <(./run-rust < check.fasta)'
	bash -c 'diff check.output <(python3 ./regex-dna.py < check.fasta)'
	bash -c 'diff check.output <(./run-c < check.fasta)'

check.fasta:
	curl 'http://benchmarksgame.alioth.debian.org/download/regexdna-input.txt' > check.fasta

check.output:
	curl 'http://benchmarksgame.alioth.debian.org/download/regexdna-output.txt' > check.output

clean:
	rm -rf big.fasta check.fasta check.output run-golang run-rust run-c generator
	rm -f perf.data*

