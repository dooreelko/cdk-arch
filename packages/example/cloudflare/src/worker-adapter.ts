import { log, err } from 'architecture';
import { ApiContainer, ApiRoutes, Function, FunctionHandler } from '@arinoto/cdk-arch';

interface RouteMatch {
  method: string;
  pattern: RegExp;
  params: string[];
  fn: Function;
}

/**
 * Parse a route string like "GET /v1/api/hello/{name}" into components.
 */
function parseRoute(route: string, fn: Function): RouteMatch {
  const parts = route.split(' ');
  const method = parts.length === 2 ? parts[0] : 'GET';
  const path = parts.length === 2 ? parts[1] : parts[0];
  const params = (path.match(/\{(\w+)\}/g) || []).map(p => p.slice(1, -1));
  const regexPath = path.replace(/\{(\w+)\}/g, '([^/]+)');

  return {
    method,
    pattern: new RegExp(`^${regexPath}$`),
    params,
    fn
  };
}

/**
 * Create a Cloudflare Worker fetch handler from an ApiContainer.
 */
export function createWorkerHandler<TRoutes extends ApiRoutes>(container: ApiContainer<TRoutes>): (request: Request) => Promise<Response> {
  const routes = Object.values(container.routes)
    .map((entry) => parseRoute(entry.path, entry.handler));

  return async (request: Request): Promise<Response> => {
    // log('worker start', {request});
    const url = new URL(request.url);
    const method = request.method;
    const path = url.pathname;

    for (const route of routes) {
      if (route.method !== method) continue;

      const match = path.match(route.pattern);
      if (!match) continue;

      // log('worker route', {route});

      try {
        const pathArgs = route.params.map((_, i) => decodeURIComponent(match[i + 1]));
        const bodyArg = (method === 'POST' || method === 'PUT')
          ? [await request.json()]
          : [];
        const args = [...pathArgs, ...bodyArg];

        log('worker invoke', {route, args});
        const result = await route.fn.invoke(...args);

        log('worker result', {result});
        return new Response(JSON.stringify(result), {
          headers: { 'Content-Type': 'application/json' }
        });
      } catch (error: any) {
        err(`Error handling ${route.method} ${path}:`, {error});
        return new Response(
          JSON.stringify({ error: error.message || 'Internal server error' }),
          { status: 500, headers: { 'Content-Type': 'application/json' } }
        );
      }
    }

    err('worker FAIL. NO ROUTE.', {path, routes});

    return new Response(
      JSON.stringify({ error: 'Not found' }),
      { status: 404, headers: { 'Content-Type': 'application/json' } }
    );
  };
}
