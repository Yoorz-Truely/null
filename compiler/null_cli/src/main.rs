//! The standalone `null` tool: checking and formatting Null files without
//! Sau running (spec, section 21). Not built yet: it only reports its version.

fn main() {
    let version = env!("CARGO_PKG_VERSION");
    match std::env::args().nth(1).as_deref() {
        Some("--version" | "-V") => println!("null {version}"),
        _ => {
            eprintln!("null {version}: nothing is built yet (docs/spec/null-v0.md, section 21)");
            std::process::exit(2);
        }
    }
}
