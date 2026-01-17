import { Construct } from 'constructs';

export type FunctionHandler = (...args: any[]) => any;

/**
 * Represents a serverless function or handler in the architecture
 */
export class Function extends Construct {
  public readonly handler: FunctionHandler;

  constructor(scope: Construct, id: string, handler: FunctionHandler) {
    super(scope, id);
    this.handler = handler;
  }

  public invoke(...args: any[]): any {
    return this.handler(...args);
  }
}
