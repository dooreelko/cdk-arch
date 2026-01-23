import { Construct } from 'constructs';
import { Function, TBDFunction } from './function';

export interface ApiRoutes {
  [path: string]: Function;
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
  private namedRoutes: Map<string, RouteEntry> = new Map();

  constructor(scope: Construct, id: string, routes: ApiRoutes = {}) {
    super(scope, id);
    this.routes = routes;
  }

  addRoute(name: string, path: string, handler: Function): void {
    this.routes[path] = handler;
    this.namedRoutes.set(name, { name, path, handler });
  }

  getRoute(path: string): Function | undefined {
    return this.routes[path];
  }

  getRouteByName(name: string): RouteEntry | undefined {
    return this.namedRoutes.get(name);
  }

  listRoutes(): string[] {
    return Object.keys(this.routes);
  }

  listNamedRoutes(): RouteEntry[] {
    return Array.from(this.namedRoutes.values());
  }

  /**
   * Returns a list of TBDFunctions that have not been overloaded.
   * Use this to validate that all required implementations are provided.
   */
  validateOverloads(): Function[] {
    return Object.values(this.routes)
      .filter(fn => fn instanceof TBDFunction && !fn.hasOverload());
  }
}
