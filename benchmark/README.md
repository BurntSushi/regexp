Rust
----
```
rustc --opt-level=3 -Z lto -g --test --cfg bench src/lib.rs -o ./build/bench
./build/bench --bench

literal                                 446 ns/iter (+/- 11)
not_literal                            1872 ns/iter (+/- 18)
match_class                            2455 ns/iter (+/- 32)
match_class_in_range                   2557 ns/iter (+/- 19)
replace_all                            5798 ns/iter (+/- 70)
anchored_literal_short_non_match       1011 ns/iter (+/- 46)
anchored_literal_long_non_match        9030 ns/iter (+/- 285)
anchored_literal_short_match            615 ns/iter (+/- 25)
anchored_literal_long_match             619 ns/iter (+/- 73)
one_pass_short_a                       2528 ns/iter (+/- 24)
one_pass_short_a_not                   2638 ns/iter (+/- 27)
one_pass_short_b                       1871 ns/iter (+/- 20)
one_pass_short_b_not                   2078 ns/iter (+/- 23)
one_pass_long_prefix                   1598 ns/iter (+/- 19)
one_pass_long_prefix_not               1602 ns/iter (+/- 28)
easy0_32                                603 ns/iter (+/- 31) = 53 MB/s
easy0_1K                               2416 ns/iter (+/- 517) = 423 MB/s
easy0_32K                             51596 ns/iter (+/- 1806) = 635 MB/s
easy0_1M                            1619241 ns/iter (+/- 47681) = 646 MB/s
easy1_32                                646 ns/iter (+/- 152) = 49 MB/s
easy1_1K                               3138 ns/iter (+/- 639) = 326 MB/s
easy1_32K                             83607 ns/iter (+/- 6703) = 391 MB/s
easy1_1M                            2644269 ns/iter (+/- 92430) = 396 MB/s
medium_32                              1615 ns/iter (+/- 88) = 19 MB/s
medium_1K                             34430 ns/iter (+/- 991) = 29 MB/s
medium_32K                          1036442 ns/iter (+/- 57986) = 31 MB/s
medium_1M                          34140437 ns/iter (+/- 1768805) = 30 MB/s
hard_32                                2466 ns/iter (+/- 104) = 12 MB/s
hard_1K                               55462 ns/iter (+/- 1596) = 18 MB/s
hard_32K                            1753933 ns/iter (+/- 4645) = 18 MB/s
hard_1M                            56102587 ns/iter (+/- 387536) = 17 MB/s
no_exponential                       272928 ns/iter (+/- 1927)
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

