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
                "[{} - {}] {}",
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
