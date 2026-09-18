use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let root = Path::new(&manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("Failed to get workspace root");

    println!("cargo:rerun-if-changed={}", root.join("ui").display());

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set by cargo");
    let output_path = Path::new(&out_dir).join("tailwind.css");

    let status = Command::new("tailwindcss")
        .current_dir(root)
        .args([
            "-c",
            "ui/tailwind.config.ts",
            "-i",
            "ui/styles/tailwind-input.css",
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

    let js_output_path = Path::new(&out_dir).join("tmtbook.js");
    let esbuild_status = Command::new("esbuild")
        .current_dir(root)
        .args([
            "ui/src/index.ts",
            "--bundle",
            "--minify",
            &format!(
                "--outfile={}",
                js_output_path.to_str().expect("OUT_DIR is not valid UTF-8")
            ),
        ])
        .status();

    match esbuild_status {
        Ok(s) if s.success() => {}
        Ok(s) => panic!("`esbuild` exited with {s}"),
        Err(e) => panic!(
            "failed to run `esbuild` ({e}). It's provided by the nix dev shell \
             (nix/dev.nix) -- run this build from inside `nix develop` / direnv."
        ),
    }
}
