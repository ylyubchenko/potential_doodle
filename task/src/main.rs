//! Project task runner (cargo-xtask pattern): `cargo task <name>`.

use std::path::{Path, PathBuf};
use std::process::{exit, Command};
use std::{env, fs};

type TaskResult = Result<(), String>;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("web") => web(),
        Some("serve") => serve(args.iter().any(|a| a == "--webgpu")),
        Some("ci") => ci(),
        _ => {
            println!("usage: cargo task <command>");
            println!();
            println!("  web               build both wasm flavors and assemble dist/");
            println!("  serve [--webgpu]  dev server with hot reload");
            println!("  ci                native check + wasm check + tests");
            exit(0);
        }
    };
    if let Err(message) = result {
        eprintln!("error: {message}");
        exit(1);
    }
}

/// Repository root (the parent of this crate's directory).
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("task crate lives in the repo root")
        .to_path_buf()
}

fn run(program: &str, args: &[&str]) -> TaskResult {
    println!("> {program} {}", args.join(" "));
    let status = Command::new(program)
        .args(args)
        .current_dir(root())
        .status()
        .map_err(|e| format!("failed to start {program}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} exited with {status}"))
    }
}

/// build.rs writes assets/shaders/params_gen.wgsl during the cargo build,
/// but trunk copies assets/ in parallel with that build — so a trunk-built
/// dist would always carry the previous build's generated file. Running
/// codegen up front keeps assets/ current before trunk starts.
fn prime_codegen() -> TaskResult {
    run("cargo", &["check", "-p", "potential_doodle", "--quiet"])
}

/// Write the WebGPU trunk template, derived from app.html — the only
/// difference is the data-cargo-features attribute.
fn write_webgpu_template() -> Result<PathBuf, String> {
    let app = fs::read_to_string(root().join("app.html"))
        .map_err(|e| format!("reading app.html: {e}"))?;
    let generated = app.replace(r#"rel="rust""#, r#"rel="rust" data-cargo-features="webgpu""#);
    let path = root().join("webgpu.gen.html");
    fs::write(&path, generated).map_err(|e| format!("writing webgpu.gen.html: {e}"))?;
    Ok(path)
}

/// Build both wasm flavors (WebGL2 + WebGPU) and assemble dist/:
///   dist/index.html   loader page, lazy-imports the right bundle (?r= override)
///   dist/assets/      shared assets (the app fetches relative to the page)
///   dist/webgl2/      trunk output, default features, unhashed filenames
///   dist/webgpu/      trunk output, --features webgpu
fn web() -> TaskResult {
    let root = root();
    const FLAGS: [&str; 6] = ["--release", "--public-url", "./", "--filehash", "false", "--dist"];

    prime_codegen()?;

    run(
        "trunk",
        &[&["build"], &FLAGS[..], &["dist/webgl2", "app.html"]].concat(),
    )?;

    let template = write_webgpu_template()?;
    let webgpu_build = run(
        "trunk",
        &[&["build"], &FLAGS[..], &["dist/webgpu", "webgpu.gen.html"]].concat(),
    );
    let _ = fs::remove_file(&template);
    webgpu_build?;

    // Trunk may name the processed page after its source file; direct
    // visits to the flavor dirs expect index.html.
    for (from, to) in [
        ("dist/webgl2/app.html", "dist/webgl2/index.html"),
        ("dist/webgpu/webgpu.gen.html", "dist/webgpu/index.html"),
    ] {
        let from = root.join(from);
        if from.exists() {
            fs::rename(&from, root.join(to)).map_err(|e| format!("renaming {from:?}: {e}"))?;
        }
    }

    // Root copies: loader page (served as index.html) + assets for the
    // lazy-loaded app.
    fs::copy(root.join("loader.html"), root.join("dist/index.html"))
        .map_err(|e| format!("copying loader: {e}"))?;
    let dist_assets = root.join("dist/assets");
    if dist_assets.exists() {
        fs::remove_dir_all(&dist_assets).map_err(|e| format!("clearing dist/assets: {e}"))?;
    }
    copy_dir(&root.join("assets"), &dist_assets)?;

    println!("dist/ assembled: loader + assets + webgl2/ + webgpu/");
    Ok(())
}

fn copy_dir(from: &Path, to: &Path) -> TaskResult {
    fs::create_dir_all(to).map_err(|e| format!("creating {to:?}: {e}"))?;
    for entry in fs::read_dir(from).map_err(|e| format!("reading {from:?}: {e}"))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target).map_err(|e| format!("copying {target:?}: {e}"))?;
        }
    }
    Ok(())
}

/// Dev server with hot reload; WebGL2 by default, --webgpu for the
/// WebGPU flavor (leaves webgpu.gen.html around while serving).
fn serve(webgpu: bool) -> TaskResult {
    prime_codegen()?;
    let template = if webgpu {
        write_webgpu_template()?;
        "webgpu.gen.html"
    } else {
        "app.html"
    };
    run(
        "trunk",
        &["serve", "--release", "--dist", "dist-dev", template],
    )
}

/// The pre-push bundle: native check, wasm check, tests.
fn ci() -> TaskResult {
    run("cargo", &["check", "--workspace"])?;
    run(
        "cargo",
        &["check", "-p", "potential_doodle", "--target", "wasm32-unknown-unknown"],
    )?;
    run("cargo", &["test", "--workspace"])
}
