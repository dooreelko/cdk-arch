#!/bin/bash
set -exuo pipefail

cd "$(dirname "$0")"

systemctl --user start podman.socket

echo "=== E2E Test ==="

echo "Deploying..."
npm run deploy

echo "Waiting for services to start..."
sleep 5

echo "Testing hello API..."
RESPONSE=$(curl -s http://localhost:3000/v1/api/hello/E2ETest)
echo "Response: $RESPONSE"

if [[ "$RESPONSE" == *"Hello, E2ETest"* ]]; then
  echo "Hello API test passed"
else
  echo "Hello API test failed"
  npm run destroy
  exit 1
fi

echo "Testing hellos API..."
HELLOS=$(curl -s http://localhost:3000/v1/api/hellos)
echo "Hellos response: $HELLOS"

if [[ "$HELLOS" == *"E2ETest"* ]]; then
  echo "Hellos API test passed"
else
  echo "Hellos API test failed"
  npm run destroy
  exit 1
fi

echo "Cleaning up..."
npm run destroy

echo "=== E2E Test Passed ==="
