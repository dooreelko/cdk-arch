FROM oven/bun:1-alpine

WORKDIR /app

# Copy root package and install
COPY package.json bun.lock* ./
RUN bun install

# Copy and build library source
COPY tsconfig.json ./
COPY src ./src
RUN bun run build

# Link the library globally
RUN bun link

# Copy example package
COPY example/package.json ./example/

# Install example dependencies and link cdk-arch
WORKDIR /app/example
RUN bun install && bun link cdk-arch

# Copy example source (includes docker servers)
COPY example/tsconfig.json ./
COPY example/src ./src

WORKDIR /app/example

# Default command
CMD ["bun", "run", "src/docker/api-server.ts"]
