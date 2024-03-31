# Contributing

## Test Coverage

Test coverage is reported to [Codecov](https://codecov.io) via a CI workflow.
However, Codecov doesn't do a good job of displaying region coverage. If you're
trying to track down untested code paths, you may want to generate a report
locally using [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov).

Here's the command:

```shell
cargo +nightly llvm-cov --all-features --open
```

Using the nightly toolchain for code coverage allows for excluding certain
functions (e.g. debug impls) from coverage reporting. The tool will still work
with the stable toolchain, however.
