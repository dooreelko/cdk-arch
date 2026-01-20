/**
 * Minimal runtime for Cloudflare Workers.
 * Does not depend on constructs or Node.js modules.
 */

export type FunctionHandler = (...args: any[]) => any;

export interface RouteDefinition {
  method: string;
  path: string;
  params: string[];
  pattern: RegExp;
  handler: FunctionHandler;
}

/**
 * Simple function wrapper with overload support.
 */
export class WorkerFunction {
  private _handler: FunctionHandler;
  private _overload?: FunctionHandler;

  constructor(handler: FunctionHandler) {
    this._handler = handler;
  }

  overload(handler: FunctionHandler): void {
    this._overload = handler;
  }

  invoke(...args: any[]): any {
    const fn = this._overload ?? this._handler;
    return fn(...args);
  }
}

/**
 * Parse a route string into components.
 */
function parseRoute(route: string): { method: string; path: string; params: string[]; pattern: RegExp } {
  const parts = route.split(' ');
  const method = parts.length === 2 ? parts[0] : 'GET';
  const path = parts.length === 2 ? parts[1] : parts[0];
  const params = (path.match(/\{(\w+)\}/g) || []).map(p => p.slice(1, -1));
  const regexPath = path.replace(/\{(\w+)\}/g, '([^/]+)');

  return {
    method,
    path,
    params,
    pattern: new RegExp(`^${regexPath}$`)
  };
}

/**
 * Worker-compatible request handler.
 */
export class WorkerRouter {
  private routes: RouteDefinition[] = [];

  addRoute(route: string, handler: FunctionHandler): void {
    const { method, path, params, pattern } = parseRoute(route);
    this.routes.push({ method, path, params, pattern, handler });
  }

  async handle(request: Request): Promise<Response> {
    const url = new URL(request.url);
    const method = request.method;
    const path = url.pathname;

    for (const route of this.routes) {
      if (route.method !== method) continue;

      const match = path.match(route.pattern);
      if (!match) continue;

      try {
        const pathArgs = route.params.map((_, i) => decodeURIComponent(match[i + 1]));
        const bodyArg = (method === 'POST' || method === 'PUT')
          ? [await request.json()]
          : [];
        const args = [...pathArgs, ...bodyArg];

        const result = await route.handler(...args);
        return new Response(JSON.stringify(result), {
          headers: { 'Content-Type': 'application/json' }
        });
      } catch (error: any) {
        console.error(`Error handling ${route.method} ${path}:`, error);
        return new Response(
          JSON.stringify({ error: error.message || 'Internal server error' }),
          { status: 500, headers: { 'Content-Type': 'application/json' } }
        );
      }
    }

    return new Response(
      JSON.stringify({ error: 'Not found' }),
      { status: 404, headers: { 'Content-Type': 'application/json' } }
    );
  }
}

/**
 * Create a handler that calls another Worker via service binding.
 */
export const serviceBindingHandler = (
  getBinding: () => { fetch: typeof fetch },
  route: string
): FunctionHandler => {
  return async (...args: any[]) => {
    const [method, path] = route.split(' ');
    const pathParams = path.match(/\{(\w+)\}/g) || [];

    const url = pathParams.reduce(
      (u, param, i) => args[i] !== undefined ? u.replace(param, encodeURIComponent(String(args[i]))) : u,
      `https://internal${path}`
    );

    const options: RequestInit = {
      method: method || 'GET',
      headers: { 'Content-Type': 'application/json' },
      ...((method === 'POST' || method === 'PUT') && args.length > pathParams.length
        ? { body: JSON.stringify(args[pathParams.length]) }
        : {})
    };

    const binding = getBinding();
    const response = await binding.fetch(url, options);
    return response.json();
  };
};
