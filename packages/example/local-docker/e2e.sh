#!/bin/bash
set -exuo pipefail

cd "$(dirname "$0")"

cleanup() {
  echo "Cleaning up..."
  LOGFILE=$(mktemp)
  npm run destroy > "$LOGFILE" 2>&1 || ( echo "Error destroying" && tail "$LOGFILE" && echo "See full log in $LOGFILE")
}

fail() {
  cleanup
  exit 1
}

systemctl --user start podman.socket

echo "=== E2E Test ==="

echo "Deploying..."
(cd ../.. && npm run clean && npm run build)

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

cleanup

echo "=== E2E Test Passed ==="
