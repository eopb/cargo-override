# cargo-override

[![License](https://img.shields.io/crates/l/cargo-override.svg)](https://crates.io/crates/cargo-override)
[![Latest version](https://img.shields.io/crates/v/cargo-override.svg)](https://crates.io/crates/cargo-override)
[![downloads-badge](https://img.shields.io/crates/d/cargo-override.svg)](https://crates.io/crates/cargo-override)

Quickly [override dependencies](https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html) using the `[patch]` section of `Cargo.toml`s.

This plugin adds a new cargo subcommand, `cargo override`, which makes it trivial to patch dependencies with custom local copies, or versions from Git.

`cargo override` infers a number of things you would otherwise need to be checked manually:

```toml
[patch.crates-io]
#      ^^^^^^^^^ The correct registry for this dependency
anyhow = { path = "../anyhow" }
#          ^^^^^^^^^^^^^^^^^ A crate called "anyhow" is exposed at this source
#^^^^^ The name of the crate to patch
#                  ^^^^^^^^^
#                  The version of anyhow exposed here meets, the requirement
#                  we set in our `Cargo.toml`, so it will be valid as a patch
```

> [!NOTE]  
> `cargo-override` is still in alpha so there may be some rough edges.
> Please let us know if you experience bugs or find areas that can be improved, even if the issue is minor.

# Installation

First, ensure that you have a recent version of `cargo` installed.

`cargo override` can then be installed with `cargo`.

```
cargo install cargo-override --locked
```

Alternative installation methods will be avalible in the future.

# Usage

## Overriding dependencies with a local version

To override a dependency with a local copy, use `--path`.

For example, if the relative path `../anyhow` contains a modified copy of the `anyhow` crate:
```
cargo override --path ../anyhow
```

As a result, a patch similar to this one would be appended to your `Cargo.toml`:

```toml
[patch.crates-io]
anyhow = { path = "../anyhow" }
```

## Overriding dependencies with a version from Git

To override a dependency with a Git source, use `--git`.

For example, if `https://github.com/dtolnay/anyhow` contains a new release of the `anyhow` crate,
that is not yet on crates.io:

```
cargo override --git https://github.com/dtolnay/anyhow
```

As a result, a patch similar to this one would be appended to your `Cargo.toml`:

```toml
[patch.crates-io]
anyhow = { git = "https://github.com/dtolnay/anyhow" }
```

Additionally, the flags `--branch`, `--tag` and `--rev` can be used to source the repository at a specific, branch, tag or Git revision, respectively.

