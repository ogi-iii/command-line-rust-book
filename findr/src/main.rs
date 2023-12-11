use std::process::exit;

fn main() {
    if let Err(e) = findr::get_args().and_then(findr::run) {
        eprintln!("{}", e);
        exit(1);
    }
}
