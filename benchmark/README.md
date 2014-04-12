Rust
----
```
rustc --opt-level=3 -Z lto -g --test --cfg bench src/lib.rs -o ./build/bench
./build/bench --bench

rustc --opt-level=3 -g -Z lto --test --cfg bench src/lib.rs -o ./build/bench
RUST_TEST_TASKS=1 RUST_LOG=regexp ./build/bench --bench

literal                                 609 ns/iter (+/- 1)
not_literal                            2309 ns/iter (+/- 10)
match_class                            3055 ns/iter (+/- 14)
match_class_in_range                   3162 ns/iter (+/- 15)
replace_all                            5195 ns/iter (+/- 37)
anchored_literal_short_non_match       1854 ns/iter (+/- 22)
anchored_literal_long_non_match       11445 ns/iter (+/- 123)
anchored_literal_short_match           1405 ns/iter (+/- 34)
anchored_literal_long_match            1404 ns/iter (+/- 26)
one_pass_short_a                       3573 ns/iter (+/- 12)
one_pass_short_a_not                   3643 ns/iter (+/- 12)
one_pass_short_b                       2656 ns/iter (+/- 14)
one_pass_short_b_not                   2896 ns/iter (+/- 16)
one_pass_long_prefix                   4167 ns/iter (+/- 44)
one_pass_long_prefix_not               4070 ns/iter (+/- 30)
easy0_32                               2023 ns/iter (+/- 12) = 15 MB/s
easy0_1K                               3224 ns/iter (+/- 109) = 317 MB/s
easy0_32K                             41714 ns/iter (+/- 870) = 785 MB/s
easy0_1M                            1278823 ns/iter (+/- 4766) = 818 MB/s
easy1_32                               1655 ns/iter (+/- 184) = 19 MB/s
easy1_1K                               4111 ns/iter (+/- 965) = 249 MB/s
easy1_32K                             79685 ns/iter (+/- 6474) = 411 MB/s
easy1_1M                            2511920 ns/iter (+/- 36025) = 417 MB/s
medium_32                              3171 ns/iter (+/- 64) = 10 MB/s
medium_1K                             40067 ns/iter (+/- 731) = 25 MB/s
medium_32K                          1218953 ns/iter (+/- 5489) = 26 MB/s
medium_1M                          38959879 ns/iter (+/- 68550) = 26 MB/s
hard_32                                4911 ns/iter (+/- 34) = 6 MB/s
hard_1K                               62908 ns/iter (+/- 501) = 16 MB/s
hard_32K                            1900414 ns/iter (+/- 19110) = 17 MB/s
hard_1M                            60233282 ns/iter (+/- 102418) = 16 MB/s
no_exponential                       281812 ns/iter (+/- 1011)
```

Golang
------
Benchmarks are taken from the `regexp` package included in the Go distribution.

```
cd go/src/pkg/regexp
go test -run ' ' -bench .

Literal                      10000000      229 ns/op
NotLiteral                   500000       3354 ns/op
MatchClass                   500000       5092 ns/op
MatchClass_InRange           500000       4200 ns/op
ReplaceAll                   500000       3548 ns/op
AnchoredLiteralShortNonMatch 20000000      145 ns/op
AnchoredLiteralLongNonMatch  20000000      142 ns/op
AnchoredShortMatch           5000000       381 ns/op
AnchoredLongMatch            5000000       383 ns/op
OnePassShortA                1000000      1045 ns/op
NotOnePassShortA             1000000      2478 ns/op
OnePassShortB                2000000       766 ns/op
NotOnePassShortB             1000000      2216 ns/op
OnePassLongPrefix            10000000      156 ns/op
OnePassLongNotPrefix         5000000       614 ns/op
MatchEasy0_32                20000000      114 ns/op  279.35 MB/s
MatchEasy0_1K                5000000       653 ns/op 1566.63 MB/s
MatchEasy0_32K               200000      12624 ns/op 2595.57 MB/s
MatchEasy0_1M                5000       458608 ns/op 2286.43 MB/s
MatchEasy1_32                20000000     96.7 ns/op  330.99 MB/s
MatchEasy1_1K                1000000      2647 ns/op  386.74 MB/s
MatchEasy1_32K               50000       57848 ns/op  566.45 MB/s
MatchEasy1_1M                1000      1991274 ns/op  526.59 MB/s
MatchMedium_32               1000000      1746 ns/op   18.33 MB/s
MatchMedium_1K               50000       58501 ns/op   17.50 MB/s
MatchMedium_32K              1000      1914850 ns/op   17.11 MB/s
MatchMedium_1M               50       61487227 ns/op   17.05 MB/s
MatchHard_32                 500000       2918 ns/op   10.97 MB/s
MatchHard_1K                 20000       92338 ns/op   11.09 MB/s
MatchHard_32K                1000      2979930 ns/op   11.00 MB/s
MatchHard_1M                 20       95889705 ns/op   10.94 MB/s
```


Very rough benchmark analysis
-----------------------------
All benchmarks were taken from RE2/Go and hopefully implemented correctly.
Both RE2/Rust and RE2/Go are benchmarked with an implicit `.*?` prefixing all 
regular expressions. (i.e., They are unachored unless there is an explicit 
'^'.)

RE2/Rust gets absolutely clobbered by RE2/Go in the Easy{0,1} benchmarks. 
Interestingly, Rust does the same or better on the Medium/Hard benchmarks. My 
suspicion is that RE2/Go is performing some optimizations on the easy 
benchmarks to make the throughput very high. This gives me hope.

For example, the EASY{0,1} benchmarks are subject to optimization. RE2/Rust
does do some optimization with literal prefix strings (explaining the higher
throughput when compared to the MEDIUM/HARD benchmarks).

It's promising that RE2/Rust is beating RE2/Go on the MEDIUM/HARD benchmarks, 
which I think suggests that the core VM implementation is probably decent.

Also note that RE2/Rust is performing much worse on the small Medium/Hard 
benchmarks (searching 32 bytes of text). My suspicion is that there are some 
big constant factors lurking somewhere that need to be fixed in RE2/Rust.
This may also explain some of the performance difference in other benchmarks 
(NOT easy/medium/hard) since they mostly work with shortish search strings.
(Although this is not true for all, since some specifically target the presence 
of optimizations in RE2/Go.)

