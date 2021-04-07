use log::*;

static LOGGER: ConsoleLogger = ConsoleLogger;

struct ConsoleLogger;

impl Log for ConsoleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= max_level()
    }
    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!(
                "\u{1B}[{}m[{} - {}] {}\u{1B}[0m",
                color_id(record.level()),
                record.level(),
                record.target(),
                record.args()
            );
        }
    }
    fn flush(&self) {}
}

pub fn init() {
    set_logger(&LOGGER).unwrap();
    set_max_level(LevelFilter::Trace);
}

pub fn color_id(level: Level) -> usize {
    match level {
        Level::Error => 31, // Red
        Level::Warn => 93,  // BrightYellow
        Level::Info => 34,  // Blue
        Level::Debug => 32, // Green
        Level::Trace => 90, // BrightBlack
    }
}
