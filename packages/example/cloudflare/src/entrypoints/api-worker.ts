import { architectureBinding } from '@arinoto/cdk-arch';
import { api, jsonStore, log } from 'architecture';
import { createWorkerHandler, serviceBindingHandler } from '../worker-adapter';

interface Env {
  JSONSTORE: { fetch: typeof fetch };
}

// Worker setup - env is passed per-request
let currentEnv: Env | null = null;

// Set up service binding overloads for jsonStore
architectureBinding.bind(jsonStore, {
  host: 'jsonstore',
  port: 0, // Not used for service bindings
  overloads: {
    store: serviceBindingHandler(
      () => currentEnv!.JSONSTORE,
      'POST /store/{collection}'
    ),
    get: serviceBindingHandler(
      () => currentEnv!.JSONSTORE,
      'GET /get/{collection}'
    )
  }
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
