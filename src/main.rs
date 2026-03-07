mod analyzer;
mod ast;
mod codegen;
mod debug_timer;
mod diagnostics;
mod ir;
mod jit;
mod lexer;
mod meta;
mod parser;
mod runtime;

#[cfg(not(target_arch = "wasm32"))]
use clap::Parser as ClapParser;
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::process::Command;

#[cfg(not(target_arch = "wasm32"))]
#[derive(ClapParser)]
#[command(name = "forge")]
#[command(about = "CoRe Language Compiler", long_about = None)]
#[cfg(not(target_arch = "wasm32"))]
struct Cli {
    /// Source file to compile
    file: Option<String>,

    /// Run using the Rust interpreter (direct execution)
    #[arg(short = 'r', long = "rust")]
    rust: bool,

    /// Force direct execution (interpreter) mode (legacy)
    #[arg(short = 'd', long = "direct")]
    direct: bool,

    /// Native execution mode (ARM64)
    #[arg(short, long)]
    native: bool,

    /// Run using the ARM64 VM (default)
    #[arg(short = 'v', long = "vm")]
    vm: bool,

    /// Assembly VM execution mode (legacy)
    #[arg(short = 'a', long)]
    asm: bool,

    /// Show detailed compiler progress logs
    #[arg(short, long)]
    info: bool,

    /// Build and link without running
    #[arg(short, long)]
    build: bool,

    /// Dump syntax mapping to syntax.fr
    #[arg(long)]
    out: bool,

    /// Load syntax mapping from syntax.fr and rebuild
    #[arg(long = "in")]
    in_syntax: bool,

    /// Install the compiler to the system path
    #[arg(long)]
    install: bool,

    /// Clean up generated files
    #[arg(long)]
    clean: bool,

    /// Run using the JIT pipeline
    #[arg(short = 'j', long = "jit")]
    jit: bool,

    /// Enable debug mode with timing information
    #[arg(short = 'D', long = "debug")]
    debug: bool,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExecMode {
    Direct,
    Native,
    Vm,
    Jit,
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_exec_mode(cli: &Cli) -> Result<ExecMode, String> {
    let wants_direct = cli.direct || cli.rust;
    let wants_vm = cli.asm || cli.vm;
    let wants_jit = cli.jit;

    if wants_jit && (cli.native || wants_vm || wants_direct) {
        return Err("Error: --jit cannot be combined with other execution modes".to_string());
    }
    if cli.native && wants_vm {
        return Err("Error: --native and --vm/--asm are mutually exclusive".to_string());
    }
    if cli.native && wants_direct {
        return Err("Error: --native cannot be combined with --rust/--direct".to_string());
    }
    if wants_vm && wants_direct {
        return Err("Error: --rust/--direct cannot be combined with --vm/--asm".to_string());
    }

    if wants_jit {
        Ok(ExecMode::Jit)
    } else if cli.native {
        Ok(ExecMode::Native)
    } else if wants_direct {
        Ok(ExecMode::Direct)
    } else {
        // Default execution mode: VM
        Ok(ExecMode::Vm)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let cli = Cli::parse();
    let verbose = cli.info;
    let debug = cli.debug;

    // Initialize debug timer
    let timer = debug_timer::DebugTimer::new(debug);

    let exec_mode = resolve_exec_mode(&cli).unwrap_or_else(|e| {
        eprintln!("{}", e);
        std::process::exit(2);
    });

    // Handle syntax dump/load (no source file required)
    if cli.out {
        let _phase = timer.phase("Syntax Dump");
        let mapping = meta::syntax_dump::SyntaxMapping::from_compiler();
        match mapping.dump_to_file("syntax.fr") {
            Ok(_) => println!("✓ Syntax mapping dumped to syntax.fr"),
            Err(e) => {
                eprintln!("✗ Error dumping syntax: {}", e);
                std::process::exit(1);
            }
        }
        _phase.finish();
        timer.print_total("syntax dump");
        return;
    }

    if cli.in_syntax {
        let _phase = timer.phase("Syntax Load");
        match meta::syntax_dump::SyntaxMapping::load_from_file("syntax.fr") {
            Ok(mapping) => match meta::syntax_load::rebuild_from_syntax(&mapping) {
                Ok(_) => println!("✓ Compiler rebuilt from syntax.fr"),
                Err(e) => {
                    eprintln!("✗ Error rebuilding: {}", e);
                    std::process::exit(1);
                }
            },
            Err(e) => {
                eprintln!("✗ Error loading syntax: {}", e);
                std::process::exit(1);
            }
        }
        _phase.finish();
        timer.print_total("syntax load");
        return;
    }

    // Handle install command
    if cli.install {
        let current_exe = std::env::current_exe().expect("Failed to get current executable path");
        let bin_dir = current_exe.parent().expect("Failed to get bin directory");

        // Try ~/.local/bin first (no sudo needed), fallback to /usr/local/bin
        let home_dir = std::env::var("HOME").ok();
        let install_dir = if let Some(home) = home_dir {
            let local_bin = std::path::PathBuf::from(home).join(".local/bin");
            // Create ~/.local/bin if it doesn't exist
            if !local_bin.exists() {
                if let Err(e) = fs::create_dir_all(&local_bin) {
                    eprintln!("⚠ Could not create ~/.local/bin: {}", e);
                    std::path::PathBuf::from("/usr/local/bin")
                } else {
                    local_bin
                }
            } else {
                local_bin
            }
        } else {
            std::path::PathBuf::from("/usr/local/bin")
        };

        // Create /usr/local/bin if it doesn't exist and we're using it
        if install_dir == std::path::PathBuf::from("/usr/local/bin") && !install_dir.exists() {
            if let Err(e) = fs::create_dir_all(&install_dir) {
                eprintln!("✗ Could not create /usr/local/bin: {}", e);
                eprintln!("  Try running with sudo or use ~/.local/bin instead");
                eprintln!("  (Add ~/.local/bin to your PATH in ~/.zshrc)");
                std::process::exit(1);
            }
        }

        println!(
            "→ Installing forge, core, fforge, forger, and metroman to {}...",
            install_dir.display()
        );

        let target_forge = install_dir.join("forge");
        let target_core = install_dir.join("core");
        let target_fforge = install_dir.join("fforge");
        let target_forger = install_dir.join("forger");
        let target_metroman = install_dir.join("metroman");

        // Install forge (skip self-copy; copying a file onto itself can corrupt it).
        let same_forge = fs::canonicalize(&current_exe)
            .ok()
            .zip(fs::canonicalize(&target_forge).ok())
            .map(|(a, b)| a == b)
            .unwrap_or(false);
        let forge_result = if same_forge {
            Ok(0)
        } else {
            fs::copy(&current_exe, &target_forge)
        };
        match forge_result {
            Ok(_) => println!(
                "✓ Successfully installed forge to {}{}",
                target_forge.display(),
                if same_forge {
                    " (already current install)"
                } else {
                    ""
                }
            ),
            Err(e) => {
                eprintln!("✗ Failed to install forge: {}", e);
                eprintln!("  Try running with sudo or add ~/.local/bin to your PATH");
                std::process::exit(1);
            }
        }

        // Install core, fforge, forger (if present)
        let extra_bins = [
            ("core", &target_core),
            ("fforge", &target_fforge),
            ("forger", &target_forger),
        ];
        for (name, target) in extra_bins {
            let bin_path = bin_dir.join(name);
            if bin_path.exists() {
                let same_bin = fs::canonicalize(&bin_path)
                    .ok()
                    .zip(fs::canonicalize(target).ok())
                    .map(|(a, b)| a == b)
                    .unwrap_or(false);
                let copy_result = if same_bin {
                    Ok(0)
                } else {
                    fs::copy(&bin_path, target)
                };
                match copy_result {
                    Ok(_) => println!("✓ Successfully installed {} to {}", name, target.display()),
                    Err(e) => {
                        eprintln!("✗ Failed to install {}: {}", name, e);
                        eprintln!("  Try running with sudo");
                    }
                }
            } else {
                eprintln!("⚠ Could not find {} binary at {:?}", name, bin_path);
                eprintln!("  Make sure to run 'cargo build --release' first.");
            }
        }

        // Install metroman
        let metroman_exe = bin_dir.join("metroman");
        if metroman_exe.exists() {
            let same_metroman = fs::canonicalize(&metroman_exe)
                .ok()
                .zip(fs::canonicalize(&target_metroman).ok())
                .map(|(a, b)| a == b)
                .unwrap_or(false);
            let metroman_copy = if same_metroman {
                Ok(0)
            } else {
                fs::copy(&metroman_exe, &target_metroman)
            };
            match metroman_copy {
                Ok(_) => println!(
                    "✓ Successfully installed metroman to {}{}",
                    target_metroman.display(),
                    if same_metroman {
                        " (already current install)"
                    } else {
                        ""
                    }
                ),
                Err(e) => {
                    eprintln!("✗ Failed to install metroman: {}", e);
                    eprintln!("  Try running with sudo");
                }
            }
        } else {
            eprintln!("⚠ Could not find metroman binary at {:?}", metroman_exe);
            eprintln!("  Make sure to run 'cargo build --release' first.");
        }

        // Print PATH instructions
        println!();
        println!("✓ Installation complete!");
        if install_dir.to_str().unwrap().contains(".local") {
            println!("  Add this to your ~/.zshrc to use the commands:");
            println!("    export PATH=\"$HOME/.local/bin:$PATH\"");
            println!("  Then run: source ~/.zshrc");
        }
        println!();
        println!("Available commands:");
        println!("  core main.fr        # VM (default)");
        println!("  core -r main.fr     # Rust interpreter");
        println!("  core -a main.fr     # Assembly VM");
        println!("  fforge main.fr      # JIT (in progress)");
        println!("  forge --native main.fr  # AOT compiler");

        return;
    }

    // Handle clean command
    if cli.clean {
        println!("→ Cleaning up generated files...");
        let files_to_clean = vec!["main.s", "main.o", "main"];
        for file in files_to_clean {
            if std::path::Path::new(file).exists() {
                match fs::remove_file(file) {
                    Ok(_) => println!("✓ Removed {}", file),
                    Err(e) => eprintln!("✗ Failed to remove {}: {}", file, e),
                }
            }
        }
        // Also clean up any .s or .o files matching *.s or *.o in current dir
        if let Ok(entries) = fs::read_dir(".") {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if ext == "s" || ext == "o" {
                        // Don't delete source files if they happen to have these extensions (unlikely for .s/.o but safe to check)
                        // Actually, .s is assembly, .o is object. Safe to delete.
                        // But let's be careful not to delete essential files if any.
                        // The user asked to delete "assembly and executable files like test.main.s, test_main"
                        // Let's just stick to the known generated ones or patterns.
                        // For now, just the specific ones is safer, or maybe files ending in .s/.o that match a source file name?
                        // Let's just clean the common ones.
                    }
                }
            }
        }
        println!("✓ Cleanup complete");
        return;
    }

    // Require a source file
    let source_file = match cli.file {
        Some(f) => f,
        None => {
            eprintln!("Error: No source file provided");
            eprintln!("Usage: forge [OPTIONS] <FILE>");
            eprintln!("       forge --out");
            eprintln!("       forge --in");
            eprintln!("       forge --install");
            eprintln!("       forge --clean");
            std::process::exit(1);
        }
    };

    // Read source code with timing
    let source = debug_time!(timer, "File Reading", {
        match fs::read_to_string(&source_file) {
            Ok(s) => {
                if debug {
                    println!("[DEBUG] Read {} bytes from {}", s.len(), source_file);
                }
                s
            },
            Err(e) => {
                eprintln!("✗ Error reading file '{}': {}", source_file, e);
                eprintln!("  Make sure the file exists and is readable");
                std::process::exit(1);
            }
        }
    });

    // Lexical analysis with timing
    let tokens = debug_time!(timer, "Lexical Analysis", {
        if verbose {
            println!("→ Lexing...");
        }
        let tokens: Result<Vec<_>, _> = lexer::Lexer::new(&source)
            .map(|(token, span)| match token {
                Ok(t) => Ok((t, span)),
                Err(e) => Err(e),
            })
            .collect();

        match tokens {
            Ok(t) => {
                if debug {
                    println!("[DEBUG] Produced {} tokens", t.len());
                }
                t
            },
            Err(e) => {
                eprintln!("✗ Lexer error: {}", e);
                eprintln!("  Check for invalid characters or syntax errors");
                std::process::exit(1);
            }
        }
    });

    // Parsing with timing
    let program = debug_time!(timer, "Parsing", {
        if verbose {
            println!("→ Parsing...");
        }
        let mut parser = parser::Parser::new(tokens);
        match parser.parse() {
            Ok(p) => {
                if debug {
                    println!("[DEBUG] Parsed {} top-level items", p.items.len());
                }
                p
            },
            Err(e) => {
                // Enhanced error reporting with file context
                if let Some(byte_pos) = e
                    .split("at byte ")
                    .nth(1)
                    .and_then(|s| s.parse::<usize>().ok())
                {
                    let mut line = 1;
                    let mut col = 1;
                    for (i, c) in source.char_indices() {
                        if i == byte_pos {
                            break;
                        }
                        if c == '\n' {
                            line += 1;
                            col = 1;
                        } else {
                            col += 1;
                        }
                    }

                    let mut diag =
                        diagnostics::Diagnostic::error(e.split(" at byte").next().unwrap_or(&e))
                            .at(line, col);

                    // Enhanced suggestions based on error content
                    if e.contains("Expected Colon") {
                        diag.suggestion = Some("Missing ':' after command or declaration".to_string());
                    } else if e.contains("Unexpected token: Some(Identifier") {
                        diag.suggestion = Some("Check if you forgot a keyword or operator".to_string());
                    } else if e.contains("Expected LBrace") {
                        diag.suggestion = Some("Missing opening '{' for block".to_string());
                    } else if e.contains("Expected RBrace") {
                        diag.suggestion = Some("Missing closing '}' for block".to_string());
                    }

                    diag.render(Some(&source));
                } else {
                    diagnostics::Diagnostic::error(&e).render(Some(&source));
                }
                std::process::exit(1);
            }
        }
    });

    // IR generation with timing
    let ir_program = debug_time!(timer, "IR Generation", {
        if verbose {
            println!("→ Generating IR...");
        }
        let mut ir_builder = ir::IrBuilder::new();
        match ir_builder.build(&program, Some(std::path::Path::new(&source_file))) {
            Ok(ir) => {
                if debug {
                    let func_instrs: usize =
                        ir.functions.values().map(|f| f.instructions.len()).sum();
                    let total_instrs = ir.global_code.len() + func_instrs;
                    println!("[DEBUG] Generated {} IR instructions", total_instrs);
                }
                ir
            },
            Err(e) => {
                eprintln!("✗ IR generation error: {}", e);
                eprintln!("  This usually indicates a semantic error in your code");
                std::process::exit(1);
            }
        }
    });

    // Static analysis with timing
    let _analyzer = debug_time!(timer, "Static Analysis", {
        if verbose {
            println!("→ Analyzing...");
        }
        let mut analyzer = analyzer::Analyzer::new();
        if let Err(errors) = analyzer.analyze(&ir_program) {
            eprintln!("✗ Analysis errors:");
            for error in errors {
                eprintln!("  - {}", error);
            }
            eprintln!("  Fix these errors before running your code");
            std::process::exit(1);
        }

        // Show warnings
        for warning in analyzer.get_warnings() {
            if verbose {
                println!("⚠ Warning: {}", warning);
            }
        }

        if debug {
            let warnings = analyzer.get_warnings();
            println!("[DEBUG] Analysis complete - {} warnings", warnings.len());
        }

        analyzer
    });

    // Execute based on mode
    match exec_mode {
        ExecMode::Native => {
            debug_time!(timer, "Native Compilation", {
                if verbose {
                    println!("→ Generating ARM64 assembly...");
                }
                let mut codegen = codegen::arm64::Arm64CodeGen::new();
                let _asm = match codegen.generate(&ir_program) {
                    Ok(a) => a,
                    Err(e) => {
                        eprintln!("✗ Code generation error: {}", e);
                        eprintln!("  This indicates an issue with ARM64 code generation");
                        std::process::exit(1);
                    }
                };

                let asm_file = source_file.replace(".fr", ".s");
                match codegen.write_to_file(&asm_file) {
                    Ok(_) => {
                        if verbose || debug {
                            println!("✓ Assembly written to {}", asm_file);
                        }
                    }
                    Err(e) => {
                        eprintln!("✗ Error writing assembly: {}", e);
                        std::process::exit(1);
                    }
                }

                if verbose {
                    println!("→ Assembling and linking...");
                }
                let output_file = source_file.replace(".fr", "");

                let status = Command::new("as")
                    .args(["-o", &format!("{}.o", output_file), &asm_file])
                    .status();
                if !status.map(|s| s.success()).unwrap_or(false) {
                    eprintln!("✗ Assembly failed");
                    eprintln!("  Make sure you have Xcode command line tools installed");
                    std::process::exit(1);
                }

                let default_sdk = "/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk";
                let sdk_path = if std::path::Path::new(default_sdk).exists() {
                    default_sdk.to_string()
                } else {
                    Command::new("xcrun")
                        .args(["--sdk", "macosx", "--show-sdk-path"])
                        .output()
                        .ok()
                        .and_then(|o| {
                            if o.status.success() {
                                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
                            } else {
                                None
                            }
                        })
                        .unwrap_or_else(|| default_sdk.to_string())
                };

                let link_result = Command::new("ld")
                    .args([
                        "-o",
                        &output_file,
                        &format!("{}.o", output_file),
                        "-lSystem",
                        "-syslibroot",
                        &sdk_path,
                        "-e",
                        "_main",
                    ])
                    .status();
                if !link_result.map(|s| s.success()).unwrap_or(false) {
                    eprintln!("✗ Linking failed");
                    eprintln!("  Check that the SDK path is correct: {}", sdk_path);
                    std::process::exit(1);
                }

                if verbose || debug {
                    println!("✓ Executable created: {}", output_file);
                }

                if !cli.build {
                    if verbose {
                        println!("→ Executing native binary...");
                    }
                    match Command::new(format!("./{}", output_file)).status() {
                        Ok(status) => {
                            if debug {
                                println!("[DEBUG] Native execution completed with status: {}", status);
                            }
                            if !status.success() {
                                if let Some(code) = status.code() {
                                    std::process::exit(code);
                                }
                                std::process::exit(1);
                            }
                        }
                        Err(e) => {
                            eprintln!("✗ Execution failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
            });
        }
        ExecMode::Jit => {
            debug_time!(timer, "JIT Compilation & Execution", {
                if verbose {
                    println!("→ Executing via JIT...");
                }

                let mut context = jit::context::JitContext::new();
                let mut jit = jit::compiler::JitCompiler::new(&mut context);

                let mut func_names: Vec<_> = ir_program.functions.keys().cloned().collect();
                func_names.sort_by(|a, b| b.cmp(a));

                if debug {
                    println!("[DEBUG] Compiling {} functions", func_names.len());
                }

                for name in func_names {
                    if let Some(func) = ir_program.functions.get(&name) {
                        match jit.compile_function(func) {
                            Ok(_) => {
                                if debug {
                                    println!("[DEBUG] Compiled function: {}", name);
                                }
                            }
                            Err(e) => {
                                eprintln!("✗ JIT Compilation Error (Function {}): {}", name, e);
                                eprintln!("  Check the function implementation for errors");
                                std::process::exit(1);
                            }
                        }
                    }
                }

                if debug {
                    println!(
                        "[DEBUG] Executing global code with {} instructions",
                        ir_program.global_code.len()
                    );
                }

                match jit.execute_global(&ir_program.global_code) {
                    Ok(res) => {
                        if verbose {
                            println!("✓ JIT execution completed. Result: {}", res);
                        }
                        println!("{}", render_encoded(res));
                    }
                    Err(e) => {
                        eprintln!("✗ JIT Runtime Error: {}", e);
                        eprintln!("  This usually indicates a runtime issue in your code");
                        std::process::exit(1);
                    }
                }
            });
        }
        ExecMode::Vm => {
            debug_time!(timer, "VM Assembly Generation & Execution", {
                if verbose {
                    println!("→ Executing via ARM64 VM...");
                }

                if verbose {
                    println!("→ Generating ARM64 assembly for VM...");
                }
                let mut codegen = codegen::arm64::Arm64CodeGen::new();
                let _asm = match codegen.generate(&ir_program) {
                    Ok(a) => a,
                    Err(e) => {
                        eprintln!("✗ Code generation error: {}", e);
                        eprintln!("  Failed to generate ARM64 assembly for VM");
                        std::process::exit(1);
                    }
                };

                let asm_file = source_file.replace(".fr", ".s");
                if let Err(e) = codegen.write_to_file(&asm_file) {
                    eprintln!("✗ Error writing assembly: {}", e);
                    std::process::exit(1);
                }

                if verbose || debug {
                    println!("✓ Assembly written to {}", asm_file);
                }

                let mut vm_cmd = find_or_build_arm64vm().unwrap_or_else(|e| {
                    eprintln!("✗ {}", e);
                    eprintln!(
                        "  Ensure arm64vm is built (vm/target/release/arm64vm) or installed in PATH."
                    );
                    std::process::exit(1);
                });

                vm_cmd.arg(&asm_file);

                if debug {
                    println!("[DEBUG] Executing VM command: {:?}", vm_cmd);
                }

                match vm_cmd.status() {
                    Ok(status) => {
                        if debug {
                            println!("[DEBUG] VM execution completed with status: {}", status);
                        }
                        if !status.success() {
                            eprintln!("✗ VM execution failed with status: {}", status);
                            if let Some(code) = status.code() {
                                std::process::exit(code);
                            }
                            std::process::exit(1);
                        }
                    }
                    Err(e) => {
                        eprintln!("✗ Failed to execute VM: {}", e);
                        eprintln!("  Make sure the ARM64 VM is properly installed and accessible");
                        std::process::exit(1);
                    }
                }
            });
        }
        ExecMode::Direct => {
            debug_time!(timer, "Direct Execution (Interpreter)", {
                if verbose {
                    println!("→ Executing via Interpreter...");
                }

                if debug {
                    println!(
                        "[DEBUG] Starting direct execution with {} global instructions",
                        ir_program.global_code.len()
                    );
                }

                let mut executor = codegen::direct::DirectExecutor::new();
                match executor.execute(&ir_program) {
                    Ok(_) => {
                        if verbose {
                            println!("✓ Interpreter execution completed");
                        }
                        if debug {
                            println!("[DEBUG] Direct execution finished successfully");
                        }
                    }
                    Err(e) => {
                        eprintln!("✗ Interpreter execution failed: {}", e);
                        eprintln!("  This usually indicates a runtime error in your code");
                        std::process::exit(1);
                    }
                }
            });
        }
    }

    // Print total execution time if debug mode is enabled
    timer.print_total("execution");
}

#[cfg(not(target_arch = "wasm32"))]
fn render_encoded(val: u64) -> String {
    if val == 0 {
        return "null".to_string();
    }
    if (val & 1) == 1 {
        return ((val as i64) >> 1).to_string();
    }
    format!("0x{:x}", val)
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod cli_tests {
    use super::*;

    #[test]
    fn default_is_vm() {
        let cli = Cli::parse_from(["forge", "main.fr"]);
        assert_eq!(resolve_exec_mode(&cli).unwrap(), ExecMode::Vm);
    }

    #[test]
    fn vm_flags_select_vm() {
        let cli = Cli::parse_from(["forge", "-v", "main.fr"]);
        assert_eq!(resolve_exec_mode(&cli).unwrap(), ExecMode::Vm);

        let cli = Cli::parse_from(["forge", "--vm", "main.fr"]);
        assert_eq!(resolve_exec_mode(&cli).unwrap(), ExecMode::Vm);

        let cli = Cli::parse_from(["forge", "-a", "main.fr"]);
        assert_eq!(resolve_exec_mode(&cli).unwrap(), ExecMode::Vm);

        let cli = Cli::parse_from(["forge", "--asm", "main.fr"]);
        assert_eq!(resolve_exec_mode(&cli).unwrap(), ExecMode::Vm);
    }

    #[test]
    fn rust_flags_select_direct() {
        let cli = Cli::parse_from(["forge", "-r", "main.fr"]);
        assert_eq!(resolve_exec_mode(&cli).unwrap(), ExecMode::Direct);

        let cli = Cli::parse_from(["forge", "--rust", "main.fr"]);
        assert_eq!(resolve_exec_mode(&cli).unwrap(), ExecMode::Direct);

        let cli = Cli::parse_from(["forge", "-d", "main.fr"]);
        assert_eq!(resolve_exec_mode(&cli).unwrap(), ExecMode::Direct);
    }

    #[test]
    fn native_is_exclusive() {
        let cli = Cli::parse_from(["forge", "--native", "main.fr"]);
        assert_eq!(resolve_exec_mode(&cli).unwrap(), ExecMode::Native);

        let cli = Cli::parse_from(["forge", "--native", "--vm", "main.fr"]);
        assert!(resolve_exec_mode(&cli).is_err());

        let cli = Cli::parse_from(["forge", "--native", "--rust", "main.fr"]);
        assert!(resolve_exec_mode(&cli).is_err());
    }

    #[test]
    fn direct_and_vm_are_exclusive() {
        let cli = Cli::parse_from(["forge", "--rust", "--vm", "main.fr"]);
        assert!(resolve_exec_mode(&cli).is_err());
    }

    #[test]
    fn jit_flag_selects_jit() {
        let cli = Cli::parse_from(["forge", "--jit", "main.fr"]);
        assert_eq!(resolve_exec_mode(&cli).unwrap(), ExecMode::Jit);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn find_or_build_arm64vm() -> Result<Command, String> {
    let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_dir = current_exe
        .parent()
        .ok_or("Failed to resolve forge executable directory")?;

    let candidates = [
        exe_dir.join("arm64vm"),
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join("vm/target/release/arm64vm"),
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join("vm/target/debug/arm64vm"),
    ];

    for path in candidates.iter() {
        if path.exists() {
            return Ok(Command::new(path));
        }
    }

    // If we're in the repo and vm/ exists, try building it.
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let vm_manifest = cwd.join("vm/Cargo.toml");
    if vm_manifest.exists() {
        let status = Command::new("cargo")
            .current_dir(cwd.join("vm"))
            .args(["build", "--release"])
            .status()
            .map_err(|e| format!("Failed to run cargo to build arm64vm: {}", e))?;

        if status.success() {
            let vm_bin = cwd.join("vm/target/release/arm64vm");
            if vm_bin.exists() {
                return Ok(Command::new(vm_bin));
            }
        }
        return Err("arm64vm build failed".to_string());
    }

    // Final fallback: PATH
    Ok(Command::new("arm64vm"))
}

// WebAssembly main function (does nothing since we use exported functions)
#[cfg(target_arch = "wasm32")]
fn main() {
    // WebAssembly builds use the exported functions in wasm.rs
    // This main function is just to satisfy the compiler
}
