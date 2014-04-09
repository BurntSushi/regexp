Rust
----
```
rustc --opt-level=3 -Z lto -g --test --cfg bench src/lib.rs -o ./build/bench
./build/bench --bench

literal                                3425 ns/iter (+/- 87)
not_literal                            4016 ns/iter (+/- 130)
match_class                            5470 ns/iter (+/- 171)
match_class_in_range                   5599 ns/iter (+/- 196)
replace_all                            7479 ns/iter (+/- 506)
anchored_literal_short_non_match       2917 ns/iter (+/- 183)
anchored_literal_long_non_match       20746 ns/iter (+/- 1348)
anchored_literal_short_match           2083 ns/iter (+/- 129)
anchored_literal_long_match            4323 ns/iter (+/- 193)
one_pass_short_a                       4632 ns/iter (+/- 148)
one_pass_short_a_not                   4577 ns/iter (+/- 268)
one_pass_short_b                       3960 ns/iter (+/- 132)
one_pass_short_b_not                   4175 ns/iter (+/- 399)
one_pass_long_prefix                   5296 ns/iter (+/- 270)
one_pass_long_prefix_not               5382 ns/iter (+/- 285)
easy0_32                               4862 ns/iter (+/- 112)     = 6 MB/s
easy0_1K                              53492 ns/iter (+/- 3837)    = 19 MB/s
easy0_32K                           1598644 ns/iter (+/- 95182)   = 20 MB/s
easy0_1M                           52294598 ns/iter (+/- 2254936) = 19 MB/s
easy1_32                               3644 ns/iter (+/- 123)     = 8 MB/s
easy1_1K                              52566 ns/iter (+/- 1383)    = 19 MB/s
easy1_32K                           1592652 ns/iter (+/- 58514)   = 20 MB/s
easy1_1M                           51788272 ns/iter (+/- 1152957) = 19 MB/s
medium_32                              5243 ns/iter (+/- 213)     = 6 MB/s
medium_1K                             64623 ns/iter (+/- 3563)    = 15 MB/s
medium_32K                          1866701 ns/iter (+/- 66780)   = 17 MB/s
medium_1M                          60509478 ns/iter (+/- 761637)  = 16 MB/s
hard_32                                6285 ns/iter (+/- 114)     = 5 MB/s
hard_1K                               85483 ns/iter (+/- 2719)    = 11 MB/s
hard_32K                            2620292 ns/iter (+/- 41568)   = 12 MB/s
hard_1M                            84727823 ns/iter (+/- 2590458) = 11 MB/s
```

Golang
------
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

RE2/Rust gets absolutely clobbered by RE2/Go in the Easy{0,1} benchmarks. 
Interestingly, Rust does the same or better on the Medium/Hard benchmarks. My 
suspicion is that RE2/Go is performing some optimizations on the easy 
benchmarks to make the throughput very high. This gives me hope.

For example, the EASY0 regex is just matching a literal string at the end of a 
search string, which could be recognized as such by RE2/Go and fall back to 
simply checking the suffix of a string.

The EASY1 regex isn't as fast, but I think is also optimized as described in 
the section on the 'FilteredRE2' class here:
http://swtch.com/~rsc/regexp/regexp3.html

Interestingly, it looks like the MEDIUM regex isn't subjected to a similar 
optimization?

The HARD regex is certainly difficult to optimize because of the leading
`[ -~]*` which probably explains its lower throughput.

Note that BOTH RE2/Rust and RE2/Go are benchmarked with an implicit `.*?` 
prefixing all regular expressions. (i.e., They are unachored.)

Also note that RE2/Rust is performing much worse on the small Medium/Hard 
benchmarks (searching 32 bytes of text). My suspicion is that there are some 
big constant factors lurking somewhere that need to be fixed in RE2/Rust.
This may also explain some of the performance difference in other benchmarks 
(NOT easy/medium/hard) since they mostly work with shortish search strings.
(Although this is not true for all, since some specifically target the presence 
of optimizations in RE2/Go.)

