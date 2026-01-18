import { Construct } from 'constructs';
import { ApiContainer } from './api-container';
import { Function, FunctionHandler } from './function';

/**
 * Service discovery configuration for runtime
 */
export interface ServiceEndpoint {
  host: string;
  port: number;
}

/**
 * Binding between architectural components and their CDKTF implementations.
 * When bound, function calls are replaced with HTTP calls via service discovery.
 */
export class ArchitectureBinding {
  private bindings: Map<Construct, ServiceEndpoint> = new Map();
  private localComponents: Set<Construct> = new Set();

  /**
   * Bind an architectural component to a service endpoint.
   * This updates any Functions within the component to make HTTP calls instead of direct invocations.
   */
  bind(component: ApiContainer, endpoint: ServiceEndpoint): void {
    this.bindings.set(component, endpoint);
  }

  /**
   * Bind a component from environment variables.
   * Looks for {PREFIX}_HOST and {PREFIX}_PORT environment variables.
   */
  bindFromEnv(component: ApiContainer, envPrefix: string): void {
    const host = process.env[`${envPrefix}_HOST`];
    const port = process.env[`${envPrefix}_PORT`];
    if (host && port) {
      this.bind(component, { host, port: parseInt(port) });
    }
  }

  /**
   * Get the endpoint for a bound component
   */
  getEndpoint(component: Construct): ServiceEndpoint | undefined {
    return this.bindings.get(component);
  }

  /**
   * Get all bindings
   */
  getAllBindings(): Map<Construct, ServiceEndpoint> {
    return this.bindings;
  }

  /**
   * Mark a component as locally served (not requiring HTTP calls)
   */
  setLocal(component: Construct): void {
    this.localComponents.add(component);
  }

  /**
   * Check if a component is served locally
   */
  isLocal(component: Construct): boolean {
    return this.localComponents.has(component);
  }

  /**
   * Check if a component should use HTTP (bound but not local)
   */
  isRemote(component: Construct): boolean {
    return this.bindings.has(component) && !this.localComponents.has(component);
  }

  /**
   * Enable remote mode for an ApiContainer.
   * This patches the container's route-based methods to make HTTP calls
   * instead of invoking functions directly.
   */
  enableRemote(container: ApiContainer): void {
    const endpoint = this.getEndpoint(container);
    if (!endpoint) {
      throw new Error(`Cannot enable remote mode: ${container.node.id} is not bound to an endpoint`);
    }

    // Patch each route's corresponding method on the container
    for (const [route, fn] of Object.entries(container.routes)) {
      const httpHandler = this.createHttpWrapper(endpoint, route);

      // Replace the function's invoke method to use HTTP
      fn.invoke = httpHandler;
    }
  }

  /**
   * Create an HTTP wrapper function that calls the remote service
   */
  createHttpWrapper(endpoint: ServiceEndpoint, route: string): FunctionHandler {
    return async (...args: any[]) => {
      const [method, path] = route.split(' ');
      let url = `http://${endpoint.host}:${endpoint.port}${path}`;

      // Replace path parameters with args
      const pathParams = path.match(/\{(\w+)\}/g) || [];
      pathParams.forEach((param, index) => {
        if (args[index] !== undefined) {
          url = url.replace(param, encodeURIComponent(String(args[index])));
        }
      });

      const options: RequestInit = {
        method: method || 'GET',
        headers: { 'Content-Type': 'application/json' }
      };

      // For POST/PUT, use remaining args as body
      if ((method === 'POST' || method === 'PUT') && args.length > pathParams.length) {
        options.body = JSON.stringify(args[pathParams.length]);
      }

      const response = await fetch(url, options);
      return response.json();
    };
  }
}

/**
 * Global binding registry
 */
export const architectureBinding = new ArchitectureBinding();
