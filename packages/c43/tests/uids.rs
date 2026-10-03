use std::collections::HashSet;
use std::fs;
use std::path::Path;
use tempfile::tempdir;
use c43::cmd::{component, container};
use c43::model::C4Document;

fn package(root: &Path, name: &str, file: &str, source: &str) {
    let dir = root.join("packages").join(name);
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("package.json"), format!(r#"{{"name": "{}", "version": "1.0.0"}}"#, name)).unwrap();
    fs::write(dir.join("src").join(file), source).unwrap();
}

fn has(doc: &C4Document, start: &str, is: &str, end: &str) -> bool {
    doc.relations.iter().any(|r| r.start == start && r.is == is && r.end == end)
}

fn assert_well_formed(doc: &C4Document) {
    let uids: HashSet<&str> = doc.nodes.iter().map(|n| n.uid.as_str()).collect();
    for r in &doc.relations {
        assert_ne!(r.start, r.end, "self-loop: {} {} {}", r.start, r.is, r.end);
        assert!(uids.contains(r.start.as_str()) && uids.contains(r.end.as_str()), "dangling: {} {} {}", r.start, r.is, r.end);
    }
}

/// Architectures and their containers sharing an id, and same ids in different
/// Architectures, get distinct uids.
#[test]
fn same_ids_do_not_collide() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    package(root, "chat-arch", "arch.ts", r#"
        import { Architecture, ApiContainer, Function } from '@arinoto/cdk-arch';
        export const arch = new Architecture('chat');
        export const send = new Function(arch, 'send', async () => 1);
        export const api = new ApiContainer(arch, 'chat', { send: { path: 'POST /send', handler: send } });
    "#);
    package(root, "mail-arch", "arch.ts", r#"
        import { Architecture, ApiContainer, Function } from '@arinoto/cdk-arch';
        export const arch = new Architecture('mail');
        export const send = new Function(arch, 'send', async () => 1);
        export const api = new ApiContainer(arch, 'chat', { send: { path: 'POST /send', handler: send } });
    "#);

    let doc = container::run(root);
    assert_well_formed(&doc);
    assert!(has(&doc, "backend:chat", "contains", "chat/chat"));
    assert!(has(&doc, "backend:mail", "contains", "mail/chat"));

    let doc = component::run(root, None);
    assert_well_formed(&doc);
    assert!(has(&doc, "chat/chat", "handles", "chat/send"));
    assert!(has(&doc, "mail/chat", "handles", "mail/send"));
    assert!(!has(&doc, "chat/chat", "handles", "mail/send"));
}

/// Subclasses of cdk-arch classes are recognised through inheritance, imported aliases
/// included, and constructs/routes created inside a class are instantiated per instance.
#[test]
fn kinds_by_inheritance_and_class_templates() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    package(root, "base", "index.ts", r#"
        import { Function as CdkFunction, ApiContainer, TBDFunction } from '@arinoto/cdk-arch';
        export class ObservedFunction extends CdkFunction {}
        export class Store extends ApiContainer {
            constructor(scope, id) {
                super(scope, id);
                this.get = new TBDFunction(this, 'get');
                this.addRoute('get', 'GET /get', this.get);
            }
        }
    "#);
    package(root, "app-arch", "arch.ts", r#"
        import { Architecture } from '@arinoto/cdk-arch';
        import { ObservedFunction as Function, Store } from 'base';
        export const arch = new Architecture('app');
        export const store = new Store(arch, 'store');
        export const read = new Function(arch, 'read', async () => store.get.invoke());
        export const oops = new Error('not a construct');
    "#);

    let doc = component::run(root, None);
    assert_well_formed(&doc);
    let read = doc.nodes.iter().find(|n| n.uid == "app/read").expect("app/read missing");
    assert_eq!(read.node_type, "function"); // class name as written (alias)
    assert_eq!(read.attributes.kind.as_deref(), Some("function"));
    assert!(has(&doc, "app/store", "handles", "app/store/get"));
    assert!(has(&doc, "app/read", "uses", "app/store/get"));
    assert!(!doc.nodes.iter().any(|n| n.name == "not a construct"));
}

/// Functions calling functions of another Architecture yield Fn->Fn `uses` in the component
/// view and collapsed container-level `uses` in the container view. Same-named variables in
/// unrelated packages are not confused.
#[test]
fn cross_architecture_uses() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    package(root, "llm-arch", "arch.ts", r#"
        import { Architecture, ApiContainer, Function } from '@arinoto/cdk-arch';
        export const arch = new Architecture('llm');
        export const complete = new Function(arch, 'complete', async () => 1);
        export const llm = new ApiContainer(arch, 'llm', { complete: { path: 'POST /c', handler: complete } });
    "#);
    package(root, "memory-arch", "arch.ts", r#"
        import { Architecture, ApiContainer, Function } from '@arinoto/cdk-arch';
        import { complete } from 'llm-arch';
        export const arch = new Architecture('memory');
        export const recall = new Function(arch, 'recall', async () => complete.invoke());
        export const worker = new Function(arch, 'worker', async () => complete.invoke());
        export const memory = new ApiContainer(arch, 'memory', { recall: { path: 'GET /r', handler: recall } });
    "#);

    let doc = component::run(root, None);
    assert_well_formed(&doc);
    assert!(has(&doc, "memory/recall", "uses", "llm/complete"));

    let doc = container::run(root);
    assert_well_formed(&doc);
    assert!(has(&doc, "memory/memory", "uses", "llm/llm"));
    // unrouted function acts through its Architecture
    assert!(has(&doc, "backend:memory", "uses", "llm/llm"));
}
