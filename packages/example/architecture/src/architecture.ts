import { Architecture, ApiContainer, Function, getCurrentContext } from '@arinoto/cdk-arch';
import { JsonStore } from './json-store';
import { DemoRequest } from './runtime-context';

export interface Greeting {
  when: number;
  name: string;
}

export const arch = new Architecture('hello-world');

export const jsonStore = new JsonStore<Greeting>(arch, 'greeted-store');

const helloFunction = new Function(arch, 'hello-handler', async (name: string) => {
  const res = await jsonStore.store('greeted', { when: Date.now(), name });
  return `Hello, ${name}!`;
});

const hellosFunction = new Function(arch, 'hellos-handler', () => {
  const res = jsonStore.get('greeted');
  return res;
});

/*
Gets runtime context using AsyncLocalStorage-based getCurrentContext.
See example/local-docker/DockerApiServer.setupRoute for how context is set via runWithContext
*/
const requestContextFunction = new Function<[], string>(arch, 'context-handler', async () => {
  const ctx = getCurrentContext() as DemoRequest | undefined;
  if (!ctx) {
    throw new Error('No runtime context available');
  }
  return ctx.request.url;
});

export const api = new ApiContainer(arch, 'api', {
  hello: { path: 'GET /v1/api/hello/{name}', handler: helloFunction },
  hellos: { path: 'GET /v1/api/hellos', handler: hellosFunction },
  ctx: { path: 'GET /v1/api/ctx', handler: requestContextFunction }
});

console.log('Architecture definition:');
console.log(JSON.stringify(arch.synth(), null, 2));
