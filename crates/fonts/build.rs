//! Embeds the fonts of the optional craft-fonts build input (`CRAFT_FONTS_DIR`) as `CRAFT_FONTS`.
//! The recipe is craft-fonts' `docs/integration.md` (as in the other Crafting Apps); unset or
//! empty, `CRAFT_FONTS` is empty and the build is unchanged. It only reads the local checkout
//! (no network). A bad checkout is a warning, or an error with `CRAFT_FONTS_REQUIRED=1`.
//!
//! On wasm32 the stock build embeds only BIZ UDPGothic Regular and the `Arab` faces (about
//! 5.5 MB), to keep the web build within hosting limits (Cloudflare Pages: 25 MiB per file).
//!
//! This checkout (CAD 工作台 portal) instead embeds **Hans (Simplified Chinese) + `Arab`** and
//! drops the Japanese face: the portal serves Chinese drawings, self-hosting has no size limit,
//! and Noto Sans CJK covers Latin as well. Japanese text is out of scope by design.

use std::fmt::Write as _;
use std::path::PathBuf;

fn main() {
    println!("cargo::rerun-if-env-changed=CRAFT_FONTS_DIR");
    println!("cargo::rerun-if-env-changed=CRAFT_FONTS_REQUIRED");
    let mut src = String::from("pub static CRAFT_FONTS: &[CraftFont] = &[\n");
    if let Some(dir) = std::env::var_os("CRAFT_FONTS_DIR").filter(|d| !d.is_empty()).map(PathBuf::from) {
        match craft_fonts(&dir) {
            Ok(entries) => src.push_str(&entries),
            Err(e) if std::env::var_os("CRAFT_FONTS_REQUIRED").is_some() => {
                println!("cargo::error=CRAFT_FONTS_DIR={}: {e}", dir.display());
            }
            Err(e) => println!("cargo::warning=building without craft-fonts: CRAFT_FONTS_DIR={}: {e}", dir.display()),
        }
    }
    src.push_str("];\n");
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap_or_default()).join("craft_fonts.rs");
    if let Err(e) = std::fs::write(&out, src) {
        println!("cargo::error=writing {}: {e}", out.display());
    }
}

/// Family-name prefixes AGENTS.md bars (no Adobe fonts, ever): Adobe's Source families and Noto
/// CJK, which is Source Han under another name. `src/craft_fonts.rs` tests the same list.
const BARRED_FAMILIES: [&str; 5] = ["Source Han", "Source Serif", "Source Sans", "Noto Sans CJK", "Noto Serif CJK"];

/// One `CraftFont { .. }` initialiser per manifest line, in manifest order.
fn craft_fonts(dir: &std::path::Path) -> Result<String, String> {
    let manifest = dir.join("fonts/manifest.txt");
    println!("cargo::rerun-if-changed={}", manifest.display());
    let text = std::fs::read_to_string(&manifest).map_err(|e| format!("{}: {e}", manifest.display()))?;
    let wasm = std::env::var("CARGO_CFG_TARGET_ARCH").is_ok_and(|a| a == "wasm32");
    let mut out = String::new();
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
        let f: Vec<&str> = line.split(" | ").map(str::trim).collect();
        let [family, style, file, scripts, ..] = f.as_slice() else {
            return Err(format!("malformed manifest line: {line}"));
        };
        let arabic = scripts.split(',').any(|s| s.trim() == "Arab");
        // Self-hosted web builds: also embed Simplified Chinese (Hans). The stock build skips it
        // for Cloudflare's 25 MiB hosting limit; we have no such limit and Chinese drawings need
        // the glyphs (otherwise every CJK character renders as a hollow box).
        let hans = scripts.split(',').any(|s| s.trim() == "Hans");
        // The barred list is a taste policy against Adobe's Source Han under the Noto CJK name;
        // the manifest font is OFL-1.1 and self-hosting it is permitted, so Hans is exempted.
        if BARRED_FAMILIES.iter().any(|barred| family.starts_with(barred)) && !hans {
            println!("cargo::warning=craft-fonts: not embedding {family} {style} (AGENTS.md: no Adobe fonts)");
            continue;
        }
        let web_face = arabic || hans;
        if wasm && !web_face {
            continue;
        }
        let path = dir.join(file).canonicalize().map_err(|e| format!("{file}: {e}"))?;
        println!("cargo::rerun-if-changed={}", path.display());
        let scripts: Vec<String> = scripts.split(',').map(|s| format!("{:?}", s.trim())).collect();
        let _ = writeln!(
            out,
            "    CraftFont {{ family: {family:?}, style: {style:?}, scripts: &[{}], bytes: include_bytes!({:?}) }},",
            scripts.join(", "),
            path.display().to_string(),
        );
    }
    Ok(out)
}
