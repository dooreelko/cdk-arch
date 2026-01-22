import { Architecture, ApiContainer, Function } from 'cdk-arch';
import { JsonStore } from './json-store';

const log = (what: string, ...args: any[]) => console.log({what, extra: args.map(a => JSON.stringify(a))});

const arch = new Architecture('hello-world');

const jsonStore = new JsonStore(arch, 'greeted-store');

const helloFunction = new Function(arch, 'hello-handler', (name: string) => {
  log('helloing', name);
  const res = jsonStore.store('greeted', { when: Date.now(), name });
  log('stored', res);
  return `Hello, ${name}!`;
});

const hellosFunction = new Function(arch, 'hellos-handler', () => {
  const res = jsonStore.get('greeted');
  log('get all', res);
  return res;
});

const api = new ApiContainer(arch, 'api', {
  '/v1/api/hello/{name}': helloFunction,
  'GET /v1/api/hellos': hellosFunction
});

console.log('Architecture definition:');
console.log(JSON.stringify(arch.synth(), null, 2));

export { arch, api, jsonStore, helloFunction, hellosFunction };
