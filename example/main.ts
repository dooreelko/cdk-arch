import { App, TerraformStack } from 'cdktf';
import { Construct } from 'constructs';
import { DockerProvider } from '@cdktf/provider-docker/lib/provider';
import { Image } from '@cdktf/provider-docker/lib/image';
import { Container } from '@cdktf/provider-docker/lib/container';
import { arch, api } from './architecture';

class HelloWorldStack extends TerraformStack {
  constructor(scope: Construct, id: string) {
    super(scope, id);

    new DockerProvider(this, 'docker', {});

    // Use a simple Node.js image for the API
    const nodeImage = new Image(this, 'node-image', {
      name: 'node:20-alpine',
      keepLocally: true
    });

    // Create a container for the API
    const apiContainer = new Container(this, 'api-container', {
      name: 'hello-api',
      image: nodeImage.imageId,
      ports: [{
        internal: 3000,
        external: 3000
      }],
      env: [
        `ROUTES=${JSON.stringify(api.listRoutes())}`
      ],
      command: [
        'node', '-e',
        `
        const http = require('http');
        const routes = JSON.parse(process.env.ROUTES);

        const server = http.createServer((req, res) => {
          const url = new URL(req.url, 'http://localhost');

          // Simple route matching for /v1/api/hello/{name}
          const match = url.pathname.match(/^\\/v1\\/api\\/hello\\/(.+)$/);
          if (match) {
            const name = decodeURIComponent(match[1]);
            res.writeHead(200, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify({ message: 'Hello, ' + name + '!' }));
          } else {
            res.writeHead(404, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify({ error: 'Not found' }));
          }
        });

        server.listen(3000, () => {
          console.log('API running on http://localhost:3000');
          console.log('Routes:', routes);
        });
        `
      ],
      mustRun: true
    });

    // Output the architecture definition
    console.log('Deploying architecture:', arch.synth());
  }
}

const app = new App();
new HelloWorldStack(app, 'hello-world');
app.synth();
