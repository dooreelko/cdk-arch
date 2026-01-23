import { Construct } from 'constructs';

export type FunctionHandler = (...args: any[]) => Promise<any>;

/**
 * Represents a serverless function or handler in the architecture
 */
export class Function extends Construct {
  public readonly handler: FunctionHandler;
  private _overload?: FunctionHandler;

  constructor(scope: Construct, id: string, handler: FunctionHandler) {
    super(scope, id);
    this.handler = handler;
  }

  /**
   * Override the function's implementation at runtime.
   * Used for replacing in-memory implementations with storage adapters or HTTP calls.
   */
  overload(handler: FunctionHandler): void {
    this._overload = handler;
  }

  hasOverload(): boolean {
    return this._overload !== undefined;
  }

  public invoke(...args: any[]): Promise<any> {
    const fn = this._overload ?? this.handler;
    return Promise.resolve(fn(...args));
  }
}

/**
 * A placeholder function that must be overloaded before use.
 * Use this when defining an API contract without providing an implementation.
 */
export class TBDFunction extends Function {
  constructor(scope: Construct, id: string) {
    super(scope, id, () => {
      return Promise.reject(new Error(`Function '${id}' is not implemented. Provide an overload before invoking.`));
    });
  }
}
