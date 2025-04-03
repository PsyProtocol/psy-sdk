mod cli;
pub mod errors;

use std::env;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::EnvFilter;
const PANIC_MESSAGE: &str = "Program panic. This is a bug to be fixed.";

fn main() {
    setup_tracing();

    // Register a panic hook to display more readable panic messages to end-users
    let (panic_hook, _) = color_eyre::config::HookBuilder::default()
        .display_env_section(false)
        .panic_section(PANIC_MESSAGE)
        .into_hooks();
    panic_hook.install();

    if let Err(report) = cli::start_cli() {
        eprintln!("{report:#}");
        std::process::exit(1);
    }
}

fn setup_tracing() {
    let subscriber = tracing_subscriber::fmt()
        .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
        .with_env_filter(EnvFilter::from_env("DARGO_LOG"));

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
