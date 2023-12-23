use std::process::exit;

fn main() {
    if let Err(e) = calr::get_args().and_then(calr::run) {
        eprintln!("{}", e);
        exit(1);
    }
}
