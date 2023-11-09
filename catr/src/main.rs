use std::process::exit;

fn main() {
    if let Err(err) = catr::get_args()
        .and_then(catr::run) { // unwrap MyResult and pass to run() as a arg
        eprintln!("{}", err);
        exit(1);
    }
}
