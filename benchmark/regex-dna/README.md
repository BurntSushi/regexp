This compares RE2/Rust with RE2/Go on the
[regex-dna](http://benchmarksgame.alioth.debian.org/u32/performance.php?test=regexdna)
benchmark.

To run, first make sure both benchmarks are correct:

```
[andrew@Liger regex-dna] make check
bash -c 'diff check.output <(./run-golang < check.fasta)'
bash -c 'diff check.output <(./run-rust < check.fasta)'
```

If there's something wrong, an error will be reported along with a non-empty
diff.

Then run the Rust benchmark:

```
[andrew@Liger regex-dna] make bench-rust
...
real    0m48.481s
user    0m42.177s
sys     0m6.303s
```

And now the Go benchmark:

```
[andrew@Liger regex-dna] make bench-golang
time GOMAXPROCS=4 ./run-golang < big.fasta
...
real    0m22.634s
user    1m10.153s
sys     0m0.213s
```

Note that the Go benchmark is multithreaded. The right solution is to make the
Rust benchmark multithreaded too, but for now, just force Go to use only one
thread:

```
[andrew@Liger regex-dna] make GOMAXPROCS=1 bench-golang
time GOMAXPROCS=1 ./run-golang < big.fasta
...
real    0m42.102s
user    0m41.370s
sys     0m0.730s
```

