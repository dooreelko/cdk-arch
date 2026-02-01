import express from 'express';
import { architectureBinding, createHttpBindings } from '@arinoto/cdk-arch';
import { DockerApiServer } from '../docker-api-server';
import { api, jsonStore } from 'architecture';

const PORT = parseInt(process.env.PORT || '3000');

// Bind api locally
architectureBinding.bind(api, { baseUrl: `hello-api:${PORT}` });

// Bind jsonStore with HTTP client for remote calls
const jsonStoreEndpoint = {
  baseUrl: `http://${process.env.JSONSTORE_HOST || 'jsonstore'}:${parseInt(process.env.JSONSTORE_PORT || '3001')}`
};

const jsonStoreClient = createHttpBindings(jsonStoreEndpoint, jsonStore, ['store', 'get']);

architectureBinding.bind(jsonStore, {
  ...jsonStoreEndpoint,
  overloads: jsonStoreClient
});

// Create and start server
const server = new DockerApiServer(api, { binding: architectureBinding });

server.start(PORT);
function mov(epress: typeof express) {
  throw new Error('Function not implemented.');
}

