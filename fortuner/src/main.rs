use std::process::exit;

fn main() {
    if let Err(e) = fortuner::get_args().and_then(fortuner::run) {
        eprintln!("{}", e);
        exit(1)
    }
}
