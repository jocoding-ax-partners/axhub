use std::process::Command;

fn main() -> anyhow::Result<()> {
    println!("cargo:rustc-check-cfg=cfg(coverage)");
    println!("cargo:rerun-if-changed=../../src/axhub-helpers/catalog.ts");
    let src = std::fs::read_to_string("../../src/axhub-helpers/catalog.ts")?;
    let json = match axhub_codegen::generate_catalog_json(&src) {
        Ok(json) => json,
        Err(parse_error) => {
            let output = Command::new("bun")
                .arg("-e")
                .arg("import { CATALOG } from './src/axhub-helpers/catalog.ts'; console.log(JSON.stringify(CATALOG, null, 2));")
                .current_dir("../..")
                .output()?;
            if !output.status.success() {
                anyhow::bail!(
                    "catalog parser failed ({parse_error}); bun fallback failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            String::from_utf8(output.stdout)?
        }
    };
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR")?).join("catalog_generated.rs");
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
