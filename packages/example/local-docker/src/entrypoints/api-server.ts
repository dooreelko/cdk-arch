import express from 'express';
import { architectureBinding, httpHandler } from '@arinoto/cdk-arch';
import { DockerApiServer } from '../docker-api-server';
import { api, jsonStore } from 'architecture';

const PORT = parseInt(process.env.PORT || '3000');

// Bind api locally
architectureBinding.bind(api, { baseUrl: `hello-api:${PORT}` });

// Bind jsonStore with HTTP overloads for remote calls
const jsonStoreEndpoint = {
  baseUrl: `http://${process.env.JSONSTORE_HOST || 'jsonstore'}:${parseInt(process.env.JSONSTORE_PORT || '3001')}`
};

architectureBinding.bind(jsonStore, {
  ...jsonStoreEndpoint,
  overloads: {
    store: httpHandler(jsonStoreEndpoint, jsonStore, 'store'),
    get: httpHandler(jsonStoreEndpoint, jsonStore, 'get')
  }
});

// Create and start server
const server = new DockerApiServer(api, { binding: architectureBinding });
server.start(express, PORT);
