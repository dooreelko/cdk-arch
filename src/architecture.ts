import { Construct } from 'constructs';

/**
 * The root construct that represents an entire architecture
 */
export class Architecture extends Construct {
  constructor(id: string = 'architecture') {
    super(undefined as any, id);
  }

  public synth(): ArchitectureDefinition {
    return {
      id: this.node.id,
      components: this.node.children.map(c => this.synthComponent(c as Construct))
    };
  }

  private synthComponent(construct: Construct): ComponentDefinition {
    return {
      id: construct.node.id,
      path: construct.node.path,
      type: construct.constructor.name
    };
  }
}

export interface ArchitectureDefinition {
  id: string;
  components: ComponentDefinition[];
}

export interface ComponentDefinition {
  id: string;
  path: string;
  type: string;
}
