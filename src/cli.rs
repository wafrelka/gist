#[derive(Debug, clap::Parser)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    #[arg(long, env = "GIST_ROOT", hide_env_values(true))]
    pub root: Option<std::path::PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, clap::Subcommand)]
pub enum Command {
    Root,
    Create { names: Vec<String> },
    List,
    Archive { names: Vec<String> },
    Unarchive { names: Vec<String> },
}
