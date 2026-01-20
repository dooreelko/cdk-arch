import { Construct, ApiContainer, TBDFunction } from 'cdk-arch';

/**
 * Represents a JSON document store in the architecture.
 * Extends ApiContainer to expose store and get as HTTP APIs.
 * Implementations must be provided via overloads.
 */
export class JsonStore extends ApiContainer {
  public readonly storeFunction: TBDFunction;
  public readonly getFunction: TBDFunction;

  constructor(scope: Construct, id: string) {
    super(scope, id, {});

    this.storeFunction = new TBDFunction(this, 'store-handler');
    this.getFunction = new TBDFunction(this, 'get-handler');

    this.addRoute('POST /store/{collection}', this.storeFunction);
    this.addRoute('GET /get/{collection}', this.getFunction);
  }

  store(collection: string, document: any): { success: boolean } {
    return this.storeFunction.invoke(collection, document);
  }

  get(collection: string): any[] {
    return this.getFunction.invoke(collection);
  }
}
