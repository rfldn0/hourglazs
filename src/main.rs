fn main() {
    let result = hourglazs::cli::parse(std::env::args().skip(1)).and_then(hourglazs::cli::run);
    if let Err(e) = result {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}
