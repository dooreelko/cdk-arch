import { ApiContainer, ApiRoutes, ArchitectureBinding, architectureBinding, Function } from '@arinoto/cdk-arch';

export interface DockerApiServerConfig {
  binding?: ArchitectureBinding;
}

/**
 * Creates an Express server from an ApiContainer's route definitions.
 * Handles routing and parameter extraction. Function implementations
 * are provided via binding overloads.
 */
export class DockerApiServer<TRoutes extends ApiRoutes = ApiRoutes> {
  private container: ApiContainer<TRoutes>;
  private binding: ArchitectureBinding;
  private app: any;

  constructor(container: ApiContainer<TRoutes>, config: DockerApiServerConfig = {}) {
    this.container = container;
    this.binding = config.binding || architectureBinding;
    this.binding.setLocal(container);
  }

  createApp(express: any): any {
    this.app = express();
    this.app.use(express.json());

    Object.values(this.container.routes)
      .forEach((entry) => this.setupRoute(entry.path, entry.handler));

    return this.app;
  }

  private setupRoute(route: string, fn: Function): void {
    const { method, expressPath, params } = this.parseRoute(route);
    this.app[method.toLowerCase()](expressPath, async (req: any, res: any) => {
      try {
        const pathArgs = params.map(p => req.params[p]);
        const args = (method === 'POST' || method === 'PUT')
        ? [...pathArgs, req.body]
        : pathArgs;
        
        console.log('Will invoke', {method, expressPath, params, args});

        const result = await fn.invoke(...args);
        res.json(result);
      } catch (error: any) {
        console.error(`Error handling ${route}:`, error);
        res.status(500).json({ error: error.message || 'Internal server error' });
      }
    });
  }

  private parseRoute(route: string): { method: string; expressPath: string; params: string[] } {
    const parts = route.split(' ');
    const method = parts.length === 2 ? parts[0] : 'GET';
    const path = parts.length === 2 ? parts[1] : parts[0];

    return {
      method,
      expressPath: path.replace(/\{(\w+)\}/g, ':$1'),
      params: (path.match(/\{(\w+)\}/g) || []).map(p => p.slice(1, -1))
    };
  }

  listen(port: number): void {
    this.app.listen(port, () => {
      console.log(`${this.container.node.id} server running on port ${port}`);
    });
  }

  start(express: any, port: number): void {
    this.createApp(express);
    this.listen(port);
  }
}
