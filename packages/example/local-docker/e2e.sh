#!/bin/bash
set -exuo pipefail

cd "$(dirname "$0")"

LOGFILE="/tmp/e2e-terraform.log"

cleanup() {
  echo "Cleaning up..."
  npm run destroy >> "$LOGFILE" 2>&1 || true
}

fail() {
  echo "=== Last 50 lines of terraform output ==="
  tail -50 "$LOGFILE"
  cleanup
  exit 1
}

systemctl --user start podman.socket

echo "=== E2E Test ==="

echo "Deploying..."
npm run deploy > "$LOGFILE" 2>&1 || fail

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

cleanup
rm -f "$LOGFILE"

echo "=== E2E Test Passed ==="
