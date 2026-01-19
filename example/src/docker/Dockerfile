FROM oven/bun:1-alpine

WORKDIR /app

# Copy everything needed
COPY package.json bun.lock* ./
COPY tsconfig.json ./
COPY src ./src
COPY example/package.json ./example/
COPY example/tsconfig.json ./example/
COPY example/src ./example/src

# Install root dependencies
RUN bun install --production

# Install example dependencies (file: reference resolves to parent)
WORKDIR /app/example
RUN bun install --production

WORKDIR /app/example

CMD ["bun", "run", "src/docker/entrypoints/api-server.ts"]
