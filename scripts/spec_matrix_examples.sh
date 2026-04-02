#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

BIN="${1:-./target/release/facet-fct}"
if [[ ! -x "$BIN" ]]; then
  echo "[matrix] binary not found/executable: $BIN"
  echo "[matrix] building release binary..."
  cargo build -q --release --bin facet-fct
  BIN="./target/release/facet-fct"
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "[matrix] jq is required for JSON assertions"
  exit 1
fi

echo "[matrix] using binary: $BIN"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

run_ok_json() {
  local out="$1"
  shift
  local raw="$TMP_DIR/stdout.raw.json"
  RUST_LOG=error "$BIN" run "$@" --format json >"$raw" 2>"$TMP_DIR/stderr.log"
  # Keep only the canonical JSON line in case host logging leaks into stdout.
  sed -n '/^{/,$p' "$raw" >"$out"
}

run_expect_fail() {
  local expected_code="$1"
  shift
  set +e
  RUST_LOG=error "$BIN" run "$@" --format json >"$TMP_DIR/stdout.fail.json" 2>"$TMP_DIR/stderr.fail.log"
  local rc=$?
  set -e
  if [[ $rc -eq 0 ]]; then
    echo "[matrix] expected failure ($expected_code), got success: $*"
    cat "$TMP_DIR/stdout.fail.json"
    exit 1
  fi
  if ! grep -q "$expected_code" "$TMP_DIR/stderr.fail.log"; then
    echo "[matrix] expected error code $expected_code, got:"
    cat "$TMP_DIR/stderr.fail.log"
    exit 1
  fi
}

assert_jq() {
  local expr="$1"
  local file="$2"
  if ! jq -e "$expr" "$file" >/dev/null; then
    echo "[matrix] jq assertion failed"
    echo "  expr: $expr"
    echo "  file: $file"
    cat "$file"
    exit 1
  fi
}

echo "[matrix] 03_input_runtime: provided values"
run_ok_json "$TMP_DIR/03_provided.json" --input examples/spec/03_input_runtime.facet --runtime-input examples/spec/03_input_runtime.input.json
assert_jq '.messages[] | select(.role=="user") | .content == "explain facet imports"' "$TMP_DIR/03_provided.json"

echo "[matrix] 03_input_runtime: defaults"
echo '{}' >"$TMP_DIR/03_default.input.json"
run_ok_json "$TMP_DIR/03_default.json" --input examples/spec/03_input_runtime.facet --runtime-input "$TMP_DIR/03_default.input.json"
assert_jq '.messages[] | select(.role=="user") | .content == "what is facet"' "$TMP_DIR/03_default.json"

echo "[matrix] 03_input_runtime: invalid type -> F453"
cat >"$TMP_DIR/03_bad.input.json" <<'JSON'
{"query":"ok","limit":"bad"}
JSON
run_expect_fail F453 --input examples/spec/03_input_runtime.facet --runtime-input "$TMP_DIR/03_bad.input.json"

echo "[matrix] 04_when_gating matrix"
for show_system in true false; do
  for show_assistant in true false; do
    in_file="$TMP_DIR/04_${show_system}_${show_assistant}.json"
    out_file="$TMP_DIR/04_${show_system}_${show_assistant}.out.json"
    cat >"$in_file" <<JSON
{"show_system":$show_system,"show_assistant":$show_assistant}
JSON
    run_ok_json "$out_file" --input examples/spec/04_when_gating.facet --runtime-input "$in_file"

    assert_jq '[.messages[] | select(.role=="user")] | length == 1' "$out_file"
    if [[ "$show_system" == "true" ]]; then
      assert_jq '[.messages[] | select(.role=="system")] | length == 1' "$out_file"
    else
      assert_jq '[.messages[] | select(.role=="system")] | length == 0' "$out_file"
    fi
    if [[ "$show_assistant" == "true" ]]; then
      assert_jq '[.messages[] | select(.role=="assistant")] | length == 1' "$out_file"
    else
      assert_jq '[.messages[] | select(.role=="assistant")] | length == 0' "$out_file"
    fi
  done
done

echo "[matrix] 05_imports_merge in exec/pure"
run_ok_json "$TMP_DIR/05_exec.json" --input examples/spec/05_imports_merge.facet --exec
assert_jq '.messages[] | select(.role=="user") | .content == "override value"' "$TMP_DIR/05_exec.json"
run_ok_json "$TMP_DIR/05_pure.json" --input examples/spec/05_imports_merge.facet --pure
assert_jq '.messages[] | select(.role=="user") | .content == "override value"' "$TMP_DIR/05_pure.json"

echo "[matrix] 06_interfaces_policy: tool exposed"
run_ok_json "$TMP_DIR/06.json" --input examples/spec/06_interfaces_policy.facet
assert_jq '.tools | length == 1' "$TMP_DIR/06.json"
assert_jq '.tools[0].function.name == "get_current"' "$TMP_DIR/06.json"

echo "[matrix] 07_policy_conditions matrix"
for allow_tools in true false; do
  for deny_assistant in true false; do
    in_file="$TMP_DIR/07_${allow_tools}_${deny_assistant}.json"
    out_file="$TMP_DIR/07_${allow_tools}_${deny_assistant}.out.json"
    cat >"$in_file" <<JSON
{"allow_tools":$allow_tools,"deny_assistant":$deny_assistant}
JSON
    run_ok_json "$out_file" --input examples/spec/07_policy_conditions.facet --runtime-input "$in_file"

    if [[ "$deny_assistant" == "true" ]]; then
      assert_jq '[.messages[] | select(.role=="assistant")] | length == 0' "$out_file"
    else
      assert_jq '[.messages[] | select(.role=="assistant")] | length == 1' "$out_file"
    fi

    if [[ "$allow_tools" == "true" && "$deny_assistant" == "false" ]]; then
      assert_jq '.tools | length == 1' "$out_file"
    else
      assert_jq '.tools | length == 0' "$out_file"
    fi
  done
done

echo "[matrix] 08_test_suite in exec passes"
"$BIN" test --input examples/spec/08_test_suite.facet --output summary --exec >"$TMP_DIR/08_test_exec.txt" 2>"$TMP_DIR/08_test_exec.err"
if ! grep -q 'PASSED' "$TMP_DIR/08_test_exec.txt"; then
  echo "[matrix] expected PASSED for exec test"
  cat "$TMP_DIR/08_test_exec.txt"
  cat "$TMP_DIR/08_test_exec.err"
  exit 1
fi

echo "[matrix] 08_test_suite in pure fails with F801"
set +e
"$BIN" test --input examples/spec/08_test_suite.facet --output summary --pure >"$TMP_DIR/08_test_pure.txt" 2>"$TMP_DIR/08_test_pure.err"
rc=$?
set -e
if [[ $rc -eq 0 ]]; then
  echo "[matrix] expected pure-mode test failure"
  cat "$TMP_DIR/08_test_pure.txt"
  exit 1
fi
if ! grep -q 'F801' "$TMP_DIR/08_test_pure.txt" && ! grep -q 'F801' "$TMP_DIR/08_test_pure.err"; then
  echo "[matrix] expected F801 in pure-mode test failure"
  cat "$TMP_DIR/08_test_pure.txt"
  cat "$TMP_DIR/08_test_pure.err"
  exit 1
fi

echo "[matrix] 09_multimodal_content canonical checks"
run_ok_json "$TMP_DIR/09.json" --input examples/spec/09_multimodal_content.facet
assert_jq '.messages[0].content | type == "array"' "$TMP_DIR/09.json"
assert_jq '.messages[0].content[1].type == "image"' "$TMP_DIR/09.json"
assert_jq '.messages[1].content[1].type == "audio"' "$TMP_DIR/09.json"

echo "[matrix] 10_layout_budget metadata checks"
run_ok_json "$TMP_DIR/10.json" --input examples/spec/10_layout_budget.facet
assert_jq '.metadata.budget_units == 120' "$TMP_DIR/10.json"
assert_jq '.messages[] | select(.role=="user") | .content == "Critical user section"' "$TMP_DIR/10.json"

echo "[matrix] expected failures 11/12"
run_expect_fail F803 --input examples/spec/11_pure_mode_expected_f803.facet --pure
run_expect_fail F454 --input examples/spec/12_exec_mode_expected_f454.facet --exec

echo "[matrix] all matrix checks passed"
