import { App, TerraformStack, TerraformOutput } from 'cdktf';
import { Construct } from 'constructs';
import { CloudflareProvider } from '@cdktf/provider-cloudflare/lib/provider';
import { WorkersKvNamespace } from '@cdktf/provider-cloudflare/lib/workers-kv-namespace';
import { WorkersScript } from '@cdktf/provider-cloudflare/lib/workers-script';
import * as path from 'path';
import * as fs from 'fs';

class CloudflareStack extends TerraformStack {
  constructor(scope: Construct, id: string) {
    super(scope, id);

    const accountId = process.env.CLOUDFLARE_ACCOUNT_ID;
    if (!accountId) {
      throw new Error('CLOUDFLARE_ACCOUNT_ID environment variable is required');
    }

    new CloudflareProvider(this, 'cloudflare', {});

    // Create KV namespace for JsonStore
    const kvNamespace = new WorkersKvNamespace(this, 'jsonstore-kv', {
      accountId,
      title: 'hello-world-jsonstore'
    });

    // Read bundled worker scripts
    const distDir = path.resolve(__dirname, '../../../dist/cloudflare');
    const jsonStoreWorkerScript = fs.readFileSync(
      path.join(distDir, 'jsonstore-worker.js'),
      'utf-8'
    );
    const apiWorkerScript = fs.readFileSync(
      path.join(distDir, 'api-worker.js'),
      'utf-8'
    );

    // JsonStore Worker with KV binding
    const jsonStoreWorker = new WorkersScript(this, 'jsonstore-worker', {
      accountId,
      name: 'hello-world-jsonstore',
      content: jsonStoreWorkerScript,
      kvNamespaceBinding: [{
        name: 'JSONSTORE_KV',
        namespaceId: kvNamespace.id
      }],
      module: true
    });

    // API Worker with service binding to JsonStore
    new WorkersScript(this, 'api-worker', {
      accountId,
      name: 'hello-world-api',
      content: apiWorkerScript,
      serviceBinding: [{
        name: 'JSONSTORE',
        service: jsonStoreWorker.name
      }],
      module: true
    });

    new TerraformOutput(this, 'kv-namespace-id', {
      value: kvNamespace.id,
      description: 'KV namespace ID for JsonStore'
    });

    new TerraformOutput(this, 'api-worker-name', {
      value: 'hello-world-api',
      description: 'API Worker name'
    });

    new TerraformOutput(this, 'api-endpoint', {
      value: 'https://hello-world-api.<subdomain>.workers.dev/v1/api/hello/{name}',
      description: 'API endpoint (replace <subdomain> with your Cloudflare subdomain)'
    });

    new TerraformOutput(this, 'example-curl', {
      value: 'curl https://hello-world-api.<subdomain>.workers.dev/v1/api/hello/world',
      description: 'Example curl command (replace <subdomain> with your Cloudflare subdomain)'
    });
  }
}

export const synth_cloudflare = () => {
  const app = new App();
  new CloudflareStack(app, 'hello-world-cloudflare');
  app.synth();

  return app;
};
