#[derive(Debug, clap::Parser)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    #[arg(long, env = "GIST_ROOT")]
    pub root: std::path::PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, clap::Subcommand)]
pub enum Command {
    Root,
    Create { names: Vec<String> },
    List,
    Archive { names: Vec<String> },
    Unarchive { names: Vec<String> },
}
