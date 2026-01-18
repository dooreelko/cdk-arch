import { ApiContainer } from './api-container';
import { ArchitectureBinding, ServiceEndpoint, architectureBinding } from './binding';
import { Construct } from 'constructs';
import { Function } from './function';

export interface DockerApiServerConfig {
  binding?: ArchitectureBinding;
}

export interface StorageAdapter {
  store(collection: string, document: any): Promise<{ success: boolean }>;
  get(collection: string): Promise<any[]>;
}

/**
 * Creates an Express server from an ApiContainer's route definitions.
 * Handles routing, parameter extraction, and cross-service HTTP calls.
 */
export class DockerApiServer {
  private container: ApiContainer;
  private binding: ArchitectureBinding;
  private app: any; // Express app
  private remoteClients: Map<ApiContainer, RemoteClient> = new Map();

  constructor(container: ApiContainer, config: DockerApiServerConfig = {}) {
    this.container = container;
    this.binding = config.binding || architectureBinding;

    // Mark this container as locally served
    this.binding.setLocal(container);

    // Create remote clients for all bound components except self
    this.setupRemoteClients();
  }

  private setupRemoteClients(): void {
    for (const [component, endpoint] of this.binding.getAllBindings()) {
      if (component === this.container) continue;
      if (component instanceof ApiContainer) {
        this.remoteClients.set(component, new RemoteClient(endpoint));
      }
    }
  }

  /**
   * Get the remote client for a component (for cross-service calls)
   */
  getRemoteClient(component: ApiContainer): RemoteClient | undefined {
    return this.remoteClients.get(component);
  }

  /**
   * Create and configure the Express app
   */
  createApp(express: any, storage?: StorageAdapter): any {
    this.app = express();
    this.app.use(express.json());

    for (const [route, fn] of Object.entries(this.container.routes)) {
      this.setupRoute(route, fn as Function, storage);
    }

    return this.app;
  }

  private setupRoute(route: string, fn: Function, storage?: StorageAdapter): void {
    const { method, expressPath, params } = this.parseRoute(route);

    this.app[method.toLowerCase()](expressPath, async (req: any, res: any) => {
      try {
        // Extract path parameters
        const args: any[] = params.map(p => req.params[p]);

        // For POST/PUT, add body
        if (method === 'POST' || method === 'PUT') {
          args.push(req.body);
        }

        // Execute the function with context
        const result = await this.executeFunction(fn, args, storage);
        res.json(result);
      } catch (error: any) {
        console.error(`Error handling ${route}:`, error);
        res.status(500).json({ error: error.message || 'Internal server error' });
      }
    });
  }

  private parseRoute(route: string): { method: string; path: string; expressPath: string; params: string[] } {
    const parts = route.split(' ');
    const method = parts.length === 2 ? parts[0] : 'GET';
    const path = parts.length === 2 ? parts[1] : parts[0];
    const expressPath = path.replace(/\{(\w+)\}/g, ':$1');
    const params = (path.match(/\{(\w+)\}/g) || []).map(p => p.slice(1, -1));

    return { method, path, expressPath, params };
  }

  private async executeFunction(fn: Function, args: any[], storage?: StorageAdapter): Promise<any> {
    // If storage adapter is provided, use it for store/get operations
    if (storage && fn.node.id === 'store-handler') {
      return storage.store(args[0], args[1]);
    }
    if (storage && fn.node.id === 'get-handler') {
      return storage.get(args[0]);
    }

    // Otherwise, invoke the function handler directly
    return fn.invoke(...args);
  }

  /**
   * Start the server
   */
  listen(port: number): void {
    this.app.listen(port, () => {
      console.log(`${this.container.node.id} server running on port ${port}`);
    });
  }
}

/**
 * HTTP client for calling remote ApiContainers
 */
export class RemoteClient {
  constructor(private endpoint: ServiceEndpoint) {}

  async store(collection: string, document: any): Promise<{ success: boolean }> {
    const response = await fetch(`http://${this.endpoint.host}:${this.endpoint.port}/store/${collection}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(document)
    });
    return response.json() as Promise<{ success: boolean }>;
  }

  async get(collection: string): Promise<any[]> {
    const response = await fetch(`http://${this.endpoint.host}:${this.endpoint.port}/get/${collection}`);
    return response.json() as Promise<any[]>;
  }

  async call(method: string, path: string, body?: any): Promise<any> {
    const url = `http://${this.endpoint.host}:${this.endpoint.port}${path}`;
    const options: RequestInit = {
      method,
      headers: { 'Content-Type': 'application/json' }
    };
    if (body) {
      options.body = JSON.stringify(body);
    }
    const response = await fetch(url, options);
    return response.json();
  }
}
