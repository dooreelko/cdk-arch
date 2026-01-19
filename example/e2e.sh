#!/bin/bash
set -exuo pipefail

cd "$(dirname "$0")"

echo "=== E2E Test ==="

echo "Deploying..."
npm run deploy

echo "Waiting for services to start..."
sleep 5

echo "Testing API..."
RESPONSE=$(curl -s http://localhost:3000/v1/api/hello/E2ETest)
echo "Response: $RESPONSE"

if [[ "$RESPONSE" == *"Hello, E2ETest"* ]]; then
  echo "API test passed"
else
  echo "API test failed"
  npm run destroy
  exit 1
fi

echo "Verifying data in Postgres..."
DATA=$(podman exec postgres psql -U postgres -d jsonstore -t -c "SELECT data FROM documents WHERE collection = 'greeted' ORDER BY created_at DESC LIMIT 1;")
echo "Stored data: $DATA"

if [[ "$DATA" == *"E2ETest"* ]]; then
  echo "Database test passed"
else
  echo "Database test failed"
  npm run destroy
  exit 1
fi

echo "Cleaning up..."
npm run destroy

echo "=== E2E Test Passed ==="
