use camino::Utf8PathBuf;
use clap::Parser;
use url::Url;

#[derive(Parser, Debug)]
#[command(bin_name = "cargo")]
pub struct Cli {
    #[command(subcommand)]
    pub command: CargoInvocation,
}

#[derive(Parser, Debug)]
pub enum CargoInvocation {
    #[command(name = "override")]
    Override {
        #[arg(long)]
        path: Option<Utf8PathBuf>,

        #[arg(long)]
        git: Option<Url>,
        #[arg(long)]
        branch: Option<String>,
        #[arg(long)]
        tag: Option<String>,
        #[arg(long)]
        rev: Option<String>,

        #[arg(long)]
        registry: Option<String>,

        #[arg(long)]
        manifest_path: Option<Utf8PathBuf>,

        /// Assert that `Cargo.lock` will remain unchanged
        #[arg(long)]
        locked: bool,
        /// Run without accessing the network
        #[arg(long)]
        offline: bool,
        /// Equivalent to specifying both --locked and --offline
        #[arg(long)]
        frozen: bool,
        #[arg(long, hide = true)]
        no_deps: bool,
    },
}
