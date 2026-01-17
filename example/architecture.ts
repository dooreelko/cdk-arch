import { Architecture, ApiContainer, Function, JsonStore } from 'cdk-arch';

const arch = new Architecture('hello-world');

const jsonStore = new JsonStore(arch, 'greeted-store');

const helloFunction = new Function(arch, 'hello-handler', (name: string) => {
  jsonStore.store('greeted', { when: Date.now(), name });
  return `Hello, ${name}!`;
});

const api = new ApiContainer(arch, 'api', {
  '/v1/api/hello/{name}': helloFunction
});

console.log('Architecture definition:');
console.log(JSON.stringify(arch.synth(), null, 2));

export { arch, api, jsonStore, helloFunction };
