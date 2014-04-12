Rust
----
```
rustc --opt-level=3 -Z lto -g --test --cfg bench src/lib.rs -o ./build/bench
./build/bench --bench

literal                                 658 ns/iter (+/- 4)
not_literal                            2690 ns/iter (+/- 8)
match_class                            3374 ns/iter (+/- 19)
match_class_in_range                   3493 ns/iter (+/- 12)
replace_all                            5526 ns/iter (+/- 27)
anchored_literal_short_non_match       1955 ns/iter (+/- 15)
anchored_literal_long_non_match       13056 ns/iter (+/- 51)
anchored_literal_short_match           1501 ns/iter (+/- 13)
anchored_literal_long_match            1504 ns/iter (+/- 22)
one_pass_short_a                       4112 ns/iter (+/- 26)
one_pass_short_a_not                   4128 ns/iter (+/- 17)
one_pass_short_b                       3053 ns/iter (+/- 25)
one_pass_short_b_not                   3274 ns/iter (+/- 19)
one_pass_long_prefix                   4499 ns/iter (+/- 51)
one_pass_long_prefix_not               4480 ns/iter (+/- 13)
easy0_32                               2060 ns/iter (+/- 23) = 15 MB/s
easy0_1K                               3279 ns/iter (+/- 115) = 312 MB/s
easy0_32K                             41892 ns/iter (+/- 722) = 782 MB/s
easy0_1M                            1278983 ns/iter (+/- 14709) = 818 MB/s
easy1_32                               1692 ns/iter (+/- 108) = 18 MB/s
easy1_1K                               4261 ns/iter (+/- 991) = 240 MB/s
easy1_32K                             84830 ns/iter (+/- 5651) = 386 MB/s
easy1_1M                            2665892 ns/iter (+/- 41217) = 393 MB/s
medium_32                              3272 ns/iter (+/- 48) = 9 MB/s
medium_1K                             42915 ns/iter (+/- 761) = 23 MB/s
medium_32K                          1315950 ns/iter (+/- 5338) = 24 MB/s
medium_1M                          42051117 ns/iter (+/- 80800) = 24 MB/s
hard_32                                5207 ns/iter (+/- 54) = 6 MB/s
hard_1K                               68031 ns/iter (+/- 302) = 15 MB/s
hard_32K                            2079640 ns/iter (+/- 7819) = 15 MB/s
hard_1M                            66479353 ns/iter (+/- 89056) = 15 MB/s
no_exponential                       326157 ns/iter (+/- 1280)
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

