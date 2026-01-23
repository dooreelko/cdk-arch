# CDK Architecture

A set of CDK primitives that allow defining conceptual event-driven solution architectures. This allows to separate design and 
implementation (later done with CDKTF) of which there could be multiple - local, different clouds, etc; generate architecture diagrams 
and, most importantly this allows an easier refactoring of architectures and validation of implementations.

Something like this for a hello world that later can be implemented as an AWS ApiGateway with a lambda function and Dynamo, 
or an k8s microservice running in azure in Azure Database for PostgreSQL

```typescript

export class ApiContainer extends Construct {...}
export class Function extends Construct {...}
export class JsonStore extends ApiContainer {...}

const arch = new Architecture();

const jsonStore = new JsonStore();

const api = new ApiContainer(arch, {
	'/v1/api/hello/{name}': new Function(arch, (name: string) => {
		jsonStore.store('greeted', {when: Date.now(), name});
		return `Hello, ${name}!`
	})
});

arch.synth()

```
After that we have C4's system or container definitions (which we can generate as diagrams) and can create a CDKTF definition for its deployment 
that will aditionally validate that all architectural components have corresponding deployment elements.
