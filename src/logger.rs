use chrono::Local;
use flexi_logger::{FileSpec, Logger, LoggerHandle, WriteMode};
use log::*;

pub fn init(
    is_debug: bool,
    log_basename: &str,
) -> Result<LoggerHandle, flexi_logger::FlexiLoggerError> {
    let handle = Logger::try_with_env_or_str(if is_debug { "debug" } else { "info" })?
        .format(format)
        .log_to_file(
            FileSpec::default()
                .directory(".")
                .basename(log_basename)
                .suffix("log")
                .suppress_timestamp(),
        )
        .write_mode(WriteMode::Async)
        .append()
        .start()?;
    Ok(handle)
}

fn format(
    w: &mut dyn std::io::Write,
    _now: &mut flexi_logger::DeferredNow,
    record: &Record,
) -> Result<(), std::io::Error> {
    write!(
        w,
        "{} [{:5}] {}",
        Local::now().format("%Y-%m-%d %H:%M:%S"),
        record.level(),
        record.args()
    )
}
