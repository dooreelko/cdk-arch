import { ApiContainer } from './api-container';
import { FunctionHandler } from './function';
import { ServiceEndpoint } from './binding';

/**
 * Create an HTTP handler for a route by name.
 * Looks up the route path from the container's registry.
 */
export const httpHandler = (
  endpoint: ServiceEndpoint,
  container: ApiContainer,
  routeName: string,
  fetcher: () => { fetch: typeof fetch } = () => ({fetch})
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