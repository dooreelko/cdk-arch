import { Construct } from 'constructs';
import { ApiContainer } from './api-container';
import { Function, FunctionHandler } from './function';

/**
 * Service discovery configuration for runtime
 */
export interface ServiceEndpoint {
  baseUrl: string;
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
   * Overload keys must be route names registered via addRoute.
   */
  bind(component: ApiContainer, options: BindOptions): void {
    this.bindings.set(component, { baseUrl: options.baseUrl });

    Object.entries(options.overloads ?? {}).forEach(([name, handler]) => {
      const route = component.getRoute(name);
      if (!route) {
        throw new Error(`Route '${name}' not found in component '${component.node.id}'`);
      }
      if (!(route.handler instanceof Function)) {
        throw new Error(`Route '${name}' handler is not a Function`);
      }
      route.handler.overload(handler);
    });
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
