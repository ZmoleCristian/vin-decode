use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=data");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let data_dir = manifest.join("data");
    let embedded_rs = out_dir.join("embedded_data.rs");

    let version = fs::read_to_string(data_dir.join("VERSION"))
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "0.0.0-dev".to_string());

    let entries = enumerate_entries(&data_dir);
    let mut buf = String::new();
    buf.push_str(&format!("pub(crate) const VERSION: &str = {version:?};\n"));
    buf.push_str("pub(crate) const FILES: &[(&str, &[u8], bool)] = &[\n");
    for (_, source_path, is_compressed, dest_name) in &entries {
        let abs = source_path.to_string_lossy();
        buf.push_str(&format!(
            "    ({dest:?}, include_bytes!({src:?}), {compressed}),\n",
            dest = dest_name,
            src = abs,
            compressed = is_compressed,
        ));
    }
    buf.push_str("];\n");

    fs::write(&embedded_rs, buf).expect("write embedded_data.rs");
}

fn enumerate_entries(data_dir: &Path) -> Vec<(String, PathBuf, bool, String)> {
    let mut out = Vec::new();
    let Ok(read) = fs::read_dir(data_dir) else {
        return out;
    };
    let mut paths: Vec<_> = read.flatten().map(|e| e.path()).collect();
    paths.sort();
    for path in paths {
        let Some(file_name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if file_name == "VERSION" || file_name.starts_with('.') {
            continue;
        }
        if file_name.ends_with(".bin.zst") {
            let dest = file_name.trim_end_matches(".zst").to_string();
            out.push((file_name.to_string(), path.clone(), true, dest));
        } else if file_name.ends_with(".fst") {
            out.push((
                file_name.to_string(),
                path.clone(),
                false,
                file_name.to_string(),
            ));
        }
    }
    out
}
