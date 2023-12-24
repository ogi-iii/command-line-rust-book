use std::process::exit;

fn main() {
    if let Err(e) = lsr::get_args().and_then(lsr::run) {
        eprintln!("{}", e);
        exit(1);
    }
}
