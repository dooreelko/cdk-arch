#!/bin/bash
set -euo pipefail

if [ -n "${DEBUG:-}" ]; then
  set -x
fi

cd "$(dirname "$0")"

cleanup() {
  echo "Cleaning up..."
  LOGFILE=$(mktemp)
  npm run destroy > "$LOGFILE" 2>&1 || echo "Destroy step failed, continuing"
}

fail() {
  echo "JSON Store Container logs ================="
  podman logs jsonstore

  echo "API Container logs ================="
  podman logs hello-api

  cleanup
  exit 1
}

systemctl --user start podman.socket

echo "=== E2E Test ==="

echo "Deploying..."
(cd ../.. && npm run clean && npm run build)

echo "Testing c43 drawio..."
DRAWIO=$(../../../target/release/c43 --drawio container ..)
[[ "$DRAWIO" == "<mxfile"* ]] || { echo "c43 --drawio produced no drawio"; exit 1; }

echo "Testing clarc (stdin to stdout)..."
CLARC=$(../../../target/release/clarc --azure < ../../clarc/tests/cases/azure-phase2/input.json)
[[ "$CLARC" == "<mxfile"* ]] || { echo "clarc produced no drawio"; exit 1; }

npm run deploy || fail

echo "Waiting for services to start..."
sleep 5

echo "Testing hello API..."
RESPONSE=$(curl -s http://localhost:3000/v1/api/hello/E2ETest)
echo "Response: $RESPONSE"

if [[ "$RESPONSE" == *"Hello, E2ETest"* ]]; then
  echo "Hello API test passed"
else
  echo "Hello API test failed"
  fail
fi

echo "Testing hellos API..."
HELLOS=$(curl -s http://localhost:3000/v1/api/hellos)
echo "Hellos response: $HELLOS"

if [[ "$HELLOS" == *"E2ETest"* ]]; then
  echo "Hellos API test passed"
else
  echo "Hellos API test failed"
  fail
fi

echo "Testing ctx API..."
CTX=$(curl -s http://localhost:3000/v1/api/ctx)
echo "Ctx response: $CTX"

if [[ "$CTX" == *"/v1/api/ctx"* ]]; then
  echo "ctx API test passed"
else
  echo "ctx API test failed"
  fail
fi

cleanup

echo "=== E2E Test Passed ==="
