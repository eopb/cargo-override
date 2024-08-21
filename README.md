# cargo-override

Quickly [override dependencies](https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html) using the `[patch]` section of `Cargo.toml`s.

This plugin adds a new cargo subcommand, `cargo override`, which makes it trivial to patch dependencies.
Just run `cargo override --path path/to/patched/dependency`!

This one command checks all of the things you would otherwise need to check manually:

```toml
# Is this the correct syntax for overriding dependencies?
[patch.crates-io]
#      ^^^^^^^^^ Is this the correct registry for this dependency?
anyhow = { path = "../anyhow" }
#                  ^^^^^^^^^ Does a crate called "anyhow" exist at this path?
#                  ^^^^^^^^^
#                  Does the version of anyhow used here meet the requirement we set in our `Cargo.toml`?
```

> [!NOTE]  
> `carog-override` is still in alpha so there may be rough edges.
> Please let us know if you experience bugs or find areas that can be improved.

