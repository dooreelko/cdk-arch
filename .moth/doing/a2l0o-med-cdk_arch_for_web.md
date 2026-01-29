# top level 
cdk-arch architectures should be used for service binding by all parties involved

## TODO

- create a simple web site using vite and react under example/web. a text input, a submit button and a list of hellos underneath
- cdk-arch has a transient dependency on node's crypto via constructs (https://github.com/aws/constructs/blob/10.x/src/private/uniqueid.ts), use vite-plugin-node-polyfills 
- move httpHandler from local-docker to a cdk-arch under src/http-handler.ts
- web site should then import architecture and use architectureBinding.bind using the httpHandler
- create an e2e script that will start local-docker and run a vitest test suite against it making sure several hellos are submitted and shown

