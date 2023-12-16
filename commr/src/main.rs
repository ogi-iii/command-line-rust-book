use std::process::exit;

fn main() {
    if let Err(e) = commr::get_args().and_then(commr::run) {
        eprintln!("{}", e);
        exit(1);
    }
}
