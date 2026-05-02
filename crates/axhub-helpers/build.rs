use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    println!("cargo:rustc-check-cfg=cfg(coverage)");
    println!("cargo:rerun-if-changed=data/catalog.json");

    let json = std::fs::read_to_string("data/catalog.json")?;
    let out = PathBuf::from(std::env::var("OUT_DIR")?).join("catalog_generated.rs");
    std::fs::write(
        out,
        format!(
            "pub const CATALOG_JSON: &str = {};\n",
            raw_string_literal(&json)
        ),
    )?;
    Ok(())
}

fn raw_string_literal(s: &str) -> String {
    for hashes in 1..8 {
        let fence = "#".repeat(hashes);
        if !s.contains(&format!("\"{fence}")) {
            return format!("r{fence}\"{s}\"{fence}");
        }
    }
    format!("{:?}", s)
}
