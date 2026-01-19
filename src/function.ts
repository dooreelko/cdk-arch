import { Construct } from 'constructs';

export type FunctionHandler = (...args: any[]) => any;

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

  public invoke(...args: any[]): any {
    const fn = this._overload ?? this.handler;
    return fn(...args);
  }
}
