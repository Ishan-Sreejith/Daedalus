#!/usr/bin/env bash



echo "=== ACTION 0.1: Verify Build ==="
cd "/Users/ishan/IdeaProjects/CoRe Main/CoRe Backup V1.0 copy"
cargo test --lib jit::trampoline --release 2>&1 | tail -5

echo ""
echo "=== ACTION 0.2: Test Other Binaries ==="
echo 'say: 42' > /tmp/test_minimal.fr

echo "Testing forger (Rust interpreter):"
timeout 3 ./target/release/forger /tmp/test_minimal.fr

echo ""
echo "Testing forge (VM):"
timeout 3 ./target/release/forge /tmp/test_minimal.fr

echo ""
echo "=== ACTION 0.3: Rebuild fforge with Debug Output ==="
cargo build --release --bin fforge 2>&1 | grep -E "(Compiling|Finished)"

echo ""
echo "=== ACTION 0.4: Run fforge with Timeout ==="
timeout 5 ./target/release/fforge /tmp/test_minimal.fr 2>&1








mkdir -p /tmp/jit_tests

cat > /tmp/jit_tests/test_const.fr << 'EOF'
say: 42
EOF

cat > /tmp/jit_tests/test_add.fr << 'EOF'
var x: 10
var y: 20
say: x + y
EOF

cat > /tmp/jit_tests/test_if.fr << 'EOF'
var x: 10
if x > 5 {
    say: "yes"
}
EOF

cat > /tmp/jit_tests/test_loop.fr << 'EOF'
var i: 0
while i < 3 {
    say: i
    var i: i + 1
}
EOF

for test_file in /tmp/jit_tests/test_*.fr; do
    echo "Testing $(basename $test_file)..."
    timeout 3 ./target/release/fforge "$test_file" 2>&1 | head -10
    echo ""
done






cat > /tmp/jit_tests/test_float.fr << 'EOF'
var pi: 3.14159
var e: 2.71828
var result: pi + e
say: result
EOF

echo "Testing floats..."
./target/release/fforge /tmp/jit_tests/test_float.fr





cat > /tmp/jit_tests/test_string.fr << 'EOF'
var greeting: "Hello"
var name: "World"
say: greeting + " " + name
EOF

echo "Testing strings..."
./target/release/fforge /tmp/jit_tests/test_string.fr





cat > /tmp/jit_tests/test_array.fr << 'EOF'
var list: [10, 20, 30]
say: list[0]
say: list[1]
say: list[2]
EOF

echo "Testing arrays..."
./target/release/fforge /tmp/jit_tests/test_array.fr
























echo "Action plan generated. Start with Phase 0 (Diagnosis) to identify the hang."

