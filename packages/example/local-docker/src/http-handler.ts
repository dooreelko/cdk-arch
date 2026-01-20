import { FunctionHandler, ServiceEndpoint } from 'cdk-arch';

/**
 * Create an HTTP handler for a route.
 * Use this to create overloads that make HTTP calls to remote services.
 */
export const httpHandler = (endpoint: ServiceEndpoint, route: string): FunctionHandler => {
  return async (...args: any[]) => {
    const [method, path] = route.split(' ');
    const pathParams = path.match(/\{(\w+)\}/g) || [];

    const url = pathParams.reduce(
      (u, param, i) => args[i] !== undefined ? u.replace(param, encodeURIComponent(String(args[i]))) : u,
      `http://${endpoint.host}:${endpoint.port}${path}`
    );

    const options: RequestInit = {
      method: method || 'GET',
      headers: { 'Content-Type': 'application/json' },
      ...((method === 'POST' || method === 'PUT') && args.length > pathParams.length
        ? { body: JSON.stringify(args[pathParams.length]) }
        : {})
    };

    const response = await fetch(url, options);
    return response.json();
  };
};
