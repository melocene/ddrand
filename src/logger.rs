use chrono::Local;
use flexi_logger::{FileSpec, Logger, WriteMode};
use log::*;

pub fn init(log_level: &str, log_filename: &str) {
    Logger::try_with_env_or_str(log_level)
        .unwrap()
        .format(format)
        .log_to_file(
            FileSpec::default()
                .directory(".")
                .basename(log_filename)
                .suffix("log"),
        )
        .write_mode(WriteMode::Async)
        .append()
        .duplicate_to_stderr(flexi_logger::Duplicate::All) // Add this
        .start()
        .unwrap();
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
