import { Construct } from 'constructs';
import { Function } from './function';

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

  public addRoute(path: string, handler: Function): void {
    this.routes[path] = handler;
  }

  public getRoute(path: string): Function | undefined {
    return this.routes[path];
  }

  public listRoutes(): string[] {
    return Object.keys(this.routes);
  }
}
