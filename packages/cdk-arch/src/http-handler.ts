import { ApiContainer, ApiRoutes, RouteHandlers } from './api-container';
import { FunctionHandler } from './function';
import { ServiceEndpoint } from './binding';

export type Fetcher = () => { fetch: typeof fetch };

/**
 * Create an HTTP handler for a route by name.
 * Looks up the route path from the container's registry.
 */
export const httpHandler = <
  TRoutes extends ApiRoutes,
  K extends keyof TRoutes & string
>(
  endpoint: ServiceEndpoint,
  container: ApiContainer<TRoutes>,
  routeName: K,
  fetcher: Fetcher = () => ({fetch})
): RouteHandlers<TRoutes>[K] => {
  const route = container.getRoute(routeName);

  const [method, path] = route.path.split(' ');
  const pathParams = path.match(/\{(\w+)\}/g) || [];

  const handler = async (...args: any[]) => {
    const url = pathParams.reduce(
      (u, param, i) => args[i] !== undefined ? u.replace(param, encodeURIComponent(String(args[i]))) : u,
      `${endpoint.baseUrl}${path}`
    );

    const options: RequestInit = {
      method: method || 'GET',
      headers: { 'Content-Type': 'application/json' },
      ...((method === 'POST' || method === 'PUT') && args.length > pathParams.length
        ? { body: JSON.stringify(args[pathParams.length]) }
        : {})
    };

    console.log('HTTP handler. Will fetch', {url, options});
    const response = await fetcher().fetch(url, options);

    if (!response.ok) {
      throw new Error(`Remote call to ${url} with ${JSON.stringify(options)} failed: ${JSON.stringify(response)}`);
    }

    return response.json();
  };

  return handler as RouteHandlers<TRoutes>[K];
};

/**
 * Create HTTP bindings for multiple routes as a callable client.
 * Returns a strongly-typed object where each route name maps to an async function
 * with the same signature as the original handler.
 *
 * @typeParam TRoutes - The routes type from the ApiContainer
 * @typeParam K - The subset of route names to include in the client
 *
 * @example
 * ```typescript
 * const api = new ApiContainer(arch, 'api', {
 *   hello: { path: 'GET /v1/api/hello/{name}', handler: helloFunction },
 *   hellos: { path: 'GET /v1/api/hellos', handler: hellosFunction }
 * });
 *
 * const client = createHttpBindings(endpoint, api, ['hello', 'hellos']);
 * await client.hello('John');  // (name: string) => Promise<string>
 * await client.hellos();       // () => Promise<Greeting[]>
 * ```
 */
export const createHttpBindings = <
  TRoutes extends ApiRoutes,
  K extends keyof TRoutes & string
>(
  endpoint: ServiceEndpoint,
  container: ApiContainer<TRoutes>,
  routeNames: readonly K[],
  fetcher: Fetcher = () => ({fetch})
): Pick<RouteHandlers<TRoutes>, K> => {
  return routeNames.reduce(
    (acc, name) => {
      acc[name] = httpHandler(endpoint, container, name, fetcher);
      return acc;
    },
    {} as Pick<RouteHandlers<TRoutes>, K>
  );
};
