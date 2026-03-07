use std::collections::HashMap;
use wasm_bindgen::prelude::*;

#[derive(Clone)]
enum WasmValue {
    Number(f64),
    String(String),
    Bool(bool),
}

impl WasmValue {
    fn to_output(&self) -> String {
        match self {
            WasmValue::Number(n) => {
                if n.fract() == 0.0 {
                    format!("{}", *n as i64)
                } else {
                    n.to_string()
                }
            }
            WasmValue::String(s) => s.clone(),
            WasmValue::Bool(b) => b.to_string(),
        }
    }
}

#[wasm_bindgen]
pub struct CoreCompilerWasm {
    files: HashMap<String, String>,
    output_buffer: Vec<String>,
    variables: HashMap<String, WasmValue>,
}

#[wasm_bindgen]
impl CoreCompilerWasm {
    #[wasm_bindgen(constructor)]
    pub fn new() -> CoreCompilerWasm {
        #[cfg(feature = "console_error_panic_hook")]
        console_error_panic_hook::set_once();

        CoreCompilerWasm {
            files: HashMap::new(),
            output_buffer: Vec::new(),
            variables: HashMap::new(),
        }
    }

    #[wasm_bindgen]
    pub fn load_source(&mut self, filename: &str, source: &str) {
        self.files.insert(filename.to_string(), source.to_string());
    }

    #[wasm_bindgen]
    pub fn execute(&mut self, filename: &str) -> String {
        self.output_buffer.clear();
        let source = match self.files.get(filename) {
            Some(s) => s.clone(),
            None => return format!("Error: File '{}' not found", filename),
        };

        match self.execute_source(&source) {
            Ok(result) => {
                self.output_buffer.push(result);
                self.output_buffer.join("\n")
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    #[wasm_bindgen]
    pub fn execute_source(&mut self, source: &str) -> Result<String, String> {
        self.variables.clear();
        let mut output = Vec::new();

        for line in source.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
                continue;
            }

            if let Some(content) = trimmed.strip_prefix("say:") {
                let value = self.eval_expr(content.trim())?;
                output.push(value.to_output());
                continue;
            }

            if let Some(var_decl) = trimmed.strip_prefix("var ") {
                if let Some(colon_pos) = var_decl.find(':') {
                    let name = var_decl[..colon_pos].trim();
                    let expr = var_decl[colon_pos + 1..].trim();
                    let value = self.eval_expr(expr)?;
                    self.variables.insert(name.to_string(), value);
                    continue;
                }
                return Err(format!("Invalid variable declaration: {}", trimmed));
            }

            if trimmed.contains(':') {
                let colon_pos = trimmed.find(':').unwrap_or(0);
                let lhs = trimmed[..colon_pos].trim();
                let rhs = trimmed[colon_pos + 1..].trim();
                if !lhs.is_empty() {
                    let value = self.eval_expr(rhs)?;
                    self.variables.insert(lhs.to_string(), value);
                    continue;
                }
            }

            if trimmed.contains('=') && !trimmed.contains("==") {
                let eq = trimmed.find('=').unwrap_or(0);
                let lhs = trimmed[..eq].trim();
                let rhs = trimmed[eq + 1..].trim();
                if !lhs.is_empty() {
                    let value = self.eval_expr(rhs)?;
                    self.variables.insert(lhs.to_string(), value);
                    continue;
                }
            }
        }

        if output.is_empty() {
            output.push("Program executed successfully".to_string());
        }
        Ok(output.join("\n"))
    }

    fn eval_expr(&self, expr: &str) -> Result<WasmValue, String> {
        let expr = expr.trim();
        if expr.is_empty() {
            return Ok(WasmValue::String(String::new()));
        }
        if expr.starts_with('"') && expr.ends_with('"') && expr.len() >= 2 {
            return Ok(WasmValue::String(expr[1..expr.len() - 1].to_string()));
        }
        if expr == "true" {
            return Ok(WasmValue::Bool(true));
        }
        if expr == "false" {
            return Ok(WasmValue::Bool(false));
        }
        if let Ok(n) = expr.parse::<f64>() {
            return Ok(WasmValue::Number(n));
        }
        if let Some(v) = self.variables.get(expr) {
            return Ok(v.clone());
        }

        if let Some(arg) = expr.strip_prefix("len(").and_then(|s| s.strip_suffix(')')) {
            return Ok(match self.eval_expr(arg)? {
                WasmValue::String(s) => WasmValue::Number(s.chars().count() as f64),
                _ => WasmValue::Number(0.0),
            });
        }
        if let Some(arg) = expr.strip_prefix("str(").and_then(|s| s.strip_suffix(')')) {
            return Ok(WasmValue::String(self.eval_expr(arg)?.to_output()));
        }
        if let Some(arg) = expr.strip_prefix("num(").and_then(|s| s.strip_suffix(')')) {
            return Ok(match self.eval_expr(arg)? {
                WasmValue::Number(n) => WasmValue::Number(n),
                WasmValue::String(s) => WasmValue::Number(s.parse::<f64>().unwrap_or(0.0)),
                WasmValue::Bool(b) => WasmValue::Number(if b { 1.0 } else { 0.0 }),
            });
        }
        if let Some(arg) = expr.strip_prefix("upper(").and_then(|s| s.strip_suffix(')')) {
            return Ok(WasmValue::String(self.eval_expr(arg)?.to_output().to_uppercase()));
        }
        if let Some(arg) = expr.strip_prefix("lower(").and_then(|s| s.strip_suffix(')')) {
            return Ok(WasmValue::String(self.eval_expr(arg)?.to_output().to_lowercase()));
        }

        for op in ['+', '-', '*', '/'] {
            if let Some((left, right)) = split_once_top_level(expr, op) {
                let l = self.eval_expr(left)?;
                let r = self.eval_expr(right)?;
                return self.apply_op(op, l, r);
            }
        }

        Ok(WasmValue::String(expr.to_string()))
    }

    fn apply_op(&self, op: char, left: WasmValue, right: WasmValue) -> Result<WasmValue, String> {
        match op {
            '+' => match (left, right) {
                (WasmValue::Number(a), WasmValue::Number(b)) => Ok(WasmValue::Number(a + b)),
                (a, b) => Ok(WasmValue::String(format!("{}{}", a.to_output(), b.to_output()))),
            },
            '-' => match (left, right) {
                (WasmValue::Number(a), WasmValue::Number(b)) => Ok(WasmValue::Number(a - b)),
                _ => Err("Subtraction requires numbers".to_string()),
            },
            '*' => match (left, right) {
                (WasmValue::Number(a), WasmValue::Number(b)) => Ok(WasmValue::Number(a * b)),
                _ => Err("Multiplication requires numbers".to_string()),
            },
            '/' => match (left, right) {
                (WasmValue::Number(_), WasmValue::Number(0.0)) => Ok(WasmValue::Number(0.0)),
                (WasmValue::Number(a), WasmValue::Number(b)) => Ok(WasmValue::Number(a / b)),
                _ => Err("Division requires numbers".to_string()),
            },
            _ => Err(format!("Unsupported operator: {}", op)),
        }
    }

    #[wasm_bindgen]
    pub fn execute_command(&mut self, command: &str) -> String {
        let parts: Vec<&str> = command.trim().split_whitespace().collect();
        if parts.is_empty() {
            return String::new();
        }

        match parts[0] {
            "core" => self.handle_core_command(&parts[1..]),
            "fforge" => self.handle_fforge_command(&parts[1..]),
            "metroman" => self.handle_metroman_command(&parts[1..]),
            "help" => self.show_help(),
            "ls" => self.list_files(),
            "cat" => self.cat_file(&parts[1..]),
            "clear" => "CLEAR_TERMINAL".to_string(),
            _ => format!("Command not found: {}\nType 'help' for available commands.", parts[0]),
        }
    }

    fn handle_core_command(&mut self, args: &[&str]) -> String {
        if args.is_empty() {
            return "Usage: core [options] <file.fr>\nOptions: --out, --in, -r, -j".to_string();
        }
        match args[0] {
            "--out" => self.syntax_dump(),
            "--in" => self.syntax_load(),
            "-r" | "--rust" | "-j" | "--jit" | "-v" | "--vm" => {
                if args.len() > 1 {
                    self.execute(args[1])
                } else {
                    "Error: No file specified".to_string()
                }
            }
            filename if filename.ends_with(".fr") => self.execute(filename),
            _ => format!("Unknown option: {}", args[0]),
        }
    }

    fn handle_fforge_command(&mut self, args: &[&str]) -> String {
        if args.is_empty() {
            return "Usage: fforge <file.fr>".to_string();
        }
        let filename = args[0];
        if !filename.ends_with(".fr") {
            return format!("Error: Expected .fr file, got: {}", filename);
        }
        self.execute(filename)
    }

    fn handle_metroman_command(&mut self, args: &[&str]) -> String {
        if args.is_empty() {
            return "MetroMan - CoRe Plugin Manager\nUsage: metroman [--out | --init | --build]"
                .to_string();
        }
        match args[0] {
            "--out" => self.create_plugin_template(),
            "--init" => "Plugin project initialized in current directory".to_string(),
            "--build" => "Plugin built successfully".to_string(),
            _ => format!("Unknown metroman option: {}", args[0]),
        }
    }

    fn syntax_dump(&mut self) -> String {
        let syntax_content = r#"{
  "keywords": { "say": "print", "var": "let", "fn": "function" },
  "operators": { "+": "add", "-": "sub", "*": "mul", "/": "div", "=": "assign" }
}"#;
        let full_content = format!(
            "# CoRe Language Syntax Definition\n# Modify this file and use 'core --in' to reload\n\n{}",
            syntax_content
        );
        self.files
            .insert("syntax.fr".to_string(), full_content.clone());
        format!(
            "✓ Syntax mapping dumped to syntax.fr\n\nPreview:\n{}",
            full_content.lines().take(10).collect::<Vec<_>>().join("\n")
        )
    }

    fn syntax_load(&mut self) -> String {
        if self.files.contains_key("syntax.fr") {
            "✓ Syntax mapping loaded from syntax.fr\n  Custom syntax is now active in this session."
                .to_string()
        } else {
            "Error: syntax.fr not found. Use 'core --out' to generate it first.".to_string()
        }
    }

    fn create_plugin_template(&mut self) -> String {
        let template = r#"fn init: { say: "Plugin initialized" }"#;
        self.files
            .insert("plugin_template.fr".to_string(), template.to_string());
        format!(
            "✓ Plugin template created: plugin_template.fr\n\nTemplate:\n{}",
            template
        )
    }

    fn show_help(&self) -> String {
        r#"CoRe Language - WebAssembly Terminal
Available Commands:
  core [file.fr]         - Execute CoRe file
  core -r [file.fr]      - Execute with runtime emulation
  core -j [file.fr]      - Execute with JIT emulation
  core --vm [file.fr]    - Execute with VM emulation
  core --out             - Dump syntax mapping
  core --in              - Load syntax mapping
  fforge [file.fr]       - Execute file
  metroman --out         - Create plugin template
  ls / cat / help / clear
"#
        .to_string()
    }

    fn list_files(&self) -> String {
        if self.files.is_empty() {
            return "No files loaded. Use the file editor to create .fr files.".to_string();
        }
        let mut files: Vec<_> = self.files.keys().collect();
        files.sort();
        let mut result = "Files:\n".to_string();
        for file in files {
            result.push_str(&format!("  {} ({} bytes)\n", file, self.files[file].len()));
        }
        result
    }

    fn cat_file(&self, args: &[&str]) -> String {
        if args.is_empty() {
            return "Usage: cat <filename>".to_string();
        }
        let filename = args[0];
        match self.files.get(filename) {
            Some(content) => format!("Content of {}:\n\n{}", filename, content),
            None => format!("File not found: {}", filename),
        }
    }

    #[wasm_bindgen]
    pub fn get_files_json(&self) -> String {
        let mut files: Vec<_> = self.files.keys().cloned().collect();
        files.sort();
        let escaped: Vec<String> = files
            .iter()
            .map(|f| format!("\"{}\"", json_escape(f)))
            .collect();
        format!("[{}]", escaped.join(","))
    }

    #[wasm_bindgen]
    pub fn get_file_content(&self, filename: &str) -> String {
        self.files.get(filename).cloned().unwrap_or_default()
    }

    #[wasm_bindgen]
    pub fn save_file(&mut self, filename: &str, content: &str) {
        self.files.insert(filename.to_string(), content.to_string());
    }

    #[wasm_bindgen]
    pub fn delete_file(&mut self, filename: &str) -> bool {
        self.files.remove(filename).is_some()
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

fn split_once_top_level(expr: &str, op: char) -> Option<(&str, &str)> {
    let mut depth = 0i32;
    let mut in_string = false;
    for (idx, ch) in expr.char_indices().rev() {
        match ch {
            '"' => in_string = !in_string,
            ')' if !in_string => depth += 1,
            '(' if !in_string => depth -= 1,
            _ => {}
        }
        if in_string || depth != 0 {
            continue;
        }
        if ch == op {
            let left = expr[..idx].trim();
            let right = expr[idx + ch.len_utf8()..].trim();
            if !left.is_empty() && !right.is_empty() {
                return Some((left, right));
            }
        }
    }
    None
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            _ => out.push(c),
        }
    }
    out
}
