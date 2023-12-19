use std::process::exit;

fn main() {
    if let Err(e) = tailr::get_args().and_then(tailr::run) {
        eprintln!("{}", e);
        exit(1);
    }
}
