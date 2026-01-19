import { App, TerraformStack, TerraformOutput, Fn } from 'cdktf';
import { Construct } from 'constructs';
import { CloudflareProvider } from '@cdktf/provider-cloudflare/lib/provider';
import { WorkersKvNamespace } from '@cdktf/provider-cloudflare/lib/workers-kv-namespace';
import { WorkersScript } from '@cdktf/provider-cloudflare/lib/workers-script';
import { NullProvider } from '@cdktf/provider-null/lib/provider';
import { Resource as NullResource } from '@cdktf/provider-null/lib/resource';
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

    // Bundled worker scripts paths (read at Terraform apply time via file() function)
    const distDir = path.resolve(__dirname, '../../dist/cloudflare');
    const jsonStoreWorkerPath = path.join(distDir, 'jsonstore-worker.js');
    const apiWorkerPath = path.join(distDir, 'api-worker.js');

    // JsonStore Worker with KV binding
    const jsonStoreWorker = new WorkersScript(this, 'jsonstore-worker', {
      accountId,
      name: 'hello-world-jsonstore',
      content: Fn.file(jsonStoreWorkerPath),
      kvNamespaceBinding: [{
        name: 'JSONSTORE_KV',
        namespaceId: kvNamespace.id
      }],
      module: true
    });

    // API Worker with service binding to JsonStore
    const apiWorker = new WorkersScript(this, 'api-worker', {
      accountId,
      name: 'hello-world-api',
      content: Fn.file(apiWorkerPath),
      serviceBinding: [{
        name: 'JSONSTORE',
        service: jsonStoreWorker.name
      }],
      module: true
    });

    // Enable workers.dev subdomain for API Worker
    new NullProvider(this, 'null', {});
    const enableSubdomain = new NullResource(this, 'enable-api-subdomain', {
      triggers: {
        worker_id: apiWorker.id
      }
    });
    enableSubdomain.addOverride('provisioner.local-exec.command',
      `curl -s -X POST "https://api.cloudflare.com/client/v4/accounts/${accountId}/workers/scripts/hello-world-api/subdomain" ` +
      `-H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" ` +
      `-H "Content-Type: application/json" ` +
      `--data '{"enabled":true}'`
    );

    new TerraformOutput(this, 'kv-namespace-id', {
      value: kvNamespace.id,
      description: 'KV namespace ID for JsonStore'
    });

    new TerraformOutput(this, 'api-worker-name', {
      value: 'hello-world-api',
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
