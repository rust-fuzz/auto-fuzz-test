Fuzzing is a great (pen)testing tool, as long as you have one or just a handful of functions you want to fuzz. Fuzzing libpng is no problem, but fuzzing something like OpenSSL is nigh-impossible because you need to manually write the fuzzing boilerplate for every single function.

This is an attempt to make fuzzing libraries with large API surfaces feasible by auto-generating the boilerplate. The process is dead simple:

1. Put a `#[create_cargofuzz_harness]` macro on your function `foo` to find its name and argument types
2. Struct `__fuzz_struct_foo` will be added to the AST, containing all the arguments with `#[derive(Arbitrary)]` on it.
3. Function `__fuzz_foo(input: __fuzz_struct_foo)`, which calls `foo` internally, also will be added.
2. Finally, the boilerplate, which call `__fuzz_foo()` with the [cargo fuzz](https://github.com/rust-fuzz/cargo-fuzz) wil be generated and added to the `fuzz/fuzz_targets` directory of your project.

That's it!

Only standalone functions without borrowed arguments are supported by now.

The implementation is very basic right now, but the idea appears to be workable. Contributions are welcome!

### Running
Attach `#[create_cargofuzz_harness]` to your function
If function is located in module `foo::bar`, write this path as macros argument (`#[create_cargofuzz_harness(foo::bar)]`)
Run this:
```Shell
cargo build
cd fuzz
cargo fuzz run <target name>
```
