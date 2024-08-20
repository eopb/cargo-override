use camino::Utf8PathBuf;
use clap::{Args, Parser};
use url::Url;

#[derive(Parser, Debug)]
#[command(bin_name = "cargo", version, about)]
#[command(propagate_version = true)]
#[cfg_attr(debug_assertions, command(term_width = 0))] // Disable `--help` linewrap for tests since it makes snapshots flaky
pub struct Cli {
    #[command(subcommand)]
    pub command: CargoInvocation,
}

#[derive(Parser, Debug)]
pub enum CargoInvocation {
    #[command(name = "override", about)]
    #[command(next_line_help = true)]
    Override(Override),
}

#[derive(Args, Debug)]
pub struct Override {
    #[command(flatten)]
    pub source: Source,

    #[command(flatten)]
    pub git: Git,

    #[arg(long)]
    /// Name of the registry to use.
    /// Usually `cargo-override` can correctly determine which regestiry to use without needing this flag
    pub registry: Option<String>,

    /// Path to the `Cargo.toml` file that needs patching.
    /// By default, `cargo-override` searches for the `Cargo.toml` file in the current directory or any parent directory
    #[arg(long)]
    pub manifest_path: Option<Utf8PathBuf>,

    /// Assert that `Cargo.lock` will remain unchanged
    #[arg(long)]
    pub locked: bool,
    /// Prevents cargo from accessing the network
    #[arg(long)]
    pub offline: bool,
    /// Equivalent to specifying both --locked and --offline
    #[arg(long)]
    pub frozen: bool,
    #[arg(long, hide = true)]
    pub no_deps: bool,
}

#[derive(Args, Debug)]
#[group(required = true, multiple = false)]
pub struct Source {
    /// Path to patched dependency, to use in override
    #[arg(long)]
    pub path: Option<Utf8PathBuf>,

    /// Git URL to source override from
    #[arg(long, value_name = "URI", group = "git-group")]
    pub git: Option<Url>,
}

#[derive(Args, Clone, Debug, Default)]
#[group(required = false, multiple = false, requires = "git-group")]
pub struct Git {
    /// Branch to use when overriding from git
    #[arg(long)]
    pub branch: Option<String>,
    /// Tag to use when overriding from git
    #[arg(long)]
    pub tag: Option<String>,
    /// Specific commit to use when overriding from git
    #[arg(long)]
    pub rev: Option<String>,
}
