FROM node:20-alpine

WORKDIR /app

# Copy package files first for better caching
COPY package*.json ./
COPY example/package*.json ./example/

# Install root dependencies
RUN npm install

# Copy source code
COPY tsconfig.json ./
COPY src ./src

# Build the library
RUN npm run build

# Install example dependencies (uses local cdk-arch)
WORKDIR /app/example
RUN npm install

# Copy example source
COPY example/tsconfig.json ./
COPY example/src ./src
COPY example/server ./server

# Build example and server
RUN npm run build

WORKDIR /app/example

# Default command (overridden by container)
CMD ["node", "server/dist/api-server.js"]
