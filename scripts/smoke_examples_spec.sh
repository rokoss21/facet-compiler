#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

BIN="${1:-./target/release/facet-fct}"
if [[ ! -x "$BIN" ]]; then
  echo "[smoke] binary not found/executable: $BIN"
  echo "[smoke] building release binary..."
  cargo build -q --release --bin facet-fct
  BIN="./target/release/facet-fct"
fi

echo "[smoke] using binary: $BIN"

BUILD_FILES=(
  examples/spec/01_minimal.facet
  examples/spec/02_vars_types_pipelines.facet
  examples/spec/03_input_runtime.facet
  examples/spec/04_when_gating.facet
  examples/spec/05_imports_merge.facet
  examples/spec/06_interfaces_policy.facet
  examples/spec/07_policy_conditions.facet
  examples/spec/08_test_suite.facet
  examples/spec/09_multimodal_content.facet
  examples/spec/10_layout_budget.facet
  examples/spec/11_pure_mode_expected_f803.facet
  examples/spec/12_exec_mode_expected_f454.facet
)

echo "[smoke] build checks"
for f in "${BUILD_FILES[@]}"; do
  echo "  - build $f"
  "$BIN" build --input "$f" >/dev/null
 done

echo "[smoke] run checks (success expected)"
"$BIN" run --input examples/spec/01_minimal.facet --format json >/dev/null
"$BIN" run --input examples/spec/02_vars_types_pipelines.facet --format json >/dev/null
"$BIN" run --input examples/spec/03_input_runtime.facet --runtime-input examples/spec/03_input_runtime.input.json --format json >/dev/null
"$BIN" run --input examples/spec/04_when_gating.facet --runtime-input examples/spec/04_when_gating.input.json --format json >/dev/null
"$BIN" run --input examples/spec/05_imports_merge.facet --format json >/dev/null
"$BIN" run --input examples/spec/06_interfaces_policy.facet --format json >/dev/null
"$BIN" run --input examples/spec/07_policy_conditions.facet --runtime-input examples/spec/07_policy_conditions.input.json --format json >/dev/null
"$BIN" run --input examples/spec/09_multimodal_content.facet --format json >/dev/null
"$BIN" run --input examples/spec/10_layout_budget.facet --format json >/dev/null

echo "[smoke] inspect check"
mkdir -p /tmp/facet-spec-inspect
"$BIN" inspect --input examples/spec/02_vars_types_pipelines.facet --ast /tmp/facet-spec-inspect/ast.json --dag /tmp/facet-spec-inspect/dag.json --layout /tmp/facet-spec-inspect/layout.json --policy /tmp/facet-spec-inspect/policy.json >/dev/null

echo "[smoke] test check"
"$BIN" test --input examples/spec/08_test_suite.facet --output summary >/dev/null

echo "[smoke] expected-failure checks"
set +e
PURE_OUT="$($BIN run --input examples/spec/11_pure_mode_expected_f803.facet --pure --format json 2>&1)"
PURE_RC=$?
EXEC_OUT="$($BIN run --input examples/spec/12_exec_mode_expected_f454.facet --exec --format json 2>&1)"
EXEC_RC=$?
set -e

if [[ $PURE_RC -eq 0 ]] || [[ "$PURE_OUT" != *"F803"* ]]; then
  echo "[smoke] expected F803 in pure mode example, got rc=$PURE_RC"
  echo "$PURE_OUT"
  exit 1
fi

if [[ $EXEC_RC -eq 0 ]] || [[ "$EXEC_OUT" != *"F454"* ]]; then
  echo "[smoke] expected F454 in exec deny example, got rc=$EXEC_RC"
  echo "$EXEC_OUT"
  exit 1
fi

echo "[smoke] all spec example checks passed"
