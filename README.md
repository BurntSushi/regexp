Regular expression library written in Rust. Based on RE2.

Currently focusing on hitting feature parity with RE2. (I think this is 
actually pretty much done.)

Have done some preliminary benchmarks with @JustAPerson on the regex-dna 
benchmark. Seems to be within spitting distance of RE2/Go.

Tests include a sizable portion from Glenn Fowler's testregex test suite. 
Currently passing all of them.

There are some other tests that makes sure the parser rejects invalid input 
without crashing (or blowing the stack).

Public API is in progress but in a reasonable state: 
http://burntsushi.net/rustdoc/regexp/

Much of this library (from parsing all the way to the VM itself) is using ideas 
from Russ Cox in his
[article series on regular expressions](http://swtch.com/~rsc/regexp/)
and from the
[Go implementation of RE2](http://golang.org/pkg/regexp/syntax/).

