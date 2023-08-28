# cargo-override
The quickest way to patch dependacies in rust projects

## Usage

```
cargo override serde  
```

Does these things
- Checks that `serde` is being used by the current project and what registry it's getting it from by checking the `Cargo.toml`.
- Downloads `serde` from the correct registry in a new local folder `patches/serde`.
- Add a `patch` section to the cargo.toml pointing to the new local folder.

There are also two other low priority variations to this 

```
cargo override https://github.com/serde-rs/serde
```

To fetch the dependancy from git rather than the registry

```
cargo override ../serde  
# or if in the same folder
cargo override ./serde  
```

to not clone anything but just point to a local folder

## Tools

I'm planning on using

- clap
- toml_edit
- cargo-util
- human-panic