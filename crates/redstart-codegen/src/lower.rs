//! Lowering Redstart handler bodies to AssemblyScript.
//!
//! This is the core of the whole bet: emit the AssemblyScript a careful human
//! would write, so the canonical `graph build` toolchain consumes it unmodified
//! (the eject path). A lightweight type environment — entity field types, ABI
//! event-parameter types, and ABI function return types — is enough to make the
//! footgun-prone lowerings correct:
//!
//! - `BigInt`/`BigDecimal` operators (`+ - * /`) lower to `.plus()`/`.minus()`/
//!   `.times()`/`.div()`, never silent native arithmetic.
//! - `loadOrCreate` lowers to the load → null-check → `new` → init dance, so the
//!   nullable-arithmetic miscompile and the forgotten-init crash cannot occur.
//! - Contract calls return `Result`, lowered to graph-ts `try_*` + `.reverted`.
//!   You cannot touch a reverted call's value because you must `match` it first.
//! - `match` on `Result`/`Option` lowers to the corresponding `.reverted` /
//!   null-check `if`/`else`.
//! - Entities created or mutated are auto-saved (dirty-tracked) at the end of the
//!   scope where they were declared — including inside `match` arms.
//!
//! The environment spans *all* modules, so a handler in one `.red` file can
//! reference an entity declared in another — multi-file is first-class here.

use crate::abi::AbiIndex;
use redstart_parser::ast::{BinOp, Block, Expr, HandlerDecl, MatchArm, Pattern, Stmt, TypeExpr, UnOp};
use redstart_parser::Ident;
use std::collections::HashMap;

/// A resolved Redstart type, used only for choosing correct lowerings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RTy {
    BigInt,
    BigDecimal,
    Bytes,
    Address,
    String,
    Boolean,
    Int,
    /// An entity reference (stored as its id in graph-ts).
    Entity(String),
    /// `Option<T>` — nullable.
    Option(Box<RTy>),
    /// A list `[T]`.
    List(Box<RTy>),
    /// A bound contract instance for ABI `name`.
    Contract(String),
    /// `Result<T, CallRevert>` — the type of every contract call.
    Result(Box<RTy>),
    /// The handler's event object (`event`).
    Event,
    /// `event.params`.
    Params,
    /// `event.block`.
    Block,
    /// `event.transaction`.
    Transaction,
    /// Anything we couldn't resolve.
    Unknown,
}

impl RTy {
    fn is_bigint(&self) -> bool {
        matches!(self, RTy::BigInt)
    }
    fn is_bigdecimal(&self) -> bool {
        matches!(self, RTy::BigDecimal)
    }
}

/// Field types for one entity.
#[derive(Debug, Clone, Default)]
pub struct EntityInfo {
    /// Field name -> resolved type.
    pub fields: HashMap<String, RTy>,
}

/// The static environment shared across all handlers.
pub struct Env<'a> {
    /// Entity name -> field info (aggregated across every module).
    pub entities: HashMap<String, EntityInfo>,
    /// Source/template name -> ABI name.
    pub source_abi: HashMap<String, String>,
    /// ABI access for event-parameter and function-return types.
    pub abis: &'a mut AbiIndex,
}

/// One lexical scope's save bookkeeping. Entities are saved at the end of the
/// scope in which they were *declared*, so a `match`-arm entity is saved inside
/// the arm (where it is in scope), and an outer entity mutated in an arm is
/// saved at the outer scope's end.
#[derive(Default)]
struct Frame {
    /// Entity locals declared in this frame.
    declared: Vec<String>,
    /// Of those, the ones that became dirty (created or mutated).
    dirty: Vec<String>,
}

/// Per-handler mutable scope.
struct Scope {
    /// Local variable -> resolved type (flat; shadowing is rare and tolerated).
    locals: HashMap<String, RTy>,
    /// The handler parameter name (the event binding).
    event_param: String,
    /// The current handler's ABI name (for event param lookup).
    abi: String,
    /// The current event name.
    event: String,
    /// Stack of save frames, one per lexical block.
    frames: Vec<Frame>,
    /// Counter for synthetic temporaries.
    tmp: usize,
    /// Warnings raised during lowering.
    warnings: Vec<String>,
}

impl Scope {
    /// Declare an entity local in the current frame.
    fn declare_entity(&mut self, name: &str, entity: String) {
        self.locals.insert(name.to_string(), RTy::Entity(entity));
        if let Some(f) = self.frames.last_mut() {
            f.declared.push(name.to_string());
        }
    }

    /// Declare a non-entity local (no save tracking).
    fn declare_local(&mut self, name: &str, ty: RTy) {
        self.locals.insert(name.to_string(), ty);
    }

    /// Mark an entity local dirty, attributing it to its declaring frame.
    fn mark_dirty(&mut self, name: &str) {
        for f in self.frames.iter_mut().rev() {
            if f.declared.iter().any(|d| d == name) {
                if !f.dirty.iter().any(|d| d == name) {
                    f.dirty.push(name.to_string());
                }
                return;
            }
        }
    }

    fn fresh(&mut self) -> String {
        let n = self.tmp;
        self.tmp += 1;
        format!("_call{n}")
    }
}

/// Map a Solidity ABI type to a resolved type.
fn sol_to_rty(sol: &str) -> RTy {
    if sol == "address" {
        RTy::Address
    } else if sol == "bool" {
        RTy::Boolean
    } else if sol == "string" {
        RTy::String
    } else if sol.starts_with("uint") || sol.starts_with("int") {
        RTy::BigInt
    } else if sol.starts_with("bytes") {
        RTy::Bytes
    } else {
        RTy::Unknown
    }
}

/// Resolve a syntactic type to a resolved type, given the known entity names.
pub fn resolve_type(ty: &TypeExpr, entities: &HashMap<String, EntityInfo>) -> RTy {
    match ty {
        TypeExpr::List { elem, .. } => RTy::List(Box::new(resolve_type(elem, entities))),
        TypeExpr::Generic { base, args, .. } => {
            let name = base.simple_name().unwrap_or("");
            match name {
                "Option" => RTy::Option(Box::new(
                    args.first().map_or(RTy::Unknown, |t| resolve_type(t, entities)),
                )),
                "Id" => args.first().map_or(RTy::Bytes, |t| resolve_type(t, entities)),
                "List" => RTy::List(Box::new(
                    args.first().map_or(RTy::Unknown, |t| resolve_type(t, entities)),
                )),
                _ => RTy::Unknown,
            }
        }
        TypeExpr::Path { .. } => match ty.simple_name().unwrap_or("") {
            "BigInt" => RTy::BigInt,
            "BigDecimal" => RTy::BigDecimal,
            "Bytes" => RTy::Bytes,
            "Address" => RTy::Address,
            "String" => RTy::String,
            "Bool" => RTy::Boolean,
            "Int" => RTy::Int,
            other if entities.contains_key(other) => RTy::Entity(other.to_string()),
            _ => RTy::Unknown,
        },
    }
}

/// Lower a single handler to an AssemblyScript function body (statements only,
/// without the surrounding `export function` line). Returns warnings too.
pub fn lower_handler(handler: &HandlerDecl, env: &mut Env) -> (String, Vec<String>) {
    let abi = env
        .source_abi
        .get(&handler.source.name)
        .cloned()
        .unwrap_or_default();

    let mut scope = Scope {
        locals: HashMap::new(),
        event_param: handler.param.name.clone(),
        abi,
        event: handler.event.name.clone(),
        frames: Vec::new(),
        tmp: 0,
        warnings: Vec::new(),
    };

    let mut body = String::new();
    lower_block(&handler.body, env, &mut scope, &mut body, 1);
    (body, scope.warnings)
}

fn indent(level: usize) -> String {
    "  ".repeat(level)
}

/// Lower a block: statements followed by auto-saves for entities declared here.
fn lower_block(block: &Block, env: &mut Env, scope: &mut Scope, out: &mut String, level: usize) {
    scope.frames.push(Frame::default());
    for stmt in &block.stmts {
        lower_stmt(stmt, env, scope, out, level);
    }
    let frame = scope.frames.pop().expect("frame pushed above");
    if !frame.dirty.is_empty() {
        let pad = indent(level);
        out.push('\n');
        for name in &frame.dirty {
            out.push_str(&format!("{pad}{name}.save()\n"));
        }
    }
}

fn lower_stmt(stmt: &Stmt, env: &mut Env, scope: &mut Scope, out: &mut String, level: usize) {
    let pad = indent(level);
    match stmt {
        Stmt::Let { name, value, .. } => {
            if let Some(ctor) = entity_ctor(value) {
                lower_entity_ctor(name, &ctor, env, scope, out, level);
            } else if matches!(value, Expr::Match { .. }) {
                scope
                    .warnings
                    .push("`match` in `let` position is not supported yet".into());
                out.push_str(&format!("{pad}// TODO: `let {name} = match …` unsupported\n"));
            } else {
                let ty = infer(value, env, scope);
                let rhs = lower_expr(value, env, scope);
                out.push_str(&format!("{pad}let {name} = {rhs}\n"));
                scope.declare_local(&name.name, ty);
            }
        }
        Stmt::Assign { target, value, .. } => lower_assign(target, value, env, scope, out, level),
        Stmt::Return { value, .. } => match value {
            Some(v) => {
                let r = lower_expr(v, env, scope);
                out.push_str(&format!("{pad}return {r}\n"));
            }
            None => out.push_str(&format!("{pad}return\n")),
        },
        Stmt::Expr(e) => {
            if let Expr::Match { scrutinee, arms, .. } = e {
                lower_match(scrutinee, arms, env, scope, out, level);
            } else {
                let s = lower_expr(e, env, scope);
                out.push_str(&format!("{pad}{s}\n"));
            }
        }
    }
}

/// A recognised entity constructor call.
struct EntityCtor<'a> {
    entity: String,
    kind: CtorKind,
    id: &'a Expr,
    record: Option<&'a [(Ident, Expr)]>,
}

enum CtorKind {
    LoadOrCreate,
    Create,
    Load,
}

fn entity_ctor(value: &Expr) -> Option<EntityCtor<'_>> {
    let Expr::Call { callee, args, .. } = value else {
        return None;
    };
    let Expr::Field { base, field, .. } = callee.as_ref() else {
        return None;
    };
    let Expr::Path { segments, .. } = base.as_ref() else {
        return None;
    };
    let entity = segments.last()?.name.clone();
    let kind = match field.name.as_str() {
        "loadOrCreate" => CtorKind::LoadOrCreate,
        "create" => CtorKind::Create,
        "load" => CtorKind::Load,
        _ => return None,
    };
    let id = args.first()?;
    let record = match args.get(1) {
        Some(Expr::Record { fields, .. }) => Some(fields.as_slice()),
        _ => None,
    };
    Some(EntityCtor {
        entity,
        kind,
        id,
        record,
    })
}

fn lower_entity_ctor(
    name: &Ident,
    ctor: &EntityCtor,
    env: &mut Env,
    scope: &mut Scope,
    out: &mut String,
    level: usize,
) {
    let pad = indent(level);
    let var = &name.name;
    let entity = &ctor.entity;
    let id = lower_expr(ctor.id, env, scope);

    scope.declare_entity(var, entity.clone());

    match ctor.kind {
        CtorKind::LoadOrCreate => {
            out.push_str(&format!("{pad}let {var} = {entity}.load({id})\n"));
            out.push_str(&format!("{pad}if ({var} == null) {{\n"));
            out.push_str(&format!("{pad}  {var} = new {entity}({id})\n"));
            if let Some(fields) = ctor.record {
                lower_record_init(var, entity, fields, env, scope, out, level + 1);
            }
            out.push_str(&format!("{pad}}}\n"));
            scope.mark_dirty(var);
        }
        CtorKind::Create => {
            out.push_str(&format!("{pad}let {var} = new {entity}({id})\n"));
            if let Some(fields) = ctor.record {
                lower_record_init(var, entity, fields, env, scope, out, level);
            }
            scope.mark_dirty(var);
        }
        CtorKind::Load => {
            // load() may return null; not auto-saved unless later mutated.
            out.push_str(&format!("{pad}let {var} = {entity}.load({id})\n"));
        }
    }
}

fn lower_record_init(
    var: &str,
    entity: &str,
    fields: &[(Ident, Expr)],
    env: &mut Env,
    scope: &mut Scope,
    out: &mut String,
    level: usize,
) {
    let pad = indent(level);
    for (key, value) in fields {
        let rhs = lower_field_value(entity, &key.name, value, env, scope);
        out.push_str(&format!("{pad}{var}.{} = {rhs}\n", key.name));
    }
}

/// Lower a value being assigned to `entity.field`, coercing an entity-typed
/// value to its `.id` when the target field is an entity reference.
fn lower_field_value(
    entity: &str,
    field: &str,
    value: &Expr,
    env: &mut Env,
    scope: &mut Scope,
) -> String {
    let target_ty = env
        .entities
        .get(entity)
        .and_then(|e| e.fields.get(field))
        .cloned();
    let mut rhs = lower_expr(value, env, scope);
    if let Some(RTy::Entity(_)) = target_ty {
        if matches!(infer(value, env, scope), RTy::Entity(_)) {
            rhs.push_str(".id");
        }
    }
    rhs
}

fn lower_assign(
    target: &Expr,
    value: &Expr,
    env: &mut Env,
    scope: &mut Scope,
    out: &mut String,
    level: usize,
) {
    let pad = indent(level);
    if let Expr::Field { base, field, .. } = target {
        if let Expr::Path { segments, .. } = base.as_ref() {
            if segments.len() == 1 {
                let var = &segments[0].name;
                if let Some(RTy::Entity(entity)) = scope.locals.get(var).cloned() {
                    let rhs = lower_field_value(&entity, &field.name, value, env, scope);
                    out.push_str(&format!("{pad}{var}.{} = {rhs}\n", field.name));
                    scope.mark_dirty(var);
                    return;
                }
            }
        }
    }
    out.push_str(&format!(
        "{pad}{} = {}\n",
        lower_expr(target, env, scope),
        lower_expr(value, env, scope)
    ));
}

/// Lower a `match` statement on a `Result` or `Option` scrutinee.
fn lower_match(
    scrutinee: &Expr,
    arms: &[MatchArm],
    env: &mut Env,
    scope: &mut Scope,
    out: &mut String,
    level: usize,
) {
    let pad = indent(level);
    let scrut_ty = infer(scrutinee, env, scope);

    // Reference the scrutinee by a stable name; bind a temp if it's not a var.
    let var = if let Expr::Path { segments, .. } = scrutinee {
        if segments.len() == 1 {
            segments[0].name.clone()
        } else {
            bind_temp(scrutinee, env, scope, out, &pad)
        }
    } else {
        bind_temp(scrutinee, env, scope, out, &pad)
    };

    match scrut_ty {
        RTy::Result(inner) => {
            let (ok_bind, ok_body) = find_arm(arms, "Ok");
            let (_err_bind, err_body) = find_arm(arms, "Err");
            out.push_str(&format!("{pad}if (!{var}.reverted) {{\n"));
            if let Some(bind) = ok_bind {
                out.push_str(&format!("{pad}  let {bind} = {var}.value\n"));
                scope.declare_local(&bind.name, (*inner).clone());
            }
            if let Some(body) = ok_body {
                lower_block(body, env, scope, out, level + 1);
            }
            out.push_str(&format!("{pad}}}"));
            if let Some(body) = err_body.filter(|b| !b.stmts.is_empty()) {
                out.push_str(" else {\n");
                lower_block(body, env, scope, out, level + 1);
                out.push_str(&format!("{pad}}}"));
            }
            out.push('\n');
        }
        RTy::Option(inner) => {
            let (some_bind, some_body) = find_arm(arms, "Some");
            let (_none_bind, none_body) = find_arm(arms, "None");
            out.push_str(&format!("{pad}if ({var} != null) {{\n"));
            if let Some(bind) = some_bind {
                out.push_str(&format!("{pad}  let {bind} = {var}!\n"));
                scope.declare_local(&bind.name, (*inner).clone());
            }
            if let Some(body) = some_body {
                lower_block(body, env, scope, out, level + 1);
            }
            out.push_str(&format!("{pad}}}"));
            if let Some(body) = none_body.filter(|b| !b.stmts.is_empty()) {
                out.push_str(" else {\n");
                lower_block(body, env, scope, out, level + 1);
                out.push_str(&format!("{pad}}}"));
            }
            out.push('\n');
        }
        _ => {
            scope.warnings.push(format!(
                "`match` on a {scrut_ty:?} scrutinee is not supported yet; emitted a comment"
            ));
            out.push_str(&format!("{pad}// TODO: unsupported match\n"));
        }
    }
}

fn bind_temp(expr: &Expr, env: &mut Env, scope: &mut Scope, out: &mut String, pad: &str) -> String {
    let name = scope.fresh();
    let rhs = lower_expr(expr, env, scope);
    out.push_str(&format!("{pad}let {name} = {rhs}\n"));
    name
}

/// Find the arm whose pattern is a constructor named `ctor`, returning its first
/// binding (if any) and its body block.
fn find_arm<'a>(arms: &'a [MatchArm], ctor: &str) -> (Option<&'a Ident>, Option<&'a Block>) {
    for arm in arms {
        if let Pattern::Ctor { name, bindings, .. } = &arm.pattern {
            if name.name == ctor {
                return (bindings.first(), Some(&arm.body));
            }
        }
    }
    (None, None)
}

/// Lower an expression to AssemblyScript text.
fn lower_expr(expr: &Expr, env: &mut Env, scope: &mut Scope) -> String {
    match expr {
        Expr::Int { raw, .. } => raw.clone(),
        Expr::Hex { raw, .. } => format!("Bytes.fromHexString(\"{raw}\")"),
        Expr::Decimal { raw, .. } => format!("BigDecimal.fromString(\"{raw}\")"),
        Expr::Str { value, .. } => format!("\"{}\"", value.replace('"', "\\\"")),
        Expr::Bool { value, .. } => value.to_string(),
        Expr::Path { segments, .. } => segments
            .iter()
            .map(|s| s.name.clone())
            .collect::<Vec<_>>()
            .join("."),
        Expr::Field { base, field, .. } => lower_field(base, &field.name, env, scope),
        Expr::Call { callee, args, .. } => lower_call(callee, args, env, scope),
        Expr::Record { .. } => "/* record */".to_string(),
        Expr::Unary { op, expr, .. } => {
            let inner = lower_expr(expr, env, scope);
            match op {
                UnOp::Not => format!("!{inner}"),
                UnOp::Neg => format!("-{inner}"),
            }
        }
        Expr::Binary { op, lhs, rhs, .. } => lower_binary(*op, lhs, rhs, env, scope),
        Expr::Match { .. } => {
            scope
                .warnings
                .push("`match` used as a value is not supported yet".into());
            "/* TODO: match */".to_string()
        }
    }
}

fn lower_field(base: &Expr, field: &str, env: &mut Env, scope: &mut Scope) -> String {
    // Synthetic `event.id` -> a unique composite id.
    if field == "id" {
        if let Expr::Path { segments, .. } = base {
            if segments.len() == 1 && segments[0].name == scope.event_param {
                return format!(
                    "{ev}.transaction.hash.concatI32({ev}.logIndex.toI32())",
                    ev = scope.event_param
                );
            }
        }
    }
    // Static zero accessors: `BigInt.zero` -> `BigInt.zero()`.
    if field == "zero" {
        if let Expr::Path { segments, .. } = base {
            if segments.len() == 1 && matches!(segments[0].name.as_str(), "BigInt" | "BigDecimal") {
                return format!("{}.zero()", segments[0].name);
            }
        }
    }
    format!("{}.{field}", lower_expr(base, env, scope))
}

fn lower_call(callee: &Expr, args: &[Expr], env: &mut Env, scope: &mut Scope) -> String {
    if let Expr::Field { base, field, .. } = callee {
        // Contract call: `<contract>.method(args)` -> `<contract>.try_method(args)`.
        if let RTy::Contract(abi) = infer(base, env, scope) {
            if env.abis.is_function(&abi, &field.name) {
                let base_s = lower_expr(base, env, scope);
                let arg_s = lower_args(args, env, scope);
                return format!("{base_s}.try_{}({arg_s})", field.name);
            }
        }
        // Remap known method names: `.toDecimal()` -> `.toBigDecimal()`.
        let method = match field.name.as_str() {
            "toDecimal" => "toBigDecimal",
            other => other,
        };
        let base_s = lower_expr(base, env, scope);
        let arg_s = lower_args(args, env, scope);
        return format!("{base_s}.{method}({arg_s})");
    }
    let callee_s = lower_expr(callee, env, scope);
    let arg_s = lower_args(args, env, scope);
    format!("{callee_s}({arg_s})")
}

fn lower_args(args: &[Expr], env: &mut Env, scope: &mut Scope) -> String {
    args.iter()
        .map(|a| lower_expr(a, env, scope))
        .collect::<Vec<_>>()
        .join(", ")
}

fn lower_binary(op: BinOp, lhs: &Expr, rhs: &Expr, env: &mut Env, scope: &mut Scope) -> String {
    let lt = infer(lhs, env, scope);
    let rt = infer(rhs, env, scope);
    let ls = lower_expr(lhs, env, scope);
    let rs = lower_expr(rhs, env, scope);

    if (lt.is_bigint() || rt.is_bigint() || lt.is_bigdecimal() || rt.is_bigdecimal())
        && bigmath_method(op).is_some()
    {
        return format!("{ls}.{}({rs})", bigmath_method(op).unwrap());
    }
    format!("{ls} {} {rs}", binop_symbol(op))
}

/// The graph-ts `BigInt`/`BigDecimal` method for an operator.
fn bigmath_method(op: BinOp) -> Option<&'static str> {
    Some(match op {
        BinOp::Add => "plus",
        BinOp::Sub => "minus",
        BinOp::Mul => "times",
        BinOp::Div => "div",
        BinOp::Eq => "equals",
        BinOp::Ne => "notEqual",
        BinOp::Lt => "lt",
        BinOp::Le => "le",
        BinOp::Gt => "gt",
        BinOp::Ge => "ge",
        _ => return None,
    })
}

fn binop_symbol(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Rem => "%",
        BinOp::Eq => "==",
        BinOp::Ne => "!=",
        BinOp::Lt => "<",
        BinOp::Le => "<=",
        BinOp::Gt => ">",
        BinOp::Ge => ">=",
        BinOp::And => "&&",
        BinOp::Or => "||",
    }
}

/// Infer the resolved type of an expression — enough to choose lowerings.
fn infer(expr: &Expr, env: &mut Env, scope: &mut Scope) -> RTy {
    match expr {
        Expr::Int { .. } => RTy::Int,
        Expr::Decimal { .. } => RTy::BigDecimal,
        Expr::Hex { .. } => RTy::Bytes,
        Expr::Str { .. } => RTy::String,
        Expr::Bool { .. } => RTy::Boolean,
        Expr::Path { segments, .. } => {
            if segments.len() == 1 {
                if segments[0].name == scope.event_param {
                    return RTy::Event;
                }
                if let Some(t) = scope.locals.get(&segments[0].name) {
                    return t.clone();
                }
            }
            RTy::Unknown
        }
        Expr::Field { base, field, .. } => infer_field(base, &field.name, env, scope),
        Expr::Call { callee, .. } => infer_call(callee, env, scope),
        Expr::Binary { op, lhs, rhs, .. } => {
            if matches!(
                op,
                BinOp::Eq
                    | BinOp::Ne
                    | BinOp::Lt
                    | BinOp::Le
                    | BinOp::Gt
                    | BinOp::Ge
                    | BinOp::And
                    | BinOp::Or
            ) {
                RTy::Boolean
            } else {
                let lt = infer(lhs, env, scope);
                if lt == RTy::Unknown {
                    infer(rhs, env, scope)
                } else {
                    lt
                }
            }
        }
        Expr::Unary { expr, .. } => infer(expr, env, scope),
        _ => RTy::Unknown,
    }
}

fn infer_field(base: &Expr, field: &str, env: &mut Env, scope: &mut Scope) -> RTy {
    let base_ty = infer(base, env, scope);
    match base_ty {
        RTy::Event => match field {
            "params" => RTy::Params,
            "block" => RTy::Block,
            "transaction" => RTy::Transaction,
            "address" => RTy::Address,
            "id" => RTy::Bytes,
            _ => RTy::Unknown,
        },
        RTy::Params => {
            let (abi, event) = (scope.abi.clone(), scope.event.clone());
            env.abis
                .event_params(&abi, &event)
                .and_then(|params| {
                    params
                        .iter()
                        .find(|p| p.name == field)
                        .map(|p| sol_to_rty(&p.sol_type))
                })
                .unwrap_or(RTy::Unknown)
        }
        RTy::Block => match field {
            "timestamp" | "number" => RTy::BigInt,
            "hash" => RTy::Bytes,
            _ => RTy::Unknown,
        },
        RTy::Transaction => match field {
            "hash" | "from" | "to" => RTy::Bytes,
            "value" | "gasPrice" => RTy::BigInt,
            _ => RTy::Unknown,
        },
        RTy::Result(inner) => match field {
            "value" => *inner,
            "reverted" => RTy::Boolean,
            _ => RTy::Unknown,
        },
        RTy::Entity(name) => env
            .entities
            .get(&name)
            .and_then(|e| e.fields.get(field))
            .cloned()
            .unwrap_or(RTy::Unknown),
        _ => RTy::Unknown,
    }
}

fn infer_call(callee: &Expr, env: &mut Env, scope: &mut Scope) -> RTy {
    if let Expr::Field { base, field, .. } = callee {
        // `Abi.bind(addr)` -> a bound contract instance.
        if field.name == "bind" {
            if let Expr::Path { segments, .. } = base.as_ref() {
                if segments.len() == 1 && env.abis.paths.contains_key(&segments[0].name) {
                    return RTy::Contract(segments[0].name.clone());
                }
            }
        }
        // `<contract>.method(args)` -> Result<ret, CallRevert>.
        if let RTy::Contract(abi) = infer(base, env, scope) {
            if let Some(outputs) = env.abis.function_outputs(&abi, &field.name) {
                let ret = outputs.first().map_or(RTy::Unknown, |s| sol_to_rty(s));
                return RTy::Result(Box::new(ret));
            }
        }
        match field.name.as_str() {
            "toDecimal" | "toBigDecimal" => return RTy::BigDecimal,
            "toBigInt" => return RTy::BigInt,
            "abs" | "plus" | "minus" | "times" | "div" => return infer(base, env, scope),
            _ => {}
        }
    }
    RTy::Unknown
}
