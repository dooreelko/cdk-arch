import express from 'express';
import { architectureBinding } from 'cdk-arch';
import { DockerApiServer } from '../docker-api-server';
import { httpHandler } from '../http-handler';
import { api, jsonStore } from 'architecture';

const PORT = parseInt(process.env.PORT || '3000');

// Bind api locally
architectureBinding.bind(api, { host: 'hello-api', port: PORT });

// Bind jsonStore with HTTP overloads for remote calls
const jsonStoreEndpoint = {
  host: process.env.JSONSTORE_HOST || 'jsonstore',
  port: parseInt(process.env.JSONSTORE_PORT || '3001')
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
