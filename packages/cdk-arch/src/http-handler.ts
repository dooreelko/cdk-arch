import { ApiContainer } from './api-container';
import { FunctionHandler } from './function';
import { ServiceEndpoint } from './binding';

export type Fetcher = () => { fetch: typeof fetch };

/**
 * Create an HTTP handler for a route by name.
 * Looks up the route path from the container's registry.
 */
export const httpHandler = (
  endpoint: ServiceEndpoint,
  container: ApiContainer,
  routeName: string,
  fetcher: Fetcher = () => ({fetch})
): FunctionHandler => {
  const route = container.getRoute(routeName);
  if (!route) {
    throw new Error(`Route '${routeName}' not found in container '${container.node.id}'`);
  }

  const [method, path] = route.path.split(' ');
  const pathParams = path.match(/\{(\w+)\}/g) || [];

  return async (...args: any[]) => {
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
};

/**
 * Create HTTP bindings for multiple routes as a callable client.
 * Returns an object where each route name maps to an async function
 * that makes HTTP calls to the remote endpoint.
 *
 * @example
 * const client = createHttpBindings(endpoint, api, ['hello', 'hellos']);
 * await client.hello('John');  // Makes HTTP call
 * await client.hellos();       // Makes HTTP call
 */
export const createHttpBindings = <T extends string>(
  endpoint: ServiceEndpoint,
  container: ApiContainer,
  routeNames: T[],
  fetcher: Fetcher = () => ({fetch})
): Record<T, FunctionHandler> => {
  return routeNames.reduce(
    (acc, name) => {
      acc[name] = httpHandler(endpoint, container, name, fetcher);
      return acc;
    },
    {} as Record<T, FunctionHandler>
  );
};