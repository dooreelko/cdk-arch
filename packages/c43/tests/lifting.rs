use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;
use c43::cmd::system;

#[test]
fn test_implementation_package_lifting() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    // 1. Create architecture package
    let arch_dir = root.join("packages/my-arch");
    fs::create_dir_all(&arch_dir.join("src")).unwrap();
    fs::write(arch_dir.join("package.json"), r#"{"name": "my-arch", "version": "1.0.0"}"#).unwrap();
    fs::write(arch_dir.join("src/arch.ts"), r#"
        import { Architecture } from '@arinoto/cdk-arch';
        export const myArch = new Architecture('my-dispatcher');
    "#).unwrap();

    // 2. Create dependency package (defining a DataStore)
    let dep_dir = root.join("packages/my-dep");
    fs::create_dir_all(&dep_dir.join("src")).unwrap();
    fs::write(dep_dir.join("package.json"), r#"{"name": "my-dep", "version": "1.0.0"}"#).unwrap();
    fs::write(dep_dir.join("src/store.ts"), r#"
        import { DataStore, Architecture } from '@arinoto/cdk-arch';
        const dummyArch = new Architecture('dummy');
        export const memory = new DataStore(dummyArch, 'my-memory');
    "#).unwrap();

    // 3. Create implementation package (ClientServer)
    let impl_dir = root.join("packages/my-server");
    fs::create_dir_all(&impl_dir.join("src")).unwrap();
    fs::write(impl_dir.join("package.json"), r#"{"name": "my-server", "version": "1.0.0"}"#).unwrap();
    fs::write(impl_dir.join("src/index.ts"), r#"
        import { architectureBinding } from '@arinoto/cdk-arch';
        import { myArch } from 'my-arch';
        import { memory } from 'my-dep';
        
        architectureBinding.bind(memory, { baseUrl: 'http://localhost' });
    "#).unwrap();

    // Run system extraction
    let doc = system::run(root);

    // Verify:
    // - my-arch (Backend: my-dispatcher) should be a node.
    // - dummy (Backend: dummy) should be a node.
    // - my-server should NOT be a node.
    // - my-memory should NOT be a node.
    // - my-dispatcher should HAVE a 'uses' relation to dummy.

    let node_ids: Vec<_> = doc.nodes.iter().map(|n| n.uid.as_str()).collect();
    assert!(node_ids.contains(&"my-dispatcher"), "Architecture node 'my-dispatcher' missing");
    assert!(node_ids.contains(&"dummy"), "Architecture node 'dummy' missing");
    assert!(!node_ids.contains(&"my-server"), "Implementation package should be omitted");
    assert!(!node_ids.contains(&"my-memory"), "Container node should be omitted");

    let has_lifted_relation = doc.relations.iter().any(|r| 
        r.start == "my-dispatcher" && r.is == "uses" && r.end == "dummy"
    );
    assert!(has_lifted_relation, "Relation should be lifted from my-server to my-dispatcher, pointing to dummy architecture ID");
}
