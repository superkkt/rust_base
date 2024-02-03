use log::error;
use stderrlog;
use backtrace::Backtrace;

fn main() {
    init_logger(log::Level::Debug);
    error!("Hello, world!");
}

fn init_logger(verbosity: log::Level) {
    // TODO: implement a custom logger to log caller's function name.
    //
    // use backtrace::Backtrace;
    //
    // #[inline(never)]
    // fn caller_name() -> Option<String> {
    //     let backtrace = Backtrace::new();
    //     let symbol = backtrace
    //         .frames()
    //         .iter()
    //         .flat_map(|frame| frame.symbols())
    //         .nth(1)?;
    //     let name = format!("{}", symbol.name()?);
    //     let name = name.rsplit_once("::")?.0.to_string();
    //     Some(name)
    // }
    //
    // https://docs.rs/log/latest/log/#implementing-a-logger

    if let Some(name) = caller_name() {
        println!("caller: {name}");
    }

    stderrlog::new()
        .module(module_path!())
        .verbosity(verbosity)
        .timestamp(stderrlog::Timestamp::Millisecond)
        .init()
        .unwrap();
}

#[inline(never)]
fn caller_name() -> Option<String> {
    let backtrace = Backtrace::new();
    let symbol = backtrace
        .frames()
        .iter()
        .flat_map(|frame| frame.symbols())
        .nth(7)?;
    let name = format!("{}", symbol.name()?);
    let name = name.rsplit_once("::")?.0.to_string();
    Some(name)
}