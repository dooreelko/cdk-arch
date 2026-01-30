import { Construct } from 'constructs';

export type FunctionHandler<TArgs extends any[] = any[], TReturn = any> = (...args: TArgs) => Promise<TReturn>;

/**
 * Represents a serverless function or handler in the architecture.
 * Generic over argument types (TArgs) and return type (TReturn).
 */
export class Function<TArgs extends any[] = any[], TReturn = any> extends Construct {
  public readonly handler: FunctionHandler<TArgs, TReturn>;
  private _overload?: FunctionHandler<TArgs, TReturn>;

  constructor(scope: Construct, id: string, handler: FunctionHandler<TArgs, TReturn>) {
    super(scope, id);
    this.handler = handler;
  }

  /**
   * Override the function's implementation at runtime.
   * Used for replacing in-memory implementations with storage adapters or HTTP calls.
   */
  overload(handler: FunctionHandler<TArgs, TReturn>): void {
    this._overload = handler;
  }

  hasOverload(): boolean {
    return this._overload !== undefined;
  }

  public invoke(...args: TArgs): Promise<TReturn> {
    const fn = this._overload ?? this.handler;
    return Promise.resolve(fn(...args));
  }
}

/**
 * A placeholder function that must be overloaded before use.
 * Use this when defining an API contract without providing an implementation.
 */
export class TBDFunction<TArgs extends any[] = any[], TReturn = any> extends Function<TArgs, TReturn> {
  constructor(scope: Construct, id: string) {
    super(scope, id, (() => Promise.reject(new Error(`Function '${id}' is not implemented. Provide an overload before invoking.`))) as FunctionHandler<TArgs, TReturn>);
  }
}
