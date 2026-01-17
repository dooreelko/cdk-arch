import { Construct } from 'constructs';

/**
 * Represents a JSON document store in the architecture
 */
export class JsonStore extends Construct {
  private data: Map<string, any[]> = new Map();

  constructor(scope: Construct, id: string) {
    super(scope, id);
  }

  public store(collection: string, document: any): void {
    if (!this.data.has(collection)) {
      this.data.set(collection, []);
    }
    this.data.get(collection)!.push(document);
  }

  public get(collection: string): any[] {
    return this.data.get(collection) || [];
  }
}
