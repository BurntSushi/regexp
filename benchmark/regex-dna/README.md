This compares RE2/Rust with RE2/Go on the
[regex-dna](http://benchmarksgame.alioth.debian.org/u32/performance.php?test=regexdna)
benchmark. The Python benchmark is also provided for additional context.

To run, first make sure all benchmarks are correct:

```
[andrew@Liger regex-dna] make check
bash -c 'diff check.output <(./run-golang < check.fasta)'
bash -c 'diff check.output <(./run-rust < check.fasta)'
bash -c 'diff check.output <(python3 ./regex-dna.py < check.fasta)'
```

If there's something wrong, an error will be reported along with a non-empty
diff.

Then run the Rust benchmark:

```
[andrew@Liger regex-dna] make bench-rust
...
real    0m11.449s
user    0m53.620s
sys     0m0.543s
```

(Note that I'm getting a lot of variation on my system for the Rust benchmark
and I'm not sure why. I've seen wall clock times as low as 9 seconds and as
high as 15 seconds. My machine was otherwise idle. The string replacement
seems to be the bottleneck.)

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

Note that all benchmarks are multithreaded and were run on an Intel i7 3930K
(12 threads).

