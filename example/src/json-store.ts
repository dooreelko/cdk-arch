import { Construct, ApiContainer, Function } from 'cdk-arch';

/**
 * Represents a JSON document store in the architecture.
 * Extends ApiContainer to expose store and get as HTTP APIs.
 */
export class JsonStore extends ApiContainer {
  private data: Map<string, any[]> = new Map();
  public readonly storeFunction: Function;
  public readonly getFunction: Function;

  constructor(scope: Construct, id: string) {
    super(scope, id, {});

    // Create functions for store and get operations
    this.storeFunction = new Function(this, 'store-handler', (collection: string, document: any) => {
      if (!this.data.has(collection)) {
        this.data.set(collection, []);
      }
      this.data.get(collection)!.push(document);
      return { success: true };
    });

    this.getFunction = new Function(this, 'get-handler', (collection: string) => {
      return this.data.get(collection) || [];
    });

    // Register as routes
    this.addRoute('POST /store/{collection}', this.storeFunction);
    this.addRoute('GET /get/{collection}', this.getFunction);
  }

  // Convenience methods that call the functions directly (for in-process use)
  public store(collection: string, document: any): { success: boolean } {
    return this.storeFunction.invoke(collection, document);
  }

  public get(collection: string): any[] {
    return this.getFunction.invoke(collection);
  }
}
