use clap::Parser;

#[derive(Debug, Parser)]
pub struct Opts {
    /// Print debug information
    #[clap(short, long)]
    pub debug: bool,

    /// Print version information
    #[clap(short, long)]
    pub version: bool,
}
