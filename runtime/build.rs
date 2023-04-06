fn main() {
    println!(
        "cargo:rustc-env=TARGET={}",
        std::env::var("TARGET").unwrap()
    );
    println!(
        "cargo:rustc-env=PROFILE={}",
        std::env::var("PROFILE").unwrap()
    );
}
