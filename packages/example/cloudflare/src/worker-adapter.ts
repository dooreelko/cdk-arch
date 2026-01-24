import { log, err } from 'architecture';
import { ApiContainer, Function, FunctionHandler } from '@arinoto/cdk-arch';

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
export function createWorkerHandler(container: ApiContainer): (request: Request) => Promise<Response> {
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

/**
 * Create a handler that calls another Worker via service binding.
 */
export function serviceBindingHandler(
  getBinding: () => { fetch: typeof fetch },
  route: string
): FunctionHandler {
  return async (...args: any[]) => {
    const [method, path] = route.split(' ');
    const pathParams = path.match(/\{(\w+)\}/g) || [];

    const url = pathParams.reduce(
      (u, param, i) => args[i] !== undefined ? u.replace(param, encodeURIComponent(String(args[i]))) : u,
      `https://example.com${path}`
    );

    const options: RequestInit = {
      method: method || 'GET',
      headers: { 'Content-Type': 'application/json' },
      ...((method === 'POST' || method === 'PUT') && args.length > pathParams.length
        ? { body: JSON.stringify(args[pathParams.length]) }
        : {})
    };

    const binding = getBinding();
    log('will do remote call', {path, url, options, 'has_fetch': !!binding.fetch, fetch: binding.fetch.toString()});
    try {
      const response = await binding.fetch(url, options);
      log('did remote call', {ok: response.ok, status: response.status, stext: response.statusText, response});
      return response.json();
    } catch (error) {
      err('remote call failed', {error});
      throw error;
    }
  };
}

