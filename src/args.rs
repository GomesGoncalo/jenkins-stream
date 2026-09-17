use clap::Parser;

/// Stream a build output from jenkins
#[derive(Parser, Debug)]
#[command(about, long_about = None)]
pub struct Args {
    /// The Jenkins domain
    #[arg(short, long, env = "JENKINS_URL")]
    pub domain: String,

    /// The builder (job) name — omit to discover interactively
    #[arg(short, long)]
    pub builder: Option<String>,

    /// Job number — omit to use the latest build
    #[arg(short, long)]
    pub number: Option<u32>,

    /// Query time min in ms
    #[arg(long, default_value_t = 100)]
    pub min_query: u64,

    /// Query time max in ms
    #[arg(long, default_value_t = 5000)]
    pub max_query: u64,

    /// Wait for the build to appear
    #[arg(short, long, action)]
    pub wait: bool,
}
