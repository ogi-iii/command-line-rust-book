use std::process::exit;

fn main() {
    if let Err(e) = cutr::get_args().and_then(cutr::run) {
        eprintln!("{}", e);
        exit(1);
    }
}
