FROM oven/bun:1
WORKDIR /app
COPY package.json bun.lock* ./
COPY packages/cdk-arch/package.json packages/cdk-arch/
COPY packages/example/architecture/package.json packages/example/architecture/
COPY packages/example/local-docker/package.json packages/example/local-docker/
RUN bun install
COPY packages packages
RUN bun run --cwd packages/cdk-arch build && \
    bun run --cwd packages/example/architecture build && \
    bun run --cwd packages/example/local-docker build
CMD ["bun", "run", "packages/example/local-docker/src/entrypoints/api-server.ts"]
