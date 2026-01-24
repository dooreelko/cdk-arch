import { Construct } from 'constructs';
import { Function, TBDFunction } from './function';

export interface ApiRoutes {
  [name: string]: RouteEntry;
}

export interface RouteEntry {
  name: string;
  path: string;
  handler: Function;
}

/**
 * Represents an API container that routes requests to functions
 */
export class ApiContainer extends Construct {
  public readonly routes: ApiRoutes;

  constructor(scope: Construct, id: string, routes: ApiRoutes = {}) {
    super(scope, id);
    this.routes = routes;
  }

  addRoute(name: string, path: string, handler: Function): void {
    this.routes[name] = { name, path, handler };
  }

  getRoute(name: string): RouteEntry {
    const entry = this.routes[name];
    if (!entry) {
      throw new Error(`Route '${name}' not found in container '${this.node.id}'`);
    }
    return entry;
  }

  listRoutes() : string[] {
    return Object.keys(this.routes);
  }

  /**
   * Returns a list of TBDFunctions that have not been overloaded.
   * Use this to validate that all required implementations are provided.
   */
  validateOverloads(): Function[] {
    return Object.values(this.routes)
      .map(entry => entry.handler)
      .filter(fn => fn instanceof TBDFunction && !fn.hasOverload());
  }
}
