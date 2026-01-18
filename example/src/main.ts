import { App, TerraformStack } from 'cdktf';
import { Construct } from 'constructs';
import { DockerProvider } from '@cdktf/provider-docker/lib/provider';
import { Image } from '@cdktf/provider-docker/lib/image';
import { Container } from '@cdktf/provider-docker/lib/container';
import { Network } from '@cdktf/provider-docker/lib/network';
import { arch, api, jsonStore, helloFunction } from './architecture';
import { architectureBinding } from 'cdk-arch';

class HelloWorldStack extends TerraformStack {
  constructor(scope: Construct, id: string) {
    super(scope, id);

    new DockerProvider(this, 'docker', {
      host: `unix://${process.env.XDG_RUNTIME_DIR}/podman/podman.sock`
    });

    // Create network for service communication
    const appNetwork = new Network(this, 'app-network', {
      name: 'hello-world-network'
    });

    // Images
    const nodeImage = new Image(this, 'node-image', {
      name: 'node:20-alpine',
      keepLocally: true
    });

    const postgresImage = new Image(this, 'postgres-image', {
      name: 'postgres:16-alpine',
      keepLocally: true
    });

    const exampleDir = `${__dirname}/..`;

    // Postgres container for JsonStore
    const postgresContainer = new Container(this, 'postgres-container', {
      name: 'postgres',
      image: postgresImage.imageId,
      env: [
        'POSTGRES_USER=postgres',
        'POSTGRES_PASSWORD=postgres',
        'POSTGRES_DB=jsonstore'
      ],
      networksAdvanced: [{
        name: appNetwork.name
      }],
      healthcheck: {
        test: ['CMD-SHELL', 'pg_isready -U postgres'],
        interval: '5s',
        timeout: '5s',
        retries: 5
      },
      mustRun: true
    });

    // Bind JsonStore to its endpoint (will be resolved at runtime via Docker DNS)
    architectureBinding.bind(jsonStore, { host: 'jsonstore', port: 3001 });

    // JsonStore container
    const jsonStoreContainer = new Container(this, 'jsonstore-container', {
      name: 'jsonstore',
      image: nodeImage.imageId,
      env: [
        'PORT=3001',
        'POSTGRES_HOST=postgres',
        'POSTGRES_PORT=5432',
        'POSTGRES_DB=jsonstore',
        'POSTGRES_USER=postgres',
        'POSTGRES_PASSWORD=postgres'
      ],
      networksAdvanced: [{
        name: appNetwork.name
      }],
      volumes: [{
        hostPath: `${exampleDir}/server/dist`,
        containerPath: '/app/dist'
      }, {
        hostPath: `${exampleDir}/node_modules`,
        containerPath: '/app/node_modules'
      }],
      workingDir: '/app',
      command: ['node', 'dist/jsonstore-server.js'],
      mustRun: true
    });

    // Bind API to its endpoint
    architectureBinding.bind(api, { host: 'hello-api', port: 3000 });

    // API container
    const apiContainer = new Container(this, 'api-container', {
      name: 'hello-api',
      image: nodeImage.imageId,
      ports: [{
        internal: 3000,
        external: 3000
      }],
      env: [
        'PORT=3000',
        'JSONSTORE_URL=http://jsonstore:3001'
      ],
      networksAdvanced: [{
        name: appNetwork.name
      }],
      volumes: [{
        hostPath: `${exampleDir}/server/dist`,
        containerPath: '/app/dist'
      }, {
        hostPath: `${exampleDir}/node_modules`,
        containerPath: '/app/node_modules'
      }],
      workingDir: '/app',
      command: ['node', 'dist/api-server.js'],
      mustRun: true
    });

    // Output the architecture definition with bindings
    console.log('Deploying architecture:', arch.synth());
    console.log('JsonStore endpoint:', architectureBinding.getEndpoint(jsonStore));
    console.log('API endpoint:', architectureBinding.getEndpoint(api));
  }
}

const app = new App();
new HelloWorldStack(app, 'hello-world');
app.synth();
