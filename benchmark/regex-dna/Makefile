GOMAXPROCS ?= 4

bench-rust: run-rust big.fasta
	time ./run-rust < big.fasta

bench-golang: run-golang big.fasta
	time GOMAXPROCS=$(GOMAXPROCS) ./run-golang < big.fasta

big.fasta: generator
	./generator 5000000 > big.fasta

generator: shootout-fasta.rs
	rustc --opt-level=3 shootout-fasta.rs -o generator

run-rust: regex-dna.rs
	(cd ../.. && make)
	rustc --opt-level=3 -Z lto -L ../../build regex-dna.rs -o run-rust

run-golang: regex-dna.go
	go build -o run-golang regex-dna.go

check: check.fasta check.output run-rust run-golang
	bash -c 'diff check.output <(./run-golang < check.fasta)'
	bash -c 'diff check.output <(./run-rust < check.fasta)'

check.fasta:
	curl 'http://benchmarksgame.alioth.debian.org/download/regexdna-input.txt' > check.fasta

check.output:
	curl 'http://benchmarksgame.alioth.debian.org/download/regexdna-output.txt' > check.output

clean:
	rm -rf big.fasta check.fasta check.output run-golang run-rust generator

