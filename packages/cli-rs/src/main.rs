fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(err) = s3_lfs_rs::run(&args) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
