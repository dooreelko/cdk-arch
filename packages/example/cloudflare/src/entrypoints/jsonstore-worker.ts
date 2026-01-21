import { architectureBinding } from 'cdk-arch';
import { jsonStore } from 'architecture';
import { createWorkerHandler } from '../worker-adapter';

interface Env {
  JSONSTORE_KV: KVNamespace;
}

// Worker setup - env is passed per-request
let currentEnv: Env | null = null;

// KV-based storage handlers
const kvStore = async (collection: string, document: any): Promise<{ success: boolean }> => {
  const kv = currentEnv!.JSONSTORE_KV;
  const id = crypto.randomUUID();
  const key = `${collection}:${id}`;
  await kv.put(key, JSON.stringify(document));
  return { success: true };
};

const kvGet = async (collection: string): Promise<any[]> => {
  const kv = currentEnv!.JSONSTORE_KV;
  const list = await kv.list({ prefix: `${collection}:` });
  const documents = await Promise.all(
    list.keys.map(async (key) => {
      const value = await kv.get(key.name);
      return value ? JSON.parse(value) : null;
    })
  );
  return documents.filter(Boolean);
};

// Bind jsonStore with KV overloads
architectureBinding.bind(jsonStore, {
  host: 'jsonstore',
  port: 0,
  overloads: {
    storeFunction: kvStore,
    getFunction: kvGet
  }
});

// Create handler from the jsonStore container
const handleRequest = createWorkerHandler(jsonStore);

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    currentEnv = env;
    try {
      return await handleRequest(request);
    } finally {
      currentEnv = null;
    }
  }
};
