import { Construct } from 'constructs';
import { Function, TBDFunction } from './function';

/**
 * A route entry that captures the handler's argument and return types.
 */
export interface RouteEntry<TArgs extends any[] = any[], TReturn = any> {
  path: string;
  handler: Function<TArgs, TReturn>;
}

/**
 * Base type for route definitions - maps route names to route entries.
 */
export type ApiRoutes = {
  [name: string]: RouteEntry;
}

/**
 * Extract the handler signature from a Function type.
 */
export type HandlerOf<T> = T extends Function<infer Args, infer Return>
  ? (...args: Args) => Promise<Return>
  : never;

/**
 * Extract handler signatures from all routes in an ApiRoutes type.
 */
export type RouteHandlers<TRoutes extends ApiRoutes> = {
  [K in keyof TRoutes]: HandlerOf<TRoutes[K]['handler']>;
};

/**
 * Represents an API container that routes requests to functions.
 *
 * @typeParam TRoutes - The type of the routes object, preserving route names and handler signatures.
 *
 * @example
 * ```typescript
 * const api = new ApiContainer(arch, 'api', {
 *   hello: { path: 'GET /v1/api/hello/{name}', handler: helloFunction },
 *   hellos: { path: 'GET /v1/api/hellos', handler: hellosFunction }
 * });
 * // api has type ApiContainer<{ hello: RouteEntry<[string], string>, hellos: RouteEntry<[], Greeting[]> }>
 * ```
 */
export class ApiContainer<TRoutes extends ApiRoutes = ApiRoutes> extends Construct {
  public readonly routes: TRoutes;

  constructor(scope: Construct, id: string, routes: TRoutes = {} as TRoutes) {
    super(scope, id);
    this.routes = routes;
  }

  addRoute<TArgs extends any[], TReturn>(
    name: string,
    path: string,
    handler: Function<TArgs, TReturn>
  ): void {
    (this.routes as ApiRoutes)[name] = { path, handler };
  }

  getRoute<K extends keyof TRoutes & string>(name: K): TRoutes[K] {
    const entry = this.routes[name];
    if (!entry) {
      throw new Error(`Route '${name}' not found in container '${this.node.id}'`);
    }
    return entry;
  }

  listRoutes(): (keyof TRoutes & string)[] {
    return Object.keys(this.routes) as (keyof TRoutes & string)[];
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
