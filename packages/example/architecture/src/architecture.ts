import { Architecture, ApiContainer, Function, FunctionRuntimeContextMarker } from '@arinoto/cdk-arch';
import { JsonStore } from './json-store';
import { DemoRequest, extractContext } from './runtime-context';

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
Define the handler as a `function ()` instead of an arrow one, then the runtime can
pass an arbitrary execution context. In this example a subset of express Request is expected

See Function.invokeWithRuntimeContext and example/local-docker/DockerApiServer.setupRoute
*/
const requestContextFunction = new Function<[], string, FunctionRuntimeContextMarker & DemoRequest>(arch, 'context-handler', function() {
  const ctx = extractContext<DemoRequest>(this);
  return Promise.resolve(ctx.request.url);
});

export const api = new ApiContainer(arch, 'api', {
  hello: { path: 'GET /v1/api/hello/{name}', handler: helloFunction },
  hellos: { path: 'GET /v1/api/hellos', handler: hellosFunction },
  ctx: { path: 'GET /v1/api/ctx', handler: requestContextFunction }
});

console.log('Architecture definition:');
console.log(JSON.stringify(arch.synth(), null, 2));
