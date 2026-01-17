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

  /**
   * Bind an architectural component to a service endpoint.
   * This updates any Functions within the component to make HTTP calls instead of direct invocations.
   */
  bind(component: ApiContainer, endpoint: ServiceEndpoint): void {
    this.bindings.set(component, endpoint);
  }

  /**
   * Get the endpoint for a bound component
   */
  getEndpoint(component: Construct): ServiceEndpoint | undefined {
    return this.bindings.get(component);
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
