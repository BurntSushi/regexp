This compares RE2/Rust with RE2/Go on the
[regex-dna](http://benchmarksgame.alioth.debian.org/u32/performance.php?test=regexdna)
benchmark. The Python and C benchmarks are also provided for additional 
context.

To run, first make sure all benchmarks are correct:

```
[andrew@Liger regex-dna] make check
bash -c 'diff check.output <(./run-golang < check.fasta)'
bash -c 'diff check.output <(./run-rust < check.fasta)'
bash -c 'diff check.output <(python3 ./regex-dna.py < check.fasta)'
bash -c 'diff check.output <(./run-c < check.fasta)'
```

If there's something wrong, an error will be reported along with a non-empty
diff.

Then run the Rust benchmark:

```
[andrew@Liger regex-dna] make bench-rust
...
real    0m5.235s
user    0m28.940s
sys     0m0.623s
```

And the Go benchmark:

```
[andrew@Liger regex-dna] make bench-golang
time ./run-golang < big.fasta
...
real    0m18.654s
user    1m44.733s
sys     0m0.420s
```

And the Python benchmark:

```
[andrew@Liger regex-dna] make bench-python
time python3 ./regex-dna.py < big.fasta
...
real    0m4.174s
user    0m13.757s
sys     0m0.407s
```

And the C (Tcl) benchmark:

```
[andrew@Liger regex-dna] make bench-c
time ./run-c < big.fasta
real    0m0.970s
user    0m3.793s
sys     0m0.380s
```

Note that all benchmarks are multithreaded and were run on an Intel i7 3930K
(12 threads).

