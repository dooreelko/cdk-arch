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

echo "=== Cloudflare E2E Test ==="

echo "Deploying..."
npm run deploy

echo "Waiting for workers to be available..."
sleep 10

echo "Testing hello API..."
RESPONSE=$(curl -s "${API_BASE_URL}/v1/api/hello/E2ETest")
echo "Response: $RESPONSE"

if [[ "$RESPONSE" == *"Hello, E2ETest"* ]]; then
  echo "Hello API test passed"
else
  echo "Hello API test failed"
  npm run destroy
  exit 1
fi

echo "Testing hellos API..."
HELLOS=$(curl -s "${API_BASE_URL}/v1/api/hellos")
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

echo "=== Cloudflare E2E Test Passed ==="
