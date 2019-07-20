Fuzzing is a great (pen)testing tool, as long as you have one or just a handful of functions you want to fuzz. Fuzzing libpng is no problem, but fuzzing something like OpenSSL is nigh-impossible because you need to manually write the fuzzing boilerplate for every single function.

This is an attempt to make fuzzing libraries with large API surfaces feasible by auto-generating the boilerplate. The process is dead simple:

1. Parse the source code with [`syn`](https://github.com/dtolnay/syn) to find name of functions and their argument types
2. Generate boilerplate that converts random bytes into Rust types via [QuickCheck](https://github.com/BurntSushi/quickcheck)'s [`Arbitrary` trait](https://docs.rs/quickcheck/0.8.5/quickcheck/trait.Arbitrary.html)

That's it! Thanks to Rust's safety guarantees, any segfault encountered through safe APIs is guaranteed to be a vulnerability.

Only [AFL](https://github.com/rust-fuzz/afl.rs) fuzzer is currently supported. We've also tried [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) but that's [blocked](https://github.com/Eh2406/auto-fuzz-test/issues/9) by an upstream bug.

The implementation is very basic right now, but the idea appears to be workable. Contributions are welcome!
