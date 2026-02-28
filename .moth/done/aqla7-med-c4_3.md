# c4 extractor

under packages create a new directory 'c43', inside initialize a rust project for a cli that has subcommands for
- `system`. scans an npm muti-workspace repository and using https://rustdoc.swc.rs/swc_ecma_parser/ idenitifies high level Architecture systems and 
generates a c4 system description document in JSON (e.g. not using c4 DSL)
- `container`. similarly uses typescript parser to identify individual components of an Architecture, generating a c4 container description in JSON (again, no DSL)
- `component`. scans an individual typescript project and using https://rustdoc.swc.rs/swc_ecma_parser/ identifies Architecture instances and its components, 
eventually generating a c4 component description document in json form. no c4 dsl. can take name of a container to narrow down the output
- `deployment`. for a given ts project with Archtiecture, component and an infra project, using swc parser, identify architectureBinding.bind calls and components that are bound and generate a c4 deployment description document in json (again no c4 dsl)

## json description common parts

all json documents must follow same format in two sections:
`nodes` - a flat array of node objects containing their direct attributes (uid, name, role, etc)
`relations` - a flat array of relation objects in form of {start: uid, is: <kind as a string>, end: uid, attributes: { optional stuff specific to the relation }}
e.g. { start: "root", is: "parent of", end: "child0" }, { start: "root", is: "parent of", end: "child1", attributes: { after: "child0" } }
