import { architectureBinding, createHttpBindings } from '@arinoto/cdk-arch';
import { api, jsonStore, log } from 'architecture';
import { createWorkerHandler } from '../worker-adapter';

interface Env {
  JSONSTORE: { fetch: typeof fetch };
}

// Worker setup - env is passed per-request
let currentEnv: Env | null = null;

// Set up service binding client for jsonStore
const jsonStoreClient = createHttpBindings(
  { baseUrl: 'https://example.com' },
  jsonStore,
  ['store', 'get'],
  () => currentEnv!.JSONSTORE
);

architectureBinding.bind(jsonStore, {
  baseUrl: 'jsonstore',
  overloads: jsonStoreClient
});

// Create handler from the api container
const handleRequest = createWorkerHandler(api);

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    currentEnv = env;
    try {
      // log('api rq', {request});
      return await handleRequest(request);
    } finally {
      currentEnv = null;
    }
  }
};
