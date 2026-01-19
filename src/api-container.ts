import { Construct } from 'constructs';
import { Function, TBDFunction } from './function';

export interface ApiRoutes {
  [path: string]: Function;
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

  addRoute(path: string, handler: Function): void {
    this.routes[path] = handler;
  }

  getRoute(path: string): Function | undefined {
    return this.routes[path];
  }

  listRoutes(): string[] {
    return Object.keys(this.routes);
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
