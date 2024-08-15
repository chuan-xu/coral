use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
#[command(next_line_help = true)]
pub struct Param {
    #[arg(long, help = "directory for storing logs")]
    pub dir: Option<String>,

    #[arg(long, help = "Log file name prefix")]
    pub prefix: Option<String>,

    #[arg(long, help = "Log file splitting period")]
    pub rotation: Option<String>,

    #[arg(long, help = "telemetry collector address")]
    pub collector: Option<String>,

    #[arg(long, help = "telemetry resource key value")]
    pub res_kv: Vec<String>,
}
