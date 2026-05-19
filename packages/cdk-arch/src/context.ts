import { AsyncLocalStorage } from 'async_hooks';

export type RuntimeContext = Record<string, unknown>;

const als = new AsyncLocalStorage<RuntimeContext>();

export function getCurrentContext(): RuntimeContext | undefined {
  return als.getStore();
}

export function runWithContext<T>(ctx: RuntimeContext, fn: () => Promise<T>): Promise<T> {
  return als.run(ctx, fn);
}
