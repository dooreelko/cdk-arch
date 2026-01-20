import { WorkerRouter, WorkerFunction, serviceBindingHandler } from '../worker-runtime';

interface Env {
  JSONSTORE: { fetch: typeof fetch };
}

// Worker setup - env is passed per-request
let currentEnv: Env | null = null;

// JsonStore functions (call via service binding)
const storeFunction = new WorkerFunction(() => { throw new Error('Not implemented'); });
const getFunction = new WorkerFunction(() => { throw new Error('Not implemented'); });

// Apply service binding overloads
storeFunction.overload(serviceBindingHandler(
  () => currentEnv!.JSONSTORE,
  'POST /store/{collection}'
));
getFunction.overload(serviceBindingHandler(
  () => currentEnv!.JSONSTORE,
  'GET /get/{collection}'
));

// Hello function - stores greeting and returns message
const helloFunction = new WorkerFunction(async (name: string) => {
  await storeFunction.invoke('greeted', { when: Date.now(), name });
  return `Hello, ${name}!`;
});

// Set up router
const router = new WorkerRouter();
router.addRoute('GET /v1/api/hello/{name}', (...args) => helloFunction.invoke(...args));

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    currentEnv = env;
    try {
      return await router.handle(request);
    } finally {
      currentEnv = null;
    }
  }
};
