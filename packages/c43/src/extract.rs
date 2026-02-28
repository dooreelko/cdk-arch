use std::path::Path;
use swc_ecma_ast::*;

use crate::parse::parse_ts_file;

/// A construct instantiation found in TS source
#[derive(Debug, Clone)]
pub struct ConstructInstance {
    pub class_name: String,
    pub id: String,
    pub scope_var: Option<String>,
    pub var_name: Option<String>,
    pub file: String,
}

/// A route entry: { path: 'GET /v1/api/hello/{name}', handler: someVar }
#[derive(Debug, Clone)]
pub struct RouteEntry {
    #[allow(dead_code)]
    pub name: String,
    #[allow(dead_code)]
    pub path: String,
    pub handler_var: String,
}

/// An architectureBinding.bind() call
#[derive(Debug, Clone)]
pub struct BindCall {
    pub component_var: String,
    pub base_url: Option<String>,
    pub overload_keys: Vec<String>,
    pub file: String,
}

/// An import mapping: local_name -> (module_source, imported_name)
#[derive(Debug, Clone)]
pub struct ImportInfo {
    pub local_name: String,
    pub source: String,
    #[allow(dead_code)]
    pub imported_name: Option<String>,
}

/// A re-export: export { name } from 'source'
#[derive(Debug, Clone)]
pub struct ReExport {
    pub local_name: String,
    pub source: String,
}

/// All extracted info from a single file
#[derive(Debug, Clone, Default)]
pub struct FileExtracts {
    pub constructs: Vec<ConstructInstance>,
    pub routes: Vec<(String, Vec<RouteEntry>)>,
    pub binds: Vec<BindCall>,
    pub imports: Vec<ImportInfo>,
    pub var_assignments: Vec<(String, String)>,
    /// Variable names that are exported (via `export const x = ...` or `export { x }`)
    pub exported_names: Vec<String>,
    /// Re-exports: `export { x } from 'source'` or `export * from 'source'`
    pub reexports: Vec<ReExport>,
}

pub fn extract_from_file(path: &Path) -> FileExtracts {
    let module = match parse_ts_file(path) {
        Some(m) => m,
        None => return FileExtracts::default(),
    };
    let file = path.to_string_lossy().to_string();
    extract_from_module(&module, &file)
}

pub fn extract_from_module(module: &Module, file: &str) -> FileExtracts {
    let mut result = FileExtracts::default();

    for item in &module.body {
        match item {
            ModuleItem::ModuleDecl(decl) => extract_from_module_decl(decl, file, &mut result),
            ModuleItem::Stmt(stmt) => extract_from_stmt(stmt, file, &mut result),
        }
    }

    result
}

fn extract_from_module_decl(decl: &ModuleDecl, file: &str, result: &mut FileExtracts) {
    match decl {
        ModuleDecl::Import(import) => {
            if import.type_only {
                return;
            }
            let source = str_value(&import.src);
            for spec in &import.specifiers {
                match spec {
                    ImportSpecifier::Named(named) => {
                        let local = named.local.sym.to_string();
                        let imported = named.imported.as_ref().map(|n| match n {
                            ModuleExportName::Ident(id) => id.sym.to_string(),
                            ModuleExportName::Str(s) => str_value(s),
                        });
                        result.imports.push(ImportInfo {
                            local_name: local,
                            source: source.clone(),
                            imported_name: imported,
                        });
                    }
                    ImportSpecifier::Default(def) => {
                        result.imports.push(ImportInfo {
                            local_name: def.local.sym.to_string(),
                            source: source.clone(),
                            imported_name: Some("default".to_string()),
                        });
                    }
                    ImportSpecifier::Namespace(ns) => {
                        result.imports.push(ImportInfo {
                            local_name: ns.local.sym.to_string(),
                            source: source.clone(),
                            imported_name: Some("*".to_string()),
                        });
                    }
                }
            }
        }
        ModuleDecl::ExportDecl(export) => {
            // Track exported variable names
            if let Decl::Var(var_decl) = &export.decl {
                for declarator in &var_decl.decls {
                    if let Some(name) = pat_to_name(&declarator.name) {
                        result.exported_names.push(name);
                    }
                }
            }
            if let Decl::Class(class_decl) = &export.decl {
                result.exported_names.push(class_decl.ident.sym.to_string());
            }
            extract_from_decl_inner(&export.decl, file, result);
        }
        ModuleDecl::ExportNamed(named) => {
            if let Some(src) = &named.src {
                // Re-export: export { x, y } from 'source'
                let source = str_value(src);
                for spec in &named.specifiers {
                    if let ExportSpecifier::Named(n) = spec {
                        let name = match &n.exported {
                            Some(ModuleExportName::Ident(id)) => id.sym.to_string(),
                            Some(ModuleExportName::Str(s)) => str_value(s),
                            None => match &n.orig {
                                ModuleExportName::Ident(id) => id.sym.to_string(),
                                ModuleExportName::Str(s) => str_value(s),
                            },
                        };
                        result.reexports.push(ReExport {
                            local_name: name,
                            source: source.clone(),
                        });
                    }
                }
            } else {
                // Local export: export { x, y }
                for spec in &named.specifiers {
                    if let ExportSpecifier::Named(n) = spec {
                        let name = match &n.orig {
                            ModuleExportName::Ident(id) => id.sym.to_string(),
                            ModuleExportName::Str(s) => str_value(s),
                        };
                        result.exported_names.push(name);
                    }
                }
            }
        }
        ModuleDecl::ExportAll(export_all) => {
            result.reexports.push(ReExport {
                local_name: "*".to_string(),
                source: str_value(&export_all.src),
            });
        }
        _ => {}
    }
}

fn extract_from_stmt(stmt: &Stmt, file: &str, result: &mut FileExtracts) {
    match stmt {
        Stmt::Decl(decl) => extract_from_decl_inner(decl, file, result),
        Stmt::Expr(expr_stmt) => {
            extract_bind_calls(&expr_stmt.expr, file, result);
        }
        _ => {}
    }
}

fn extract_from_decl_inner(decl: &Decl, file: &str, result: &mut FileExtracts) {
    match decl {
        Decl::Var(var_decl) => {
            for declarator in &var_decl.decls {
                let var_name = pat_to_name(&declarator.name);
                if let Some(init) = &declarator.init {
                    // Check for new expressions
                    if let Some(ci) = extract_new_expr(init, file) {
                        let mut ci = ci;
                        ci.var_name = var_name.clone();
                        // Check for routes in ApiContainer
                        if ci.class_name == "ApiContainer" {
                            if let Some(routes) = extract_api_routes(init) {
                                result.routes.push((ci.id.clone(), routes));
                            }
                        }
                        result.constructs.push(ci);
                    }
                    // Track variable assignments for bind resolution
                    if let Some(vn) = &var_name {
                        if let Some(str_val) = expr_to_string(init) {
                            result.var_assignments.push((vn.clone(), str_val));
                        }
                    }
                    // Check for bind calls in init expressions
                    extract_bind_calls(init, file, result);
                }
            }
        }
        Decl::Class(class_decl) => {
            extract_from_class(&class_decl.class, file, result);
        }
        _ => {}
    }
}

fn extract_from_class(class: &Class, file: &str, result: &mut FileExtracts) {
    for member in &class.body {
        if let ClassMember::Constructor(ctor) = member {
            if let Some(body) = &ctor.body {
                for stmt in &body.stmts {
                    extract_from_class_stmt(stmt, file, result);
                }
            }
        }
    }
}

fn extract_from_class_stmt(stmt: &Stmt, file: &str, result: &mut FileExtracts) {
    match stmt {
        Stmt::Expr(expr_stmt) => {
            // Handle this.field = new SomeClass(this, 'id')
            if let Expr::Assign(assign) = expr_stmt.expr.as_ref() {
                if let Some(ci) = extract_new_expr(&assign.right, file) {
                    result.constructs.push(ci);
                }
                // Handle this.addRoute('name', 'path', this.field)
                extract_add_route_calls(&assign.right, result);
            }
            // Handle direct method calls like this.addRoute(...)
            if let Expr::Call(call) = expr_stmt.expr.as_ref() {
                extract_add_route_from_call(call, result);
            }
            extract_bind_calls(&expr_stmt.expr, file, result);
        }
        Stmt::Decl(decl) => extract_from_decl_inner(decl, file, result),
        _ => {}
    }
}

fn extract_add_route_calls(expr: &Expr, result: &mut FileExtracts) {
    if let Expr::Call(call) = expr {
        extract_add_route_from_call(call, result);
    }
}

fn extract_add_route_from_call(call: &CallExpr, result: &mut FileExtracts) {
    if let Callee::Expr(callee) = &call.callee {
        if let Expr::Member(member) = callee.as_ref() {
            if let MemberProp::Ident(prop) = &member.prop {
                if prop.sym.as_ref() == "addRoute" && call.args.len() >= 3 {
                    let name = call.args.get(0).and_then(|a| expr_to_string(&a.expr));
                    let path = call.args.get(1).and_then(|a| expr_to_string(&a.expr));
                    let handler_var = call.args.get(2).and_then(|a| {
                        // Could be this.someField or a direct ident
                        match a.expr.as_ref() {
                            Expr::Member(m) => {
                                if let MemberProp::Ident(p) = &m.prop {
                                    Some(p.sym.to_string())
                                } else {
                                    None
                                }
                            }
                            Expr::Ident(id) => Some(id.sym.to_string()),
                            _ => None,
                        }
                    });
                    // We need to find the container id — for class-internal routes we need
                    // to figure out which construct this belongs to. For now, collect them
                    // and associate later.
                    if let (Some(name), Some(path)) = (name, path) {
                        // Use a sentinel container id that we'll resolve later
                        let handler = handler_var.unwrap_or_default();
                        result.routes.push((
                            "__class__".to_string(),
                            vec![RouteEntry {
                                name,
                                path,
                                handler_var: handler,
                            }],
                        ));
                    }
                }
            }
        }
    }
}

fn extract_new_expr(expr: &Expr, file: &str) -> Option<ConstructInstance> {
    let new_expr = match expr {
        Expr::New(n) => n,
        Expr::TsAs(ts_as) => return extract_new_expr(&ts_as.expr, file),
        Expr::Paren(paren) => return extract_new_expr(&paren.expr, file),
        _ => return None,
    };

    let class_name = expr_to_ident_name(&new_expr.callee)?;
    let args = new_expr.args.as_ref()?;

    // Pattern 1: new Architecture('id') — single string arg
    if class_name == "Architecture" {
        let id = args
            .first()
            .and_then(|a| expr_to_string(&a.expr))
            .unwrap_or_else(|| "architecture".to_string());
        return Some(ConstructInstance {
            class_name,
            id,
            scope_var: None,
            var_name: None,
            file: file.to_string(),
        });
    }

    // Pattern 2: new SomeClass(scope, 'id', ...) — scope + string id + optional rest
    if args.len() >= 2 {
        let scope_var = expr_to_ident_name(&args[0].expr);
        let id = expr_to_string(&args[1].expr);
        if let Some(id) = id {
            return Some(ConstructInstance {
                class_name,
                id,
                scope_var,
                var_name: None,
                file: file.to_string(),
            });
        }
    }

    None
}

fn extract_api_routes(expr: &Expr) -> Option<Vec<RouteEntry>> {
    let new_expr = match expr {
        Expr::New(n) => n,
        _ => return None,
    };
    let args = new_expr.args.as_ref()?;
    // Third argument should be the routes object
    let routes_arg = args.get(2)?;
    extract_routes_from_obj(&routes_arg.expr)
}

fn extract_routes_from_obj(expr: &Expr) -> Option<Vec<RouteEntry>> {
    let obj = match expr {
        Expr::Object(o) => o,
        _ => return None,
    };

    let mut routes = Vec::new();
    for prop in &obj.props {
        if let PropOrSpread::Prop(prop) = prop {
            if let Prop::KeyValue(kv) = prop.as_ref() {
                let name = prop_name_to_string(&kv.key)?;
                // Value should be { path: 'GET /...', handler: someVar }
                if let Expr::Object(route_obj) = kv.value.as_ref() {
                    let mut path = None;
                    let mut handler_var = None;
                    for route_prop in &route_obj.props {
                        if let PropOrSpread::Prop(rp) = route_prop {
                            if let Prop::KeyValue(rkv) = rp.as_ref() {
                                let key = prop_name_to_string(&rkv.key);
                                match key.as_deref() {
                                    Some("path") => path = expr_to_string(&rkv.value),
                                    Some("handler") => {
                                        handler_var = expr_to_ident_name(&rkv.value)
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    if let (Some(path), Some(handler_var)) = (path, handler_var) {
                        routes.push(RouteEntry {
                            name,
                            path,
                            handler_var,
                        });
                    }
                }
            }
        }
    }
    Some(routes)
}

fn extract_bind_calls(expr: &Expr, file: &str, result: &mut FileExtracts) {
    match expr {
        Expr::Call(call) => {
            if let Callee::Expr(callee) = &call.callee {
                // Check for architectureBinding.bind(component, options)
                if is_architecture_binding_bind(callee) {
                    if let Some(bind) = parse_bind_call(&call.args, file) {
                        result.binds.push(bind);
                    }
                }
            }
            // Recurse into args
            for arg in &call.args {
                extract_bind_calls(&arg.expr, file, result);
            }
        }
        Expr::Seq(seq) => {
            for expr in &seq.exprs {
                extract_bind_calls(expr, file, result);
            }
        }
        Expr::Assign(assign) => {
            extract_bind_calls(&assign.right, file, result);
        }
        _ => {}
    }
}

fn is_architecture_binding_bind(expr: &Expr) -> bool {
    if let Expr::Member(member) = expr {
        if let MemberProp::Ident(prop) = &member.prop {
            if prop.sym.as_ref() == "bind" {
                if let Expr::Ident(obj) = member.obj.as_ref() {
                    return obj.sym.as_ref() == "architectureBinding";
                }
            }
        }
    }
    false
}

fn parse_bind_call(args: &[ExprOrSpread], file: &str) -> Option<BindCall> {
    let component_var = args.first().and_then(|a| expr_to_ident_name(&a.expr))?;
    let options = args.get(1);

    let mut base_url = None;
    let mut overload_keys = Vec::new();

    if let Some(opts) = options {
        if let Expr::Object(obj) = opts.expr.as_ref() {
            for prop in &obj.props {
                if let PropOrSpread::Prop(p) = prop {
                    if let Prop::KeyValue(kv) = p.as_ref() {
                        let key = prop_name_to_string(&kv.key);
                        match key.as_deref() {
                            Some("baseUrl") => {
                                base_url = expr_to_string_or_template(&kv.value);
                            }
                            Some("overloads") => {
                                overload_keys = extract_object_keys(&kv.value);
                            }
                            _ => {}
                        }
                    }
                    if let Prop::Shorthand(ident) = p.as_ref() {
                        // Handle spread shorthand like { ...jsonStoreEndpoint }
                        let _name = ident.sym.to_string();
                    }
                }
                if let PropOrSpread::Spread(spread) = prop {
                    // Spread from another variable — try to extract base_url
                    if let Expr::Ident(_) = spread.expr.as_ref() {
                        // Can't resolve at static analysis time
                    }
                }
            }
        }
    }

    Some(BindCall {
        component_var,
        base_url,
        overload_keys,
        file: file.to_string(),
    })
}

fn extract_object_keys(expr: &Expr) -> Vec<String> {
    match expr {
        Expr::Object(obj) => obj
            .props
            .iter()
            .filter_map(|p| match p {
                PropOrSpread::Prop(prop) => match prop.as_ref() {
                    Prop::KeyValue(kv) => prop_name_to_string(&kv.key),
                    Prop::Shorthand(ident) => Some(ident.sym.to_string()),
                    Prop::Method(m) => prop_name_to_string(&m.key),
                    _ => None,
                },
                _ => None,
            })
            .collect(),
        Expr::Ident(_) => {
            // Variable reference — can't resolve statically
            Vec::new()
        }
        _ => Vec::new(),
    }
}

// Helper functions

fn str_value(s: &Str) -> String {
    s.value.to_string_lossy().into_owned()
}

fn expr_to_ident_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Ident(ident) => Some(ident.sym.to_string()),
        _ => None,
    }
}

fn expr_to_string(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Lit(Lit::Str(s)) => Some(str_value(s)),
        Expr::Tpl(tpl) if tpl.exprs.is_empty() => {
            // Template literal with no expressions, just quasis
            tpl.quasis.first().map(|q| q.raw.to_string())
        }
        _ => None,
    }
}

fn expr_to_string_or_template(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Lit(Lit::Str(s)) => Some(str_value(s)),
        Expr::Tpl(tpl) => {
            // Reconstruct template literal as pattern
            let mut parts = Vec::new();
            for (i, quasi) in tpl.quasis.iter().enumerate() {
                parts.push(quasi.raw.to_string());
                if i < tpl.exprs.len() {
                    if let Some(name) = expr_to_ident_name(&tpl.exprs[i]) {
                        parts.push(format!("${{{}}}", name));
                    } else {
                        parts.push("${...}".to_string());
                    }
                }
            }
            Some(parts.join(""))
        }
        _ => None,
    }
}

fn pat_to_name(pat: &Pat) -> Option<String> {
    match pat {
        Pat::Ident(binding) => Some(binding.id.sym.to_string()),
        _ => None,
    }
}

fn prop_name_to_string(name: &PropName) -> Option<String> {
    match name {
        PropName::Ident(ident) => Some(ident.sym.to_string()),
        PropName::Str(s) => Some(str_value(s)),
        _ => None,
    }
}
