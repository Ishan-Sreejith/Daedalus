#!/bin/bash


set -e

PROJECT_DIR="/Users/ishan/IdeaProjects/CoRe Main/CoRe Backup V1.0 copy"
cd "$PROJECT_DIR"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

PASSED=0
FAILED=0

run_test() {
    local test_name="$1"
    local test_code="$2"
    local expected_output="$3"

    echo -e "\n${BLUE}Testing: $test_name${NC}"

    cat > /tmp/test_fn.fr << EOF
$test_code
EOF

    actual_output=$(./target/debug/fforge /tmp/test_fn.fr 2>&1 | grep "✓ Result:" | sed 's/.*Result: //' || echo "ERROR")

    if [[ "$actual_output" == "$expected_output" ]]; then
        echo -e "${GREEN}✓ PASS${NC} - Output: $actual_output"
        ((PASSED++))
    else
        echo -e "${RED}✗ FAIL${NC}"
        echo "  Expected: $expected_output"
        echo "  Got: $actual_output"
        ((FAILED++))
    fi
}

echo -e "\n${BLUE}Building project...${NC}"
cargo build 2>&1 | grep -E "(Compiling|Finished)" || echo "Build in progress..."

echo -e "\n${BLUE}════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}FUNCTION RETURN VALUE TESTS${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════════${NC}"

run_test "Simple constant return" \
'fn five {
    return 5
}
var x: five
say: x' \
'5'

run_test "Different constant return" \
'fn get_ten {
    return 10
}
var x: get_ten
say: x' \
'10'

run_test "Arithmetic return" \
'fn add: a, b {
    return a + b
}
var x: add: 3, 4
say: x' \
'7'

run_test "Subtraction" \
'fn subtract: a, b {
    return a - b
}
var x: subtract: 10, 3
say: x' \
'7'

run_test "Multiplication" \
'fn multiply: a, b {
    return a * b
}
var x: multiply: 4, 5
say: x' \
'20'

run_test "Three parameters" \
'fn add_three: a, b, c {
    return a + b + c
}
var x: add_three: 1, 2, 3
say: x' \
'6'

run_test "Local variable" \
'fn compute {
    var x: 5
    return x
}
var y: compute
say: y' \
'5'

run_test "Local variable computation" \
'fn double: x {
    var result: x + x
    return result
}
var y: double: 5
say: y' \
'10'

run_test "Nested computation" \
'fn complex: x, y {
    var a: x + y
    var b: a + x
    return b
}
var z: complex: 2, 3
say: z' \
'7'

run_test "Zero return" \
'fn zero {
    return 0
}
var x: zero
say: x' \
'0'

run_test "Large number" \
'fn big_number {
    return 1000
}
var x: big_number
say: x' \
'1000'

run_test "Subtraction chain" \
'fn calc: a, b {
    return a - b + 10
}
var x: calc: 15, 5
say: x' \
'20'

run_test "Global code" \
'var x: 5
var y: 3
var z: x + y
say: z' \
'8'

run_test "Function then global use" \
'fn add: a, b {
    return a + b
}
var result: add: 5, 5
var final: result + 10
say: final' \
'20'

run_test "Multiple function calls" \
'fn add: a, b {
    return a + b
}
var x: add: 2, 3
var y: add: x, 5
say: y' \
'10'

echo -e "\n${BLUE}════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}TEST SUMMARY${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════════${NC}"
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"
echo -e "Total:  $((PASSED + FAILED))"

if [ $FAILED -eq 0 ]; then
    echo -e "\n${GREEN}✓ All tests passed!${NC}"
    exit 0
else
    echo -e "\n${RED}✗ Some tests failed${NC}"
    exit 1
fi

