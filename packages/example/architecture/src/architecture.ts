import { Architecture, ApiContainer, Function } from 'cdk-arch';
import { JsonStore } from './json-store';

const trace = (what: string, args: any) => {}; //console.log({message: what, level: 'trace', extra: args});
const log = (what: string, args: any) => console.log({message: what, level: 'info', extra: args});
const err = (what: string, args: any) => console.log({message: what, level: 'error', extra: args});

const arch = new Architecture('hello-world');

const jsonStore = new JsonStore(arch, 'greeted-store');

const helloFunction = new Function(arch, 'hello-handler', (name: string) => {
  trace('helloing', {name});
  const res = jsonStore.store('greeted', { when: Date.now(), name });
  trace('stored', {res});
  return `Hello, ${name}!`;
});

const hellosFunction = new Function(arch, 'hellos-handler', () => {
  const res = jsonStore.get('greeted');
  trace('get all', {res});
  return res;
});

const api = new ApiContainer(arch, 'api', {
  '/v1/api/hello/{name}': helloFunction,
  'GET /v1/api/hellos': hellosFunction
});

console.log('Architecture definition:');
console.log(JSON.stringify(arch.synth(), null, 2));

export { arch, api, jsonStore, helloFunction, hellosFunction, trace, log, err };
