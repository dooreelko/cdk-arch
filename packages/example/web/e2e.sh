#!/bin/bash
set -exuo pipefail

cd "$(dirname "$0")"

cleanup() {
  echo "Cleaning up..."

  LOGFILE=$(mktemp)
  cd ../local-docker
  npm run destroy > "$LOGFILE" 2>&1 || ( echo "Error destroying" && tail "$LOGFILE" && echo "See full log in $LOGFILE")
}

echo "=== Web E2E Test ==="

# Deploy local-docker
(
  echo "Deploying local-docker..."
  cd ../local-docker
  npm run deploy
)

echo "Waiting for services to start..."
sleep 5

# Wait for API to be ready
echo "Waiting for API to be ready..."
for i in {1..30}; do
  if curl -s http://localhost:3000/v1/api/hellos > /dev/null 2>&1; then
    echo "API is ready"
    break
  fi
  echo "Waiting for API... ($i/30)"
  sleep 1
done

# Run the web tests with real API
cd ../web
echo "Running web tests with real API..."
OVERRIDE_BASE_URL=http://localhost:3000 npm run test

echo "=== Web E2E Test Complete ==="

cleanup