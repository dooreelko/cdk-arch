import { WorkerRouter, WorkerFunction } from '../worker-runtime';

interface Env {
  JSONSTORE_KV: KVNamespace;
}

// Worker setup - env is passed per-request
let currentEnv: Env | null = null;

// KV-based storage handlers
const storeFunction = new WorkerFunction(async (collection: string, document: any): Promise<{ success: boolean }> => {
  const kv = currentEnv!.JSONSTORE_KV;
  const id = crypto.randomUUID();
  const key = `${collection}:${id}`;
  await kv.put(key, JSON.stringify(document));
  return { success: true };
});

const getFunction = new WorkerFunction(async (collection: string): Promise<any[]> => {
  const kv = currentEnv!.JSONSTORE_KV;
  const list = await kv.list({ prefix: `${collection}:` });
  const documents = await Promise.all(
    list.keys.map(async (key) => {
      const value = await kv.get(key.name);
      return value ? JSON.parse(value) : null;
    })
  );
  return documents.filter(Boolean);
});

// Set up router
const router = new WorkerRouter();
router.addRoute('POST /store/{collection}', (...args) => storeFunction.invoke(...args));
router.addRoute('GET /get/{collection}', (...args) => getFunction.invoke(...args));

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
