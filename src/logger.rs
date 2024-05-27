use std::{fs::OpenOptions, str::FromStr};
use tracing_subscriber::{filter::LevelFilter, fmt::time::ChronoLocal, prelude::*};

pub fn init(level: &str, log_file: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let mut layers = vec![];

    // Ignore file writing if a filename was not provided for the log.
    // Even with a valid filename, logging to a file is considered not required so will get ignored on write errors.
    if let Some(fname) = log_file {
        match OpenOptions::new().append(true).create(true).open(fname) {
            Ok(log_file_path) => {
                let log_file = tracing_subscriber::fmt::layer()
                    .with_ansi(false)
                    .with_timer(ChronoLocal::rfc_3339())
                    .with_target(false)
                    .with_writer(log_file_path)
                    .compact()
                    .with_filter(LevelFilter::from_str(level).unwrap_or(LevelFilter::INFO))
                    .boxed();
                layers.push(log_file);
            }
            Err(e) => {
                eprintln!("ERROR: Could not write log file, will only log to stdout");
                eprintln!("{}", e);
            }
        }
    }

    tracing_subscriber::registry().with(layers).init();

    Ok(())
}
