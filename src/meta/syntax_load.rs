use crate::meta::syntax_dump::SyntaxMapping;
use std::fs;
use std::process::Command;

fn normalize_lexeme(s: &str) -> String {
    let trimmed = s.trim();
    if let Some(without_colon) = trimmed.strip_suffix(':') {
        without_colon.to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn rebuild_from_syntax(mapping: &SyntaxMapping) -> Result<(), String> {
    println!("Rebuilding compiler from syntax mapping...");

    let lexer_path = "src/lexer.rs";
    let lexer_content =
        fs::read_to_string(lexer_path).map_err(|e| format!("Failed to read lexer.rs: {}", e))?;

    let mut new_lexer = lexer_content;

    for (old, new) in mapping.keywords.iter().chain(mapping.operators.iter()) {
        let old = normalize_lexeme(old);
        let new = normalize_lexeme(new);
        if old.is_empty() || new.is_empty() {
            continue;
        }
        let target = format!("#[token(\"{}\")]", old);
        let replacement = format!("#[token(\"{}\")]", new);
        new_lexer = new_lexer.replace(&target, &replacement);
    }

    fs::write(lexer_path, new_lexer).map_err(|e| format!("Failed to write new lexer.rs: {}", e))?;

    println!("✓ Updated src/lexer.rs with new syntax");

    println!("→ Rebuilding compiler (cargo build --release)...");

    let status = Command::new("cargo")
        .args(&["build", "--release"])
        .status()
        .map_err(|e| format!("Failed to run cargo: {}", e))?;

    if !status.success() {
        return Err("Compilation failed".to_string());
    }

    println!("✓ Compiler rebuilt successfully");

    let target_path = "target/release/forge";
    if std::path::Path::new(target_path).exists() {
        println!("New compiler binary is at: {}", target_path);
        println!("You can replace your current binary with:");
        println!("  cp {} $(which forge)", target_path);
    }

    Ok(())
}
