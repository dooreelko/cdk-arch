import { Architecture, ApiContainer, Function, FunctionRuntimeContextMarker } from '@arinoto/cdk-arch';
import { JsonStore } from './json-store';
import { trace } from './log';

export interface Greeting {
  when: number;
  name: string;
}

export const arch = new Architecture('hello-world');

export const jsonStore = new JsonStore<Greeting>(arch, 'greeted-store');

const helloFunction = new Function(arch, 'hello-handler', async (name: string) => {
  trace('helloing', {name});
  const res = await jsonStore.store('greeted', { when: Date.now(), name });
  trace('stored', {res});
  return `Hello, ${name}!`;
});

const hellosFunction = new Function(arch, 'hellos-handler', () => {
  const res = jsonStore.get('greeted');
  trace('get all', {res});
  return res;
});

const extractContext = <TCast>(that: any) => {
  if (!that) {
    throw new Error('Context is null or underfined');
  }

  if (!that.runtimeContext) {
    throw new Error('Context is missing runtimeContext marker');
  }

  return that as TCast;
};

type DemoRequest = {
  request: {
    url: string
  }
};

/*
If you define your handler as a function instead of an arrow one, the runtime can
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
