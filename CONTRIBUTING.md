# Contribution guidelines

First off, thank you for considering contributing to `cargo override`.

If you run into any issues contributing, you're very welcome to open a [discussion](https://github.com/eopb/cargo-override/discussions).
We're happy to help.

If your contribution is not straightforward, it may be best to first discuss the change you
wish to make by creating a new issue.

## Reporting issues

Before reporting an issue on our [issue tracker](https://github.com/TrueLayer/cargo-override/issues),
please check that it has not already been reported by searching for some related keywords.

## Opening a PR

Consider including a test which exercises the change you're making.
This can be helpful to ensure your change does not regress,
and make it easier for your PR reviewer to see what your change is looking to achieve.

That being said, don't let the lack of a test prevent you from opening your PR.
If writing a test is difficult or impossible, we can discuss it on the PR.

## Tests

`cargo override` uses standard `cargo` commands for running tests

To run the test suit, use the command:

```shell
cargo test
```

`cargo override` makes heavy use of snapshot tests.
This is so that changes to behaviour, such as subtle changes to toml formatting will show up in code review diffs.

If tests involving snapshots are failing, you can accept the changes with [`cargo insta`](https://insta.rs/):

```shell
cargo insta accept
```
