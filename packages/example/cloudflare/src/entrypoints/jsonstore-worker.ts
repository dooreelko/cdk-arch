import { architectureBinding } from 'cdk-arch';
import { jsonStore } from 'architecture';
import { createWorkerHandler } from '../worker-adapter';

const log = (what: string, ...args: any[]) => console.log({what, extra: args.map(a => JSON.stringify(a))});

interface Env {
  JSONSTORE_KV: KVNamespace;
}

// Worker setup - env is passed per-request
let currentEnv: Env | null = null;

// KV-based storage handlers
const kvStore = async (collection: string, document: any): Promise<{ success: boolean }> => {
  log('store', collection, document);
  const kv = currentEnv!.JSONSTORE_KV;
  const id = crypto.randomUUID();
  const key = `${collection}:${id}`;
  await kv.put(key, JSON.stringify(document));
  return { success: true };
};

const kvGet = async (collection: string): Promise<any[]> => {
  log('get all', collection);
  const kv = currentEnv!.JSONSTORE_KV;
  const list = await kv.list({ prefix: `${collection}:` });
  log('get all keys', list);
  const documents = await Promise.all(
    list.keys.map(async (key) => {
      const value = await kv.get(key.name);
      return value ? JSON.parse(value) : null;
    })
  );
  log('get all results', documents);
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
      log('got req', request, env);
      return await handleRequest(request);
    } finally {
      currentEnv = null;
    }
  }
};
