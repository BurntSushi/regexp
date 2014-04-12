Rust
----
```
rustc --opt-level=3 -Z lto -g --test --cfg bench src/lib.rs -o ./build/bench
./build/bench --bench

literal                                 435 ns/iter (+/- 2)
not_literal                            1967 ns/iter (+/- 10)
match_class                            2545 ns/iter (+/- 17)
match_class_in_range                   2644 ns/iter (+/- 34)
replace_all                            6224 ns/iter (+/- 398)
anchored_literal_short_non_match        991 ns/iter (+/- 4)
anchored_literal_long_non_match        9119 ns/iter (+/- 20)
anchored_literal_short_match            571 ns/iter (+/- 4)
anchored_literal_long_match             565 ns/iter (+/- 2)
one_pass_short_a                       2149 ns/iter (+/- 17)
one_pass_short_a_not                   2644 ns/iter (+/- 27)
one_pass_short_b                       1565 ns/iter (+/- 7)
one_pass_short_b_not                   2157 ns/iter (+/- 10)
one_pass_long_prefix                   1281 ns/iter (+/- 11)
one_pass_long_prefix_not               1234 ns/iter (+/- 6)
easy0_32                                651 ns/iter (+/- 4) = 49 MB/s
easy0_1K                               2123 ns/iter (+/- 115) = 482 MB/s
easy0_32K                             48763 ns/iter (+/- 896) = 671 MB/s
easy0_1M                            1545978 ns/iter (+/- 5075) = 677 MB/s
easy1_32                                609 ns/iter (+/- 154) = 52 MB/s
easy1_1K                               3091 ns/iter (+/- 815) = 331 MB/s
easy1_32K                             83045 ns/iter (+/- 4995) = 394 MB/s
easy1_1M                            2654424 ns/iter (+/- 34276) = 394 MB/s
medium_32                              1648 ns/iter (+/- 63) = 19 MB/s
medium_1K                             33882 ns/iter (+/- 838) = 30 MB/s
medium_32K                          1072079 ns/iter (+/- 5921) = 30 MB/s
medium_1M                          34140609 ns/iter (+/- 51115) = 30 MB/s
hard_32                                2479 ns/iter (+/- 40) = 12 MB/s
hard_1K                               54950 ns/iter (+/- 255) = 18 MB/s
hard_32K                            1738851 ns/iter (+/- 3483) = 18 MB/s
hard_1M                            55405512 ns/iter (+/- 40061) = 18 MB/s
no_exponential                       269850 ns/iter (+/- 380)
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

