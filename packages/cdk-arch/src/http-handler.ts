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
    const baseUrl = pathParams.reduce(
      (u, param, i) => args[i] !== undefined ? u.replace(param, encodeURIComponent(String(args[i]))) : u,
      `${endpoint.baseUrl}${path}`
    );

    const extraArgs = args.slice(pathParams.length);
    const isGet = (method ?? 'GET') === 'GET';

    let url = baseUrl;
    if (isGet && extraArgs.length > 0 && extraArgs[0] != null && typeof extraArgs[0] === 'object') {
      const qs = new URLSearchParams(extraArgs[0] as Record<string, string>).toString();
      if (qs) { url = `${baseUrl}?${qs}`; }
    }

    const options: RequestInit = {
      method: method || 'GET',
      headers: { 'Content-Type': 'application/json' },
      ...(!isGet && extraArgs.length > 0
        ? { body: extraArgs.length === 1 ? JSON.stringify(extraArgs[0]) : JSON.stringify(extraArgs) }
        : {})
    };

    const response = await fetcher().fetch(url, options);

    if (!response.ok) {
      throw new Error(`Remote call to ${url} with ${JSON.stringify(options)} failed: ${response.status} ${response.statusText}`);
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
