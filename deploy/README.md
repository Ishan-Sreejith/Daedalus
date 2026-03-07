# CoRe Language - WebAssembly Edition

🚀 Experience the CoRe programming language directly in your browser with WebAssembly!

## ✨ Features

- **Real-time Code Execution**: Run CoRe programs instantly in your browser
- **WebAssembly Powered**: High-performance execution via Rust-compiled WebAssembly
- **25+ Built-in Functions**: Comprehensive standard library
- **Enhanced Collections**: Native support for lists, maps, and complex data structures
- **Browser Ready**: No installation required - just open and code!

## 🎯 Quick Start

1. **Open the webpage**: Simply open `index.html` in a modern web browser
2. **Write some code**: Use the editor or try the provided examples
3. **Run your program**: Click "▶️ Run Code" or press Ctrl+Enter (Cmd+Enter on Mac)
4. **See the output**: Results appear in the output panel below

## 📝 Example Programs

### Hello World
```core
var greeting: "Hello from CoRe Language!"
var user: "WebAssembly User" 
var message: greeting + " Welcome, " + user + "!"

say: message
say: "🚀 Running in your browser via WebAssembly!"
```

### Working with Collections
```core
var person: {
    "name": "Alice",
    "age": 30,
    "skills": ["Rust", "WebAssembly", "JavaScript"]
}

say: "Name: " + person["name"]
say: "Skills: " + str(person["skills"])
```

### Math and Control Flow
```core
var numbers: [1, 2, 3, 4, 5]
var sum: 0
var i: 0

while i < len(numbers) {
    sum: sum + numbers[i]
    i: i + 1
}

say: "Sum: " + str(sum)
```

## 🔧 Core Language Features

- **Variables**: `var name: value`
- **Data Types**: Numbers, strings, booleans, lists, maps
- **Output**: `say: expression`
- **Control Flow**: `if/else`, `while` loops, `for` loops
- **Functions**: Built-in functions like `len()`, `str()`, `num()`, etc.
- **Collections**: Array-style lists and key-value maps
- **String Operations**: Concatenation with `+`
- **Math Operations**: `+`, `-`, `*`, `/`, `%`
- **Comparisons**: `<`, `>`, `<=`, `>=`, `==`, `!=`

## 🌐 GitHub Pages Deployment

To deploy this on GitHub Pages:

1. **Create a new repository** on GitHub
2. **Upload these files**:
   - `index.html` (the web interface)
   - `pkg/` folder (contains the compiled WebAssembly)
3. **Enable GitHub Pages**:
   - Go to repository Settings → Pages
   - Select "Deploy from a branch"
   - Choose "main" branch and "/ (root)" folder
   - Save

Your CoRe Language playground will be available at:
`https://yourusername.github.io/your-repo-name/`

## 📦 What's Included

```
├── index.html              # Web interface
├── pkg/                    # WebAssembly package
│   ├── forge.js           # JavaScript bindings
│   ├── forge_bg.wasm      # Compiled WebAssembly binary
│   ├── forge.d.ts         # TypeScript definitions
│   └── package.json       # Package metadata
└── README.md              # This file
```

## 🏗️ Building from Source

If you want to rebuild the WebAssembly module:

1. **Install Rust and wasm-pack**:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   cargo install wasm-pack
   ```

2. **Add WebAssembly target**:
   ```bash
   rustup target add wasm32-unknown-unknown
   ```

3. **Build the WebAssembly package**:
   ```bash
   wasm-pack build --target web --features wasm --no-default-features
   ```

4. **Serve locally** (optional):
   ```bash
   # Using Python
   python -m http.server 8000
   
   # Using Node.js
   npx http-server
   
   # Using any other local server
   ```

## 🎮 Keyboard Shortcuts

- **Ctrl+Enter** (Cmd+Enter on Mac): Run code
- **Tab** in editor: Insert indentation
- **Ctrl+A**: Select all code

## 🔍 Browser Compatibility

This works in all modern browsers that support WebAssembly:
- ✅ Chrome 57+
- ✅ Firefox 52+
- ✅ Safari 11+
- ✅ Edge 16+

## 🐛 Troubleshooting

**If the page doesn't load:**
- Make sure you're serving the files over HTTP(S), not opening `index.html` directly
- Check the browser console for error messages
- Ensure your browser supports WebAssembly

**If code doesn't run:**
- Check for syntax errors in your CoRe code
- Look at the output panel for error messages
- Try one of the example programs first

## 🚀 Advanced Usage

The WebAssembly module exposes these methods:
- `execute(code)`: Run CoRe code and return output
- `get_version()`: Get the engine version
- `get_features()`: List available features
- `reset()`: Reset the engine state

## 🤝 Contributing

Found a bug or want to add a feature? 
- Report issues on GitHub
- Submit pull requests
- Share your CoRe programs with the community!

## 📄 License

This project is licensed under the same terms as the main CoRe language project.

---

**Happy coding with CoRe! 🔥**
