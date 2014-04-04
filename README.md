Regular expression library written in Rust. Based on RE2.

Currently focusing on hitting feature parity with RE2. (I think this is 
actually pretty much done.)

No benchmarks yet. It's likely pretty slow.

More importantly, no tests yet.

There's no public API yet. See tests in `src/lib.rs` if you want to try it out.

Much of this library (from parsing all the way to the VM itself) is using ideas 
from Russ Cox in his
[article series on regular expressions](http://swtch.com/~rsc/regexp/)
and from the
[Go implementation of RE2](http://golang.org/pkg/regexp/syntax/).


