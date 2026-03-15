# CoRe Language

CoRe is a small, pragmatic programming language implemented in Rust. This repo includes multiple execution pathways (interpreter, VM, and ARM64 JIT), plus a browser-based demo.

## What’s Inside

- **Interpreter**: reliable, easy to debug
- **VM**: ARM64 virtual machine execution
- **JIT**: ARM64-only fast path (`fforge`)
- **Web Demo**: static site that runs in the browser and works on GitHub Pages

## Quick Start

Build the toolchain:

```bash
cargo build
```

Run with the interpreter:

```bash
./target/debug/forge --rust feature_showcase.fr
```

Run with the VM:

```bash
./target/debug/forge --vm feature_showcase.fr
./target/debug/forge -a feature_showcase.fr
```

Run with the JIT (ARM64 only):

```bash
./target/debug/fforge feature_showcase.fr
```

## Web Demo (Local)

Serve the repo root and open the page:

```bash
python3 -m http.server 8000
```

Then visit:

```
http://localhost:8000/
```

The page will try WASM first (if `pkg/` exists) and fall back to the JS engine.

## GitHub Pages

A GitHub Actions workflow is included to deploy the repo root to Pages. Ensure GitHub Pages is set to **GitHub Actions** in repo settings.

## Notes

- JIT only works on **aarch64**.
- VM/JIT output renders lists/maps differently than the interpreter.

## License

Add your license here.
