import express from 'express';
import { architectureBinding } from 'cdk-arch';
import { DockerApiServer } from './docker-api-server';
import { api, jsonStore } from '../architecture';

const PORT = parseInt(process.env.PORT || '3000');

// Bind components to their endpoints
architectureBinding.bind(api, { host: 'hello-api', port: PORT });
architectureBinding.bindFromEnv(jsonStore, 'JSONSTORE');

// Enable remote mode for jsonStore - calls become HTTP requests
architectureBinding.enableRemote(jsonStore);

// Create and start server
const server = new DockerApiServer(api, { binding: architectureBinding });
server.start(express, PORT);
