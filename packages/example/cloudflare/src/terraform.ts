import { App, TerraformStack, TerraformOutput } from 'cdktf';
import { Construct } from 'constructs';
import { CloudflareProvider } from '@cdktf/provider-cloudflare/lib/provider';
import { WorkersKvNamespace } from '@cdktf/provider-cloudflare/lib/workers-kv-namespace';
import { Worker } from '@cdktf/provider-cloudflare/lib/worker';
import { WorkerVersion } from '@cdktf/provider-cloudflare/lib/worker-version';
import { WorkersDeployment } from '@cdktf/provider-cloudflare/lib/workers-deployment';
import * as path from 'path';

class CloudflareStack extends TerraformStack {
  constructor(scope: Construct, id: string) {
    super(scope, id);

    const accountId = process.env.CLOUDFLARE_ACCOUNT_ID;
    if (!accountId) {
      throw new Error('CLOUDFLARE_ACCOUNT_ID environment variable is required');
    }

    const subdomain = process.env.CLOUDFLARE_SUBDOMAIN;
    if (!subdomain) {
      throw new Error('CLOUDFLARE_SUBDOMAIN environment variable is required');
    }

    new CloudflareProvider(this, 'cloudflare', {});

    // Create KV namespace for JsonStore
    const kvNamespace = new WorkersKvNamespace(this, 'jsonstore-kv', {
      accountId,
      title: 'hello-world-jsonstore'
    });

    // Bundled worker scripts paths
    const distDir = path.resolve(__dirname, '../dist/cloudflare');
    const jsonStoreWorkerPath = path.join(distDir, 'jsonstore-worker.js');
    const apiWorkerPath = path.join(distDir, 'api-worker.js');

    // JsonStore Worker (depends on kvNamespace for proper destroy ordering)
    const jsonStoreWorker = new Worker(this, 'jsonstore-worker', {
      accountId,
      name: 'hello-world-jsonstore',
      observability: {
        enabled: true,
        logs: {
          enabled: true,
          invocationLogs: true
        }
      },
      subdomain: {
        enabled: true
      },
      dependsOn: [kvNamespace]
    });

    const jsonStoreVersion = new WorkerVersion(this, 'jsonstore-version', {
      accountId,
      workerId: jsonStoreWorker.id,
      mainModule: 'index.js',
      modules: [{
        name: 'index.js',
        contentFile: jsonStoreWorkerPath,
        contentType: 'application/javascript+module'
      }],
      bindings: [{
        type: 'kv_namespace',
        name: 'JSONSTORE_KV',
        namespaceId: kvNamespace.id
      }],
      compatibilityDate: '2024-09-23',
      compatibilityFlags: ['nodejs_compat']
    });

    const jsonStoreDeployment = new WorkersDeployment(this, 'jsonstore-deployment', {
      accountId,
      scriptName: jsonStoreWorker.name,
      strategy: 'percentage',
      versions: [{
        versionId: jsonStoreVersion.id,
        percentage: 100
      }]
    });

    // API Worker (depends on jsonStoreWorker for proper destroy ordering)
    const apiWorker = new Worker(this, 'api-worker', {
      accountId,
      name: 'hello-world-api',
      observability: {
        enabled: true,
        logs: {
          enabled: true,
          headSamplingRate: 1,
          invocationLogs: true
        }
      },
      subdomain: {
        enabled: true
      },
      dependsOn: [jsonStoreWorker]
    });

    const apiVersion = new WorkerVersion(this, 'api-version', {
      accountId,
      workerId: apiWorker.id,
      mainModule: 'index.js',
      modules: [{
        name: 'index.js',
        contentFile: apiWorkerPath,
        contentType: 'application/javascript+module'
      }],
      bindings: [{
        type: 'service',
        name: 'JSONSTORE',
        service: jsonStoreWorker.name
      }],
      compatibilityDate: '2024-09-23',
      compatibilityFlags: ['nodejs_compat'],
      dependsOn: [jsonStoreDeployment]
    });

    new WorkersDeployment(this, 'api-deployment', {
      accountId,
      scriptName: apiWorker.name,
      strategy: 'percentage',
      versions: [{
        versionId: apiVersion.id,
        percentage: 100
      }]
    });

    new TerraformOutput(this, 'kv-namespace-id', {
      value: kvNamespace.id,
      description: 'KV namespace ID for JsonStore'
    });

    new TerraformOutput(this, 'api-worker-name', {
      value: apiWorker.name,
      description: 'API Worker name'
    });

    const apiBaseUrl = `https://hello-world-api.${subdomain}.workers.dev`;

    new TerraformOutput(this, 'api-endpoint', {
      value: `${apiBaseUrl}/v1/api/hello/{name}`,
      description: 'API endpoint for the hello service'
    });

    new TerraformOutput(this, 'example-curl', {
      value: `curl ${apiBaseUrl}/v1/api/hello/world`,
      description: 'Example curl command to test the API'
    });
  }
}

export const synth_cloudflare = () => {
  const app = new App();
  new CloudflareStack(app, 'hello-world-cloudflare');
  app.synth();

  return app;
};
