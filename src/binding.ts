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
 * Binding options for an ApiContainer
 */
export interface BindOptions extends ServiceEndpoint {
  /**
   * Override function implementations.
   * Keys are function property names on the container.
   */
  overloads?: Record<string, FunctionHandler>;
}

/**
 * Binding between architectural components and their runtime endpoints.
 * Supports function overloading for replacing implementations at runtime.
 */
export class ArchitectureBinding {
  private bindings: Map<Construct, ServiceEndpoint> = new Map();
  private localComponents: Set<Construct> = new Set();

  /**
   * Bind an architectural component to a service endpoint.
   * Optionally override function implementations with the overloads option.
   */
  bind(component: ApiContainer, options: BindOptions): void {
    this.bindings.set(component, { host: options.host, port: options.port });

    Object.entries(options.overloads ?? {})
      .filter(([name]) => (component as any)[name] instanceof Function)
      .forEach(([name, handler]) => (component as any)[name].overload(handler));
  }

  /**
   * Bind a component from environment variables.
   * Looks for {PREFIX}_HOST and {PREFIX}_PORT environment variables.
   */
  bindFromEnv(component: ApiContainer, envPrefix: string, overloads?: Record<string, FunctionHandler>): void {
    const host = process.env[`${envPrefix}_HOST`];
    const port = process.env[`${envPrefix}_PORT`];
    if (host && port) {
      this.bind(component, { host, port: parseInt(port), overloads });
    }
  }

  getEndpoint(component: Construct): ServiceEndpoint | undefined {
    return this.bindings.get(component);
  }

  getAllBindings(): Map<Construct, ServiceEndpoint> {
    return this.bindings;
  }

  setLocal(component: Construct): void {
    this.localComponents.add(component);
  }

  isLocal(component: Construct): boolean {
    return this.localComponents.has(component);
  }
}

/**
 * Global binding registry
 */
export const architectureBinding = new ArchitectureBinding();
