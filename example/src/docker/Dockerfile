FROM docker.io/oven/bun:1-alpine

WORKDIR /app/cdk-arch

# Copy everything needed
COPY package.json bun.lock* ./
COPY tsconfig.json ./
COPY src ./src
# Install root dependencies
RUN bun install && bun link --production
RUN bun run build

WORKDIR /app/example
COPY example/package.json ./
COPY example/tsconfig.json ./
COPY example/src ./src

# Install example dependencies (file: reference resolves to parent)
RUN bun install --production 
RUN bun link --production cdk-arch

CMD ["echo", "I UP"]
