Fuzzing is a great (pen)testing tool, as long as you have one or just a handful of functions you want to fuzz. Fuzzing libpng is no problem, but fuzzing something like OpenSSL is nigh-impossible because you need to manually write the fuzzing boilerplate for every single function.

This is an attempt to make fuzzing libraries with large API surfaces feasible by auto-generating the boilerplate. The process is dead simple:

1. Parse the source code with `syn` to find name of functions and their argument types
2. Generate boilerplate that converts random bytes into Rust types via QuickCheck's `Arbitrary` trait
3. That's it!

Only AFL fuzzer is currently supported. We've tried cargo-fuzz but that's blocked by an upstream bug.

This is currently fairly basic, but the idea appears to be workable. Contributions are welcome!
