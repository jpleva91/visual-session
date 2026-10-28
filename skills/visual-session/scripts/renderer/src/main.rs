fn main() {
    if let Err(error) = visual_session::run(std::env::args().skip(1).collect()) {
        eprintln!("visual-session: {error}");
        std::process::exit(1);
    }
}
