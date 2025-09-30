fn main() {
    // This binary target exists to make `cargo run -p frontend-web` useful during
    // development when `wasm32-unknown-unknown` is not the active target. It
    // simply calls the wasm entrypoint, which will panic when executed natively.
    frontend_web::start_app();
}
