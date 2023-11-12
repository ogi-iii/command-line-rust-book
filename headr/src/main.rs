use std::process::exit;

fn main() {
    if let Err(e) = headr::get_args().and_then(headr::run) {
        eprintln!("{}", e);
        exit(1);
    }
}
