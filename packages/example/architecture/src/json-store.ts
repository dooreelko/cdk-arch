import { Construct, ApiContainer, TBDFunction } from 'cdk-arch';

/**
 * Represents a JSON document store in the architecture.
 * Extends ApiContainer to expose store and get as HTTP APIs.
 * Implementations must be provided via overloads.
 *
 * @typeParam TDoc - The type of documents stored in this store
 */
export class JsonStore<TDoc = any> extends ApiContainer {
  public readonly storeFunction: TBDFunction<[string, TDoc], { success: boolean }>;
  public readonly getFunction: TBDFunction<[string], TDoc[]>;

  constructor(scope: Construct, id: string) {
    super(scope, id, {});

    this.storeFunction = new TBDFunction(this, 'store-handler');
    this.getFunction = new TBDFunction(this, 'get-handler');

    this.addRoute('POST /v1/api/store/{collection}', this.storeFunction);
    this.addRoute('GET /v1/api/get/{collection}', this.getFunction);
  }

  store(collection: string, document: TDoc): Promise<{ success: boolean }> {
    return this.storeFunction.invoke(collection, document);
  }

  get(collection: string): Promise<TDoc[]> {
    return this.getFunction.invoke(collection);
  }
}
