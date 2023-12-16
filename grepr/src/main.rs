use std::process::exit;

fn main() {
    if let Err(e) = grepr::get_args().and_then(grepr::run) {
        eprintln!("{}", e);
        exit(1);
    }
}
