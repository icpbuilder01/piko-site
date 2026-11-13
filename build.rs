// Embeds a built static site (PIKO_SITE_DIR, e.g. a Vite `dist/`) into the
// wasm at compile time, with a gzip copy of every compressible file.
//
// Output is deterministic: files are sorted, gzip headers carry no name and
// mtime 0, so the same `dist/` always gives the same module hash.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

fn walk(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            walk(root, &p, out);
        } else if p.is_file() {
            out.push(p.strip_prefix(root).unwrap().to_path_buf());
        }
    }
}

fn compressible(path: &str) -> bool {
    let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    matches!(
        ext.as_str(),
        "html" | "js" | "mjs" | "css" | "svg" | "json" | "txt" | "xml" | "wasm" | "map" | "webmanifest" | "ico"
    )
}

fn main() {
    println!("cargo:rerun-if-env-changed=PIKO_SITE_DIR");
    let site = PathBuf::from(
        std::env::var("PIKO_SITE_DIR").expect("set PIKO_SITE_DIR to a built site, see build-site.sh"),
    );
    let site = fs::canonicalize(&site)
        .unwrap_or_else(|e| panic!("PIKO_SITE_DIR {}: {e}", site.display()));
    println!("cargo:rerun-if-changed={}", site.display());

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let gz_dir = out_dir.join("gz");
    let _ = fs::remove_dir_all(&gz_dir);
    fs::create_dir_all(&gz_dir).unwrap();

    let mut files = Vec::new();
    walk(&site, &site, &mut files);
    if files.is_empty() {
        panic!("PIKO_SITE_DIR {} is empty", site.display());
    }
    // (relative path, absolute source) -- a site without its own 404.html
    // gets src/404.html, gzipped like the rest.
    let mut entries: Vec<(String, PathBuf)> = files
        .iter()
        .map(|rel| (rel.to_str().expect("non-UTF-8 path").replace('\\', "/"), site.join(rel)))
        .collect();
    if !entries.iter().any(|(r, _)| r == "404.html") {
        let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        entries.push(("404.html".to_string(), manifest.join("src/404.html")));
    }

    let mut code = String::from("pub static FILES: &[(&str, &[u8], Option<&[u8]>)] = &[\n");
    for (i, (rel_str, abs)) in entries.iter().enumerate() {
        let rel_str = rel_str.as_str();
        println!("cargo:rerun-if-changed={}", abs.display());
        let raw = fs::read(&abs).unwrap();

        let mut gz_entry = "None".to_string();
        if compressible(rel_str) && raw.len() > 256 {
            let mut enc = flate2::GzBuilder::new()
                .mtime(0)
                .write(Vec::new(), flate2::Compression::best());
            enc.write_all(&raw).unwrap();
            let gz = enc.finish().unwrap();
            if gz.len() < raw.len() {
                let gz_path = gz_dir.join(format!("{i}.gz"));
                fs::write(&gz_path, &gz).unwrap();
                gz_entry = format!("Some(include_bytes!({:?}))", gz_path.to_str().unwrap());
            }
        }
        code.push_str(&format!(
            "    ({:?}, include_bytes!({:?}), {}),\n",
            rel_str,
            abs.to_str().unwrap(),
            gz_entry
        ));
    }
    code.push_str("];\n");
    fs::write(out_dir.join("assets.rs"), code).unwrap();
}
