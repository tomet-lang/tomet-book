use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=tailwind.config.js");
    println!("cargo:rerun-if-changed=src/assets/tailwind-input.css");
    println!("cargo:rerun-if-changed=src/assets/templates");

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set by cargo");
    let output_path = Path::new(&out_dir).join("tailwind.css");

    let status = Command::new("tailwindcss")
        .args([
            "-i",
            "src/assets/tailwind-input.css",
            "-o",
            output_path.to_str().expect("OUT_DIR is not valid UTF-8"),
            "--minify",
        ])
        .status();

    match status {
        Ok(s) if s.success() => {}
        Ok(s) => panic!("`tailwindcss` exited with {s}"),
        Err(e) => panic!(
            "failed to run `tailwindcss` ({e}). It's provided by the nix dev shell \
             (nix/dev.nix) -- run this build from inside `nix develop` / direnv."
        ),
    }
}
