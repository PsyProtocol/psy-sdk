mod cli;
pub mod errors;

use std::env;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::format::FmtSpan;

fn main() {
    setup_tracing();

    if let Err(report) = cli::start_cli() {
        eprintln!("{report:#}");
        std::process::exit(1);
    }
    println!("Hello, world!");
}

fn setup_tracing() {
    let subscriber = tracing_subscriber::fmt()
        .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
        .with_env_filter(EnvFilter::from_env("NOIR_LOG"));

    if let Ok(log_dir) = env::var("DARGO_LOG_DIR") {
        let debug_file = tracing_appender::rolling::daily(log_dir, "dargo-log");
        subscriber
            .with_writer(debug_file)
            .with_ansi(false)
            .json()
            .init();
    } else {
        subscriber
            .with_writer(std::io::stderr)
            .with_ansi(true)
            .init();
    }
}
