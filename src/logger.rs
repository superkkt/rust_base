use backtrace::Backtrace;
use log::{Level, Log, Metadata, Record};

pub struct Logger;

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Trace
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) == false {
            return;
        }

        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f %Z");
        if let Some(caller) = caller_name() {
            println!(
                "{}: {}: {}: {}",
                timestamp,
                record.level(),
                caller,
                record.args()
            );
        } else {
            println!("{}: {}: {}", timestamp, record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

#[inline(never)]
fn caller_name() -> Option<String> {
    let backtrace = Backtrace::new();
    let symbol = backtrace
        .frames()
        .iter()
        .flat_map(|frame| frame.symbols())
        .nth(8)?;
    let name = format!("{}", symbol.name()?);
    let name = name.rsplit_once("::")?.0.to_string();
    Some(name)
}
