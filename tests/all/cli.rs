use std::path::PathBuf;

use clap::{CommandFactory, Parser};
use fake::{Fake, Faker};
use googletest::{
    expect_that,
    matchers::{eq, matches_pattern, ok, some},
};

use cargo_override::{cli, CargoInvocation, Cli};

#[googletest::test]
fn path_parse_from_args() {
    for base_command in ["cargo override", "cargo-override"] {
        let path: PathBuf = Faker.fake();

        let path = path.to_str().unwrap();

        let output = Cli::try_parse_from([base_command, "override", "--path", path]);

        expect_that!(
            output,
            ok(matches_pattern!(Cli {
                command: matches_pattern!(CargoInvocation::Override(matches_pattern!(
                    cli::Override {
                        source: matches_pattern!(cli::Source {
                            path: some(eq(path))
                        })
                    }
                )))
            }))
        )
    }
}

#[googletest::test]
fn override_subcommand_help_message() {
    insta::allow_duplicates! {
        for base_command in ["cargo override", "cargo-override"] {
            let output = Cli::try_parse_from([base_command, "override", "--help"]);

            let output = output.expect_err("`--help` messages comes up as an `Result::Err`");

            let output = strip_ansi_escapes::strip_str(format!("{}", output.render().ansi()));

            insta::assert_snapshot!(output, @r###"
            Quickly override dependencies using the `[patch]` section of `Cargo.toml`s.

            Usage: cargo override [OPTIONS] <--path <PATH>|--git <URI>>

            Options:
                  --path <PATH>
                      Path to patched dependency, to use in override
                  --git <URI>
                      Git URL to source override from
                  --branch <BRANCH>
                      Branch to use when overriding from git
                  --tag <TAG>
                      Tag to use when overriding from git
                  --rev <REV>
                      Specific commit to use when overriding from git
                  --registry <REGISTRY>
                      Name of the registry to use. Usually `cargo-override` can correctly determine which regestiry to use without needing this flag
                  --manifest-path <MANIFEST_PATH>
                      Path to the `Cargo.toml` file that needs patching. By default, `cargo-override` searches for the `Cargo.toml` file in the current directory or any parent directory
                  --locked
                      Assert that `Cargo.lock` will remain unchanged
                  --offline
                      Prevents cargo from accessing the network
                  --frozen
                      Equivalent to specifying both --locked and --offline
                  --force
                      Force the override, ignoring compatibility checks
              -h, --help
                      Print help
              -V, --version
                      Print version
            "###);
        }
    }
}

#[googletest::test]
fn base_help_message() {
    insta::allow_duplicates! {
        for base_command in ["cargo override", "cargo-override"] {
            let output = Cli::try_parse_from([base_command, "--help"]);

            let output = output.expect_err("`--help` messages comes up as an `Result::Err`");

            let output = strip_ansi_escapes::strip_str(format!("{}", output.render().ansi()));

            insta::assert_snapshot!(output, @r###"
            Quickly override dependencies using the `[patch]` section of `Cargo.toml`s.

            Usage: cargo <COMMAND>

            Commands:
              override  Quickly override dependencies using the `[patch]` section of `Cargo.toml`s.
              help      Print this message or the help of the given subcommand(s)

            Options:
              -h, --help     Print help
              -V, --version  Print version
            "###);
        }
    }
}

#[test]
fn verify_cli() {
    Cli::command().debug_assert();
}
