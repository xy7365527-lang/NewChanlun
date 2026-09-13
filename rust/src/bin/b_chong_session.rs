//! #1374 单个持久重 B 的独立 owner，正式入口。
fn main() {
    if let Err(error) = newchan_rust::economic_session::cli(true) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
