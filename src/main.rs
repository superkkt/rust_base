mod logger;

fn main() {
    init_logger(log::LevelFilter::Debug);
    log::debug!("Hello World!");
}

fn init_logger(level: log::LevelFilter) {
    log::set_boxed_logger(Box::new(logger::Logger)).unwrap();
    log::set_max_level(level);
}
