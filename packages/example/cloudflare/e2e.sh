#!/bin/bash
set -exuo pipefail

cd "$(dirname "$0")"

if [ -z "${CLOUDFLARE_ACCOUNT_ID:-}" ]; then
  echo "Error: CLOUDFLARE_ACCOUNT_ID environment variable is required"
  exit 1
fi

if [ -z "${CLOUDFLARE_SUBDOMAIN:-}" ]; then
  echo "Error: CLOUDFLARE_SUBDOMAIN environment variable is required"
  exit 1
fi

API_BASE_URL="https://hello-world-api.${CLOUDFLARE_SUBDOMAIN}.workers.dev"
# LOGFILE="/tmp/e2e-terraform.log"

cleanup() {
  echo "Cleaning up..."
  npm run destroy # >> "$LOGFILE" 2>&1 || true
}

fail() {
  # echo "=== Last 50 lines of terraform output ==="
  # tail -50 "$LOGFILE"

  # cleanup
  exit 1
}

echo "=== Cloudflare E2E Test ==="

echo "Deploying..."
npm run deploy # > "$LOGFILE" 2>&1 || fail

echo "Waiting for workers to be available..."
sleep 10

echo "Testing hello API..."
RESPONSE=$(curl -s "${API_BASE_URL}/v1/api/hello/E2ETest")
echo "Response: $RESPONSE"

if [[ "$RESPONSE" == *"Hello, E2ETest"* ]]; then
  echo "Hello API test passed"
else
  echo "Hello API test failed"
  fail
fi

echo "Testing hellos API..."
HELLOS=$(curl -s "${API_BASE_URL}/v1/api/hellos")
echo "Hellos response: $HELLOS"

if [[ "$HELLOS" == *"E2ETest"* ]]; then
  echo "Hellos API test passed"
else
  echo "Hellos API test failed"
  fail
fi

cleanup
# rm -f "$LOGFILE"

echo "=== Cloudflare E2E Test Passed ==="
