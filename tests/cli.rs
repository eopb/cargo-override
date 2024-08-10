mod manifest;

use clap::Parser;

use cargo_override::Cli;

#[googletest::test]
fn override_subcommand_help_message() {
    insta::allow_duplicates! {
        for base_command in ["cargo override", "cargo-override"] {
            let output = Cli::try_parse_from([base_command, "override", "--help"]);

            let output = output.expect_err("`--help` messages comes up as an `Result::Err`");

            let output = strip_ansi_escapes::strip_str(format!("{}", output.render().ansi()));

            insta::assert_toml_snapshot!(output, @r###"
            '''
            Usage: cargo override --path <PATH>

            Options:
              -p, --path <PATH>  
              -h, --help         Print help
            '''
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

            insta::assert_toml_snapshot!(output, @r###"
            '''
            Usage: cargo <COMMAND>

            Commands:
              override  
              help      Print this message or the help of the given subcommand(s)

            Options:
              -h, --help  Print help
            '''
            "###);
        }
    }
}
