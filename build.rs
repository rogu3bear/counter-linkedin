use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=target/debug/hash.txt");
    println!("cargo:rerun-if-changed=target/release/hash.txt");

    let output_name =
        env::var("LEPTOS_OUTPUT_NAME").unwrap_or_else(|_| "counter-linkedin".to_string());
    let target = env::var("TARGET").unwrap_or_default();
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    let hash_path = PathBuf::from("target").join(profile).join("hash.txt");
    let pkg_dir = PathBuf::from("target/site/pkg");

    if target == "wasm32-unknown-unknown" {
        clean_generated_pkg_dir(&pkg_dir, &output_name);
    }

    let mut js_file = format!("{output_name}.js");
    let mut wasm_file = format!("{output_name}.wasm");
    let mut css_file = format!("{output_name}.css");

    if let Ok(content) = fs::read_to_string(hash_path) {
        let mut js_hash = None::<String>;
        let mut wasm_hash = None::<String>;
        let mut css_hash = None::<String>;

        for line in content.lines().map(str::trim).filter(|line| !line.is_empty()) {
            if let Some((file, hash)) = line.split_once(':') {
                match file.trim() {
                    "js" => js_hash = Some(hash.trim().to_string()),
                    "wasm" => wasm_hash = Some(hash.trim().to_string()),
                    "css" => css_hash = Some(hash.trim().to_string()),
                    _ => {}
                }
            }
        }

        if let Some(hash) = js_hash {
            js_file = format!("{output_name}.{hash}.js");
        }
        if let Some(hash) = wasm_hash {
            wasm_file = format!("{output_name}.{hash}.wasm");
        }
        if let Some(hash) = css_hash {
            css_file = format!("{output_name}.{hash}.css");
        }
    }

    let generated = format!(
        "pub const JS_FILE: &str = \"{js_file}\";\n\
         pub const WASM_FILE: &str = \"{wasm_file}\";\n\
         pub const CSS_FILE: &str = \"{css_file}\";\n"
    );

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR should be present"));
    fs::write(out_dir.join("asset_manifest.rs"), generated)
        .expect("failed to write generated asset manifest");
}

fn clean_generated_pkg_dir(pkg_dir: &PathBuf, output_name: &str) {
    let Ok(entries) = fs::read_dir(pkg_dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        let is_generated = file_name.starts_with(output_name)
            || file_name.starts_with("__wasm_split")
            || file_name == "snippets";

        if !is_generated {
            continue;
        }

        if path.is_dir() {
            let _ = fs::remove_dir_all(&path);
        } else {
            let _ = fs::remove_file(&path);
        }
    }
}
