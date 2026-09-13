//! #1374 E 独立经济原事实 owner，正式入口。
fn main() {
    if let Err(error) = newchan_rust::economic_session::cli(false) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
