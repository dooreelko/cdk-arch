use std::path::Path;
use swc_ecma_ast::*;

use crate::parse::parse_ts_file;

/// What a construct is, by its cdk-arch base class (resolved through `extends` chains
/// and import aliases after scanning; see `analysis::resolve_construct_kinds`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConstructKind {
    /// Not resolved yet (fresh from the extractor)
    #[default]
    Unresolved,
    /// The class does not derive from a cdk-arch Construct
    NotConstruct,
    Architecture,
    ApiContainer,
    /// Function or TBDFunction
    Function,
    /// Any other Construct (McpContainer, ...)
    Construct,
}

impl ConstructKind {
    pub fn is_function(self) -> bool {
        self == ConstructKind::Function
    }
    pub fn is_architecture(self) -> bool {
        self == ConstructKind::Architecture
    }
    pub fn as_str(self) -> &'static str {
        match self {
            ConstructKind::Unresolved => "unresolved",
            ConstructKind::NotConstruct => "none",
            ConstructKind::Architecture => "architecture",
            ConstructKind::ApiContainer => "apicontainer",
            ConstructKind::Function => "function",
            ConstructKind::Construct => "construct",
        }
    }
}

/// A construct instantiation found in TS source
#[derive(Debug, Clone)]
pub struct ConstructInstance {
    /// Class name as written at the `new` site (display type)
    pub class_name: String,
    pub kind: ConstructKind,
    pub id: String,
    pub scope_var: Option<String>,
    pub var_name: Option<String>,
    pub file: String,
    /// Variables used as objects in method calls within the handler body (function-bodied constructs)
    pub called_vars: Vec<String>,
}

/// A class declaration: its base class and what its constructor creates.
/// Constructs and routes inside are templates, instantiated for every instance of the class
/// (and of its subclasses). `this.<field>` handler references are kept as `this.<field>`.
#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub name: String,
    pub extends: Option<String>,
    /// (field name, construct created with `this` as scope)
    pub constructs: Vec<(String, ConstructInstance)>,
    pub routes: Vec<RouteEntry>,
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
// Updated BindCall struct with has_overloads flag
pub struct BindCall {
    pub component_var: String,
    pub base_url: Option<String>,
    pub overload_keys: Vec<String>,
    // true when the "overloads" key is present, regardless of its value
    pub has_overloads: bool,
    pub file: String,
}


fn parse_bind_call(args: &[ExprOrSpread], file: &str) -> Option<BindCall> {
    let component_var = args.first().and_then(|a| expr_to_ident_name(&a.expr))?;
    let options = args.get(1);

    let mut base_url = None;
    let mut overload_keys = Vec::new();
    // Track whether the overloads property existed
    let mut has_overloads = false;

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
                                // Extract overload keys, but also mark that overloads existed
                                overload_keys = extract_object_keys(&kv.value);
                                has_overloads = true;
                            }
                            _ => {}
                        }
                    }
                    if let Prop::Shorthand(ident) = p.as_ref() {
                        // Handle shorthand like { overloads } – we treat it as overloads present
                        if ident.sym == "overloads" {
                            has_overloads = true;
                        }
                    }
                }
                if let PropOrSpread::Spread(_spread) = prop {
                    // We cannot resolve overloads from a spread, ignore for now
                }
            }
        }
    }

    Some(BindCall {
        component_var,
        base_url,
        overload_keys,
        has_overloads,
        file: file.to_string(),
    })
}


/// An import mapping: local_name -> (module_source, imported_name)
#[derive(Debug, Clone)]
pub struct ImportInfo {
    pub local_name: String,
    pub source: String,
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
    pub classes: Vec<ClassInfo>,
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
                        // Constructor route literal (ApiContainer and subclasses; kind checked later)
                        if let Some(routes) = extract_api_routes(init) {
                            if !routes.is_empty() {
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
            extract_from_class(&class_decl.ident.sym, &class_decl.class, file, result);
        }
        _ => {}
    }
}

fn extract_from_class(name: &str, class: &Class, file: &str, result: &mut FileExtracts) {
    let mut info = ClassInfo {
        name: name.to_string(),
        extends: class.super_class.as_deref().and_then(expr_to_ident_name),
        constructs: Vec::new(),
        routes: Vec::new(),
    };
    for member in &class.body {
        match member {
            ClassMember::Constructor(ctor) => {
                if let Some(body) = &ctor.body {
                    for stmt in &body.stmts {
                        extract_from_class_stmt(stmt, file, &mut info, result);
                    }
                }
            }
            // private readonly x = new SomeConstruct(this, 'id')
            ClassMember::ClassProp(prop) => {
                if let (Some(field), Some(value)) = (prop_name_to_string(&prop.key), &prop.value) {
                    if let Some(ci) = extract_new_expr(value, file) {
                        info.constructs.push((field, ci));
                    }
                }
            }
            _ => {}
        }
    }
    result.classes.push(info);
}

fn extract_from_class_stmt(stmt: &Stmt, file: &str, info: &mut ClassInfo, result: &mut FileExtracts) {
    match stmt {
        Stmt::Expr(expr_stmt) => {
            match expr_stmt.expr.as_ref() {
                // this.field = new SomeClass(this, 'id')
                Expr::Assign(assign) => {
                    if let Some(ci) = extract_new_expr(&assign.right, file) {
                        if let Some(field) = this_field_of_target(&assign.left) {
                            info.constructs.push((field, ci));
                        }
                    }
                }
                // this.addRoute('name', 'path', handler)
                Expr::Call(call) => {
                    if let Some(route) = extract_add_route_from_call(call) {
                        info.routes.push(route);
                    }
                }
                _ => {}
            }
            extract_bind_calls(&expr_stmt.expr, file, result);
        }
        Stmt::Decl(decl) => extract_from_decl_inner(decl, file, result),
        _ => {}
    }
}

/// `this.field` as an assignment target -> "field"
fn this_field_of_target(target: &AssignTarget) -> Option<String> {
    if let AssignTarget::Simple(SimpleAssignTarget::Member(m)) = target {
        if let (Expr::This(_), MemberProp::Ident(p)) = (m.obj.as_ref(), &m.prop) {
            return Some(p.sym.to_string());
        }
    }
    None
}

/// `this.addRoute('name', 'path', handler)`. The handler is kept as `this.<field>`
/// or as a plain identifier.
fn extract_add_route_from_call(call: &CallExpr) -> Option<RouteEntry> {
    let Callee::Expr(callee) = &call.callee else { return None };
    let Expr::Member(member) = callee.as_ref() else { return None };
    let MemberProp::Ident(prop) = &member.prop else { return None };
    if prop.sym.as_ref() != "addRoute" || call.args.len() < 3 || !matches!(member.obj.as_ref(), Expr::This(_)) {
        return None;
    }
    let name = expr_to_string(&call.args[0].expr)?;
    let path = expr_to_string(&call.args[1].expr)?;
    let handler_var = match call.args[2].expr.as_ref() {
        Expr::Member(m) => match (m.obj.as_ref(), &m.prop) {
            (Expr::This(_), MemberProp::Ident(p)) => format!("this.{}", p.sym),
            _ => return None,
        },
        Expr::Ident(id) => id.sym.to_string(),
        _ => return None,
    };
    Some(RouteEntry { name, path, handler_var })
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

    // Pattern 1: new Architecture('id') — single string arg (any class; kind decides later)
    if args.len() == 1 {
        let id = expr_to_string(&args[0].expr)?;
        return Some(ConstructInstance {
            class_name,
            kind: ConstructKind::Unresolved,
            id,
            scope_var: None,
            var_name: None,
            file: file.to_string(),
            called_vars: vec![],
        });
    }

    // Pattern 2: new SomeClass(scope, 'id', ...) — scope + string id + optional rest
    if args.len() >= 2 {
        let scope_var = expr_to_ident_name(&args[0].expr);
        let id = expr_to_string(&args[1].expr);
        if let Some(id) = id {
            // For function-bodied constructs, extract variables called in the handler body
            let called_vars = args
                .get(2)
                .map_or_else(Vec::new, |a| collect_handler_called_vars(&a.expr));
            return Some(ConstructInstance {
                class_name,
                kind: ConstructKind::Unresolved,
                id,
                scope_var,
                var_name: None,
                file: file.to_string(),
                called_vars,
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

/// Collect all identifier names used as the object of a method call within a function body.
/// Walks arrow functions and regular function expressions passed as handler args.
fn collect_handler_called_vars(expr: &Expr) -> Vec<String> {
    let mut vars = Vec::new();
    match expr {
        Expr::Arrow(arrow) => match arrow.body.as_ref() {
            BlockStmtOrExpr::BlockStmt(block) => {
                block.stmts.iter().for_each(|s| collect_calls_in_stmt(s, &mut vars));
            }
            BlockStmtOrExpr::Expr(e) => collect_calls_in_expr(e, &mut vars),
        },
        Expr::Fn(fn_expr) => {
            if let Some(body) = &fn_expr.function.body {
                body.stmts.iter().for_each(|s| collect_calls_in_stmt(s, &mut vars));
            }
        }
        _ => {}
    }
    vars.sort();
    vars.dedup();
    vars
}

fn collect_calls_in_expr(expr: &Expr, vars: &mut Vec<String>) {
    match expr {
        Expr::Call(call) => {
            if let Callee::Expr(callee) = &call.callee {
                if let Expr::Member(member) = callee.as_ref() {
                    if let Some(name) = member_path(&member.obj) {
                        if !name.starts_with("this") {
                            vars.push(name);
                        }
                    }
                    collect_calls_in_expr(&member.obj, vars);
                } else {
                    collect_calls_in_expr(callee, vars);
                }
            }
            call.args.iter().for_each(|a| collect_calls_in_expr(&a.expr, vars));
        }
        Expr::Await(a) => collect_calls_in_expr(&a.arg, vars),
        Expr::Paren(p) => collect_calls_in_expr(&p.expr, vars),
        Expr::TsAs(t) => collect_calls_in_expr(&t.expr, vars),
        Expr::TsNonNull(t) => collect_calls_in_expr(&t.expr, vars),
        Expr::Assign(a) => collect_calls_in_expr(&a.right, vars),
        Expr::Bin(b) => {
            collect_calls_in_expr(&b.left, vars);
            collect_calls_in_expr(&b.right, vars);
        }
        Expr::Cond(c) => {
            collect_calls_in_expr(&c.test, vars);
            collect_calls_in_expr(&c.cons, vars);
            collect_calls_in_expr(&c.alt, vars);
        }
        Expr::Seq(s) => s.exprs.iter().for_each(|e| collect_calls_in_expr(e, vars)),
        Expr::Unary(u) => collect_calls_in_expr(&u.arg, vars),
        _ => {}
    }
}

fn collect_calls_in_stmt(stmt: &Stmt, vars: &mut Vec<String>) {
    match stmt {
        Stmt::Expr(e) => collect_calls_in_expr(&e.expr, vars),
        Stmt::Return(r) => {
            if let Some(arg) = &r.arg {
                collect_calls_in_expr(arg, vars);
            }
        }
        Stmt::Decl(d) => {
            if let Decl::Var(var_decl) = d {
                for declarator in &var_decl.decls {
                    if let Some(init) = &declarator.init {
                        collect_calls_in_expr(init, vars);
                    }
                }
            }
        }
        Stmt::Block(b) => b.stmts.iter().for_each(|s| collect_calls_in_stmt(s, vars)),
        Stmt::If(i) => {
            collect_calls_in_expr(&i.test, vars);
            collect_calls_in_stmt(&i.cons, vars);
            if let Some(alt) = &i.alt {
                collect_calls_in_stmt(alt, vars);
            }
        }
        Stmt::Try(t) => {
            t.block.stmts.iter().for_each(|s| collect_calls_in_stmt(s, vars));
            if let Some(handler) = &t.handler {
                handler.body.stmts.iter().for_each(|s| collect_calls_in_stmt(s, vars));
            }
            if let Some(finalizer) = &t.finalizer {
                finalizer.stmts.iter().for_each(|s| collect_calls_in_stmt(s, vars));
            }
        }
        Stmt::Throw(t) => collect_calls_in_expr(&t.arg, vars),
        _ => {}
    }
}

// Helper functions

fn str_value(s: &Str) -> String {
    s.value.to_string_lossy().into_owned()
}

/// `a` or `a.b.c` (identifiers and plain property names only).
fn member_path(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Ident(id) => Some(id.sym.to_string()),
        Expr::Member(m) => match &m.prop {
            MemberProp::Ident(prop) => Some(format!("{}.{}", member_path(&m.obj)?, prop.sym)),
            _ => None,
        },
        _ => None,
    }
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
