import { App, TerraformStack } from 'cdktf';
import { Construct } from 'constructs';
import { DockerProvider } from '@cdktf/provider-docker/lib/provider';
import { Image } from '@cdktf/provider-docker/lib/image';
import { Container } from '@cdktf/provider-docker/lib/container';
import { Network } from '@cdktf/provider-docker/lib/network';
import * as path from 'path';

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

    // Build the app image from workspace root
    const workspaceRoot = path.resolve(__dirname, '../../..');
    const dockerFile = path.join(__dirname, 'Dockerfile')
    const appImage = new Image(this, 'app-image', {
      name: 'cdk-arch-app:latest',
      buildAttribute: {
        context: workspaceRoot,
        dockerfile: dockerFile
      }
    });

    const postgresImage = new Image(this, 'postgres-image', {
      name: 'postgres:16-alpine',
      keepLocally: true
    });

    // Postgres container for JsonStore
    new Container(this, 'postgres-container', {
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

    // JsonStore container
    new Container(this, 'jsonstore-container', {
      name: 'jsonstore',
      image: appImage.imageId,
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
      command: ['bun', 'run', 'src/docker/entrypoints/jsonstore-server.ts'],
      mustRun: true
    });

    // API container
    new Container(this, 'api-container', {
      name: 'hello-api',
      image: appImage.imageId,
      ports: [{
        internal: 3000,
        external: 3000
      }],
      env: [
        'PORT=3000',
        'JSONSTORE_HOST=jsonstore',
        'JSONSTORE_PORT=3001'
      ],
      networksAdvanced: [{
        name: appNetwork.name
      }],
      command: ['bun', 'run', 'src/docker/entrypoints/api-server.ts'],
      mustRun: true
    });
  }
}

export const synth_local_docker = () => {
  const app = new App();
  new HelloWorldStack(app, 'hello-world');
  app.synth();

  return app;
}