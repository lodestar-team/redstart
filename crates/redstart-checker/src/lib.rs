//! Semantic analysis for Redstart.
//!
//! [`check`] runs over a loaded [`ModuleTree`] and enforces the guarantees the
//! design report makes load-bearing — "impossible states unrepresentable" — as
//! actual compile errors, spanning every module:
//!
//! - handlers must bind to a real source and a real ABI event;
//! - sources must declare their required settings and a known ABI;
//! - entity field types must resolve; `derived from` must name a real back-ref;
//! - `loadOrCreate`/`create` must initialise every required field;
//! - you cannot assign to a `derived` field;
//! - you cannot read a contract call's `.value` without `match`ing it first
//!   (contract calls can revert);
//! - you cannot do arithmetic on an `Option` without unwrapping it (the
//!   nullable-arithmetic miscompile).
//!
//! On success it returns a [`Checked`] symbol table that codegen consumes, so
//! the resolved types are computed once and shared.

#![forbid(unsafe_code)]

pub mod abi;
mod diag;
pub mod ty;

pub use abi::{resolve_abi_path, AbiIndex, EventParam};
pub use diag::Diag;
pub use ty::{is_scalar, resolve_type, sol_to_rty, EntityInfo, RTy};
use redstart_loader::ModuleTree;
use redstart_parser::ast::{
    EntityDecl, Expr, FieldDecl, ForIter, HandlerDecl, HandlerKind, MatchArm, Pattern, Setting,
    SourceDecl, Stmt, TemplateDecl, TypeExpr,
};
use std::collections::HashMap;

/// The validated symbol table produced by a successful [`check`].
pub struct Checked {
    /// Entity name -> resolved field types.
    pub entities: HashMap<String, EntityInfo>,
    /// Source/template name -> ABI name.
    pub source_abi: HashMap<String, String>,
    /// ABI access (resolved file paths, event/function lookups).
    pub abis: AbiIndex,
}

/// Run semantic analysis. On error, returns rendered diagnostics (one string
/// per problem), ready to print.
///
/// # Errors
/// Returns the rendered diagnostics if any check fails.
pub fn check(tree: &ModuleTree) -> Result<Checked, Vec<String>> {
    let (checked, diags) = analyze(tree);
    if diags.is_empty() {
        Ok(checked)
    } else {
        Err(diags.iter().map(Diag::render).collect())
    }
}

/// Run semantic analysis, returning structured diagnostics (empty if clean).
/// Used by the language server to publish editor squiggles.
#[must_use]
pub fn check_diags(tree: &ModuleTree) -> Vec<Diag> {
    analyze(tree).1
}

fn analyze(tree: &ModuleTree) -> (Checked, Vec<Diag>) {
    let mut diags: Vec<Diag> = Vec::new();

    // ---- gather modules with their filenames ----
    let modules = tree.ordered();
    let files: Vec<String> = modules
        .iter()
        .map(|m| m.file_path.display().to_string())
        .collect();

    // ---- global symbol build ----
    let mut abis = AbiIndex::default();
    for m in &modules {
        let dir = m.file_path.parent().unwrap_or_else(|| std::path::Path::new("."));
        for a in &m.program.abis {
            abis.insert(a.name.name.clone(), resolve_abi_path(dir, &a.path));
        }
    }

    // Entity names (first pass), checking duplicates.
    let mut entities: HashMap<String, EntityInfo> = HashMap::new();
    let mut entity_meta: HashMap<String, EntityMeta> = HashMap::new();
    let mut seen: HashMap<String, ()> = HashMap::new();
    for (m, file) in modules.iter().zip(&files) {
        for e in &m.program.entities {
            if seen.insert(e.name.name.clone(), ()).is_some() {
                diags.push(
                    Diag::new(file, &e.name.span, "E010", format!("duplicate entity `{}`", e.name.name), "already declared")
                        .with_help("each entity must be declared exactly once across all modules"),
                );
            }
            entities.insert(e.name.name.clone(), EntityInfo::default());
            entity_meta.insert(e.name.name.clone(), EntityMeta::from_decl(e));
        }
    }
    // Second pass: resolve field types now that all names are known.
    for m in &modules {
        for e in &m.program.entities {
            let fields = e
                .fields
                .iter()
                .map(|f| (f.name.name.clone(), resolve_type(&f.ty, &entities)))
                .collect();
            entities.insert(e.name.name.clone(), EntityInfo { fields });
        }
    }

    // Source / template tables.
    let mut source_abi: HashMap<String, String> = HashMap::new();
    let mut data_source_names: HashMap<String, ()> = HashMap::new();
    for m in &modules {
        for s in &m.program.sources {
            data_source_names.insert(s.name.name.clone(), ());
            if let Some(a) = path_setting(&s.settings, "abi") {
                source_abi.insert(s.name.name.clone(), a);
            }
        }
        for t in &m.program.templates {
            data_source_names.insert(t.name.name.clone(), ());
            if let Some(a) = path_setting(&t.settings, "abi") {
                source_abi.insert(t.name.name.clone(), a);
            }
        }
    }

    // ---- per-declaration validation ----
    let entity_names: Vec<String> = entities.keys().cloned().collect();
    for (m, file) in modules.iter().zip(&files) {
        for e in &m.program.entities {
            check_entity(e, &entity_names, &entity_meta, file, &mut diags);
        }
        for s in &m.program.sources {
            check_source(s, &abis, file, &mut diags);
        }
        for t in &m.program.templates {
            check_template(t, &abis, file, &mut diags);
        }
        for h in &m.program.handlers {
            check_handler(
                h,
                &entities,
                &entity_meta,
                &source_abi,
                &data_source_names,
                &mut abis,
                file,
                &mut diags,
            );
        }
    }

    (
        Checked {
            entities,
            source_abi,
            abis,
        },
        diags,
    )
}

// ---- entity metadata (from AST) ----

struct EntityMeta {
    fields: Vec<FieldMeta>,
}

struct FieldMeta {
    name: String,
    is_optional: bool,
    is_derived: bool,
}

impl EntityMeta {
    fn from_decl(e: &EntityDecl) -> Self {
        let fields = e
            .fields
            .iter()
            .map(|f| FieldMeta {
                name: f.name.name.clone(),
                is_optional: is_option_type(&f.ty),
                is_derived: f.derived_from.is_some(),
            })
            .collect();
        Self { fields }
    }

    /// Fields a constructor record must initialise: not the id, not derived,
    /// not optional.
    fn required(&self) -> Vec<&str> {
        self.fields
            .iter()
            .filter(|f| f.name != "id" && !f.is_derived && !f.is_optional)
            .map(|f| f.name.as_str())
            .collect()
    }

    fn has_field(&self, name: &str) -> bool {
        self.fields.iter().any(|f| f.name == name)
    }

    fn is_derived(&self, name: &str) -> bool {
        self.fields.iter().any(|f| f.name == name && f.is_derived)
    }
}

fn is_option_type(ty: &TypeExpr) -> bool {
    matches!(ty, TypeExpr::Generic { base, .. } if base.simple_name() == Some("Option"))
}

// ---- declaration checks ----

fn check_entity(
    e: &EntityDecl,
    entity_names: &[String],
    meta: &HashMap<String, EntityMeta>,
    file: &str,
    diags: &mut Vec<Diag>,
) {
    for f in &e.fields {
        validate_type(&f.ty, entity_names, file, diags);
        if let Some(back) = &f.derived_from {
            check_derived(f, back, meta, entity_names, file, diags);
        }
    }
}

fn check_derived(
    f: &FieldDecl,
    back: &redstart_parser::Ident,
    meta: &HashMap<String, EntityMeta>,
    entity_names: &[String],
    file: &str,
    diags: &mut Vec<Diag>,
) {
    let Some(target) = entity_name_of(&f.ty) else {
        diags.push(
            Diag::new(file, f.ty.span(), "E020", "a `derived from` field must reference an entity", "not an entity type")
                .with_help("derived fields look like `swaps: [Swap] derived from pool`"),
        );
        return;
    };
    if !entity_names.iter().any(|n| n == &target) {
        return; // already reported by validate_type
    }
    match meta.get(&target) {
        Some(tm) if tm.has_field(&back.name) => {}
        Some(_) => diags.push(
            Diag::new(
                file,
                &back.span,
                "E021",
                format!("`{target}` has no field `{}` to derive from", back.name),
                "no such field",
            )
            .with_help(format!("add a `{}: {}` field to `{target}`", back.name, "…")),
        ),
        None => {}
    }
}

fn check_source(s: &SourceDecl, abis: &AbiIndex, file: &str, diags: &mut Vec<Diag>) {
    for key in ["abi", "network", "address", "startBlock"] {
        if get_setting(&s.settings, key).is_none() {
            diags.push(
                Diag::new(file, &s.name.span, "E030", format!("source `{}` is missing `{key}`", s.name.name), format!("add `{key}: …`"))
                    .with_help("a source needs `abi`, `network`, `address`, and `startBlock`"),
            );
        }
    }
    check_abi_ref(&s.settings, abis, file, diags);
}

fn check_template(t: &TemplateDecl, abis: &AbiIndex, file: &str, diags: &mut Vec<Diag>) {
    for key in ["abi", "network"] {
        if get_setting(&t.settings, key).is_none() {
            diags.push(Diag::new(
                file,
                &t.name.span,
                "E031",
                format!("template `{}` is missing `{key}`", t.name.name),
                format!("add `{key}: …`"),
            ));
        }
    }
    check_abi_ref(&t.settings, abis, file, diags);
}

fn check_abi_ref(settings: &[Setting], abis: &AbiIndex, file: &str, diags: &mut Vec<Diag>) {
    if let Some(setting) = get_setting(settings, "abi") {
        if let Some(name) = path_name(&setting.value) {
            if !abis.paths.contains_key(&name) {
                diags.push(
                    Diag::new(file, setting.value.span(), "E032", format!("unknown ABI `{name}`"), "not imported")
                        .with_help(format!("import it with `abi {name} from \"./abis/{name}.json\"`")),
                );
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn check_handler(
    h: &HandlerDecl,
    entities: &HashMap<String, EntityInfo>,
    meta: &HashMap<String, EntityMeta>,
    source_abi: &HashMap<String, String>,
    data_sources: &HashMap<String, ()>,
    abis: &mut AbiIndex,
    file: &str,
    diags: &mut Vec<Diag>,
) {
    // The source must exist.
    if !data_sources.contains_key(&h.source.name) {
        diags.push(
            Diag::new(file, &h.source.span, "E040", format!("unknown source `{}`", h.source.name), "no such source or template")
                .with_help("declare it with a `source` (or `template`) block"),
        );
        return;
    }

    let abi_name = source_abi.get(&h.source.name).cloned().unwrap_or_default();

    // Resolve the handler's trigger members and the type of its parameter.
    let (param_ty, params, outputs) = match h.kind {
        HandlerKind::Event => {
            if abis.readable(&abi_name) && abis.event_params(&abi_name, &h.event.name).is_none() {
                diags.push(
                    Diag::new(file, &h.event.span, "E041", format!("event `{}` not found in ABI `{abi_name}`", h.event.name), "no such event")
                        .with_help("check the event name and casing against the ABI"),
                );
            }
            (RTy::Event, rty_map(abis.event_params(&abi_name, &h.event.name)), HashMap::new())
        }
        HandlerKind::Call => {
            if abis.readable(&abi_name) && abis.function_inputs(&abi_name, &h.event.name).is_none() {
                diags.push(
                    Diag::new(file, &h.event.span, "E042", format!("function `{}` not found in ABI `{abi_name}`", h.event.name), "no such function")
                        .with_help("call handlers bind a contract function by name — check it against the ABI"),
                );
            }
            (
                RTy::Call,
                rty_map(abis.function_inputs(&abi_name, &h.event.name)),
                rty_map(abis.function_output_params(&abi_name, &h.event.name)),
            )
        }
        HandlerKind::Block(_) => (RTy::Block, HashMap::new(), HashMap::new()),
    };

    let ctx = BodyCtx {
        entities,
        meta,
        event_param: h.param.name.clone(),
        param_ty,
        event_params: params,
        call_outputs: outputs,
        abis,
    };
    let mut locals: HashMap<String, RTy> = HashMap::new();
    check_block(&h.body.stmts, &ctx, &mut locals, file, diags);
}

/// Convert an optional ABI parameter list into a name → resolved-type map.
fn rty_map(params: Option<Vec<EventParam>>) -> HashMap<String, RTy> {
    params
        .unwrap_or_default()
        .into_iter()
        .map(|p| (p.name, sol_to_rty(&p.sol_type)))
        .collect()
}

// ---- handler body checks ----

struct BodyCtx<'a> {
    entities: &'a HashMap<String, EntityInfo>,
    meta: &'a HashMap<String, EntityMeta>,
    event_param: String,
    /// The resolved type of the handler parameter (Event / Call / Block).
    param_ty: RTy,
    /// Event params (event handler) or function inputs (call handler).
    event_params: HashMap<String, RTy>,
    /// Function outputs (call handler only).
    call_outputs: HashMap<String, RTy>,
    abis: &'a AbiIndex,
}

fn check_block(
    stmts: &[Stmt],
    ctx: &BodyCtx,
    locals: &mut HashMap<String, RTy>,
    file: &str,
    diags: &mut Vec<Diag>,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Let { name, value, .. } => {
                if let Some((entity, record)) = entity_ctor(value) {
                    check_ctor_record(&entity, record, ctx, file, value, diags);
                    check_expr(value, ctx, locals, file, diags);
                    // load/loadOrCreate/create yield the entity type.
                    locals.insert(name.name.clone(), RTy::Entity(entity));
                } else {
                    check_expr(value, ctx, locals, file, diags);
                    locals.insert(name.name.clone(), infer(value, ctx, locals));
                }
            }
            Stmt::Assign { target, value, .. } => {
                check_assign_target(target, ctx, locals, file, diags);
                check_expr(target, ctx, locals, file, diags);
                check_expr(value, ctx, locals, file, diags);
            }
            Stmt::Return { value: Some(v), .. } => check_expr(v, ctx, locals, file, diags),
            Stmt::Return { .. } => {}
            Stmt::If {
                cond,
                then_block,
                else_ifs,
                else_block,
                ..
            } => {
                check_expr(cond, ctx, locals, file, diags);
                let mut b = locals.clone();
                check_block(&then_block.stmts, ctx, &mut b, file, diags);
                for (c, block) in else_ifs {
                    check_expr(c, ctx, locals, file, diags);
                    let mut bb = locals.clone();
                    check_block(&block.stmts, ctx, &mut bb, file, diags);
                }
                if let Some(block) = else_block {
                    let mut bb = locals.clone();
                    check_block(&block.stmts, ctx, &mut bb, file, diags);
                }
            }
            Stmt::While { cond, body, .. } => {
                check_expr(cond, ctx, locals, file, diags);
                let mut b = locals.clone();
                check_block(&body.stmts, ctx, &mut b, file, diags);
            }
            Stmt::For { var, iter, body, .. } => {
                let elem = check_for_iter(iter, ctx, locals, file, diags);
                let mut b = locals.clone();
                b.insert(var.name.clone(), elem);
                check_block(&body.stmts, ctx, &mut b, file, diags);
            }
            Stmt::Expr(e) => {
                if let Expr::Match { scrutinee, arms, .. } = e {
                    check_match(scrutinee, arms, ctx, locals, file, diags);
                } else {
                    check_expr(e, ctx, locals, file, diags);
                }
            }
        }
    }
}

/// Check a `for` iterable, returning the loop variable's element type.
fn check_for_iter(
    iter: &ForIter,
    ctx: &BodyCtx,
    locals: &HashMap<String, RTy>,
    file: &str,
    diags: &mut Vec<Diag>,
) -> RTy {
    match iter {
        ForIter::Range { start, end } => {
            check_expr(start, ctx, locals, file, diags);
            check_expr(end, ctx, locals, file, diags);
            RTy::Int
        }
        ForIter::Each(list) => {
            check_expr(list, ctx, locals, file, diags);
            match infer(list, ctx, locals) {
                RTy::List(inner) => *inner,
                _ => RTy::Unknown,
            }
        }
    }
}

fn check_match(
    scrutinee: &Expr,
    arms: &[MatchArm],
    ctx: &BodyCtx,
    locals: &mut HashMap<String, RTy>,
    file: &str,
    diags: &mut Vec<Diag>,
) {
    check_expr(scrutinee, ctx, locals, file, diags);
    let scrut_ty = infer(scrutinee, ctx, locals);
    for arm in arms {
        let mut arm_locals = locals.clone();
        if let Pattern::Ctor { name, bindings, .. } = &arm.pattern {
            if let Some(b) = bindings.first() {
                let bound = match (&scrut_ty, name.name.as_str()) {
                    (RTy::Result(inner), "Ok") | (RTy::Option(inner), "Some") => (**inner).clone(),
                    _ => RTy::Unknown,
                };
                arm_locals.insert(b.name.clone(), bound);
            }
        }
        check_block(&arm.body.stmts, ctx, &mut arm_locals, file, diags);
    }

    check_exhaustive(scrutinee, arms, &scrut_ty, file, diags);
}

/// Require a `match` to cover every variant (or carry a wildcard).
fn check_exhaustive(scrutinee: &Expr, arms: &[MatchArm], scrut_ty: &RTy, file: &str, diags: &mut Vec<Diag>) {
    let required: &[&str] = match scrut_ty {
        RTy::Result(_) => &["Ok", "Err"],
        RTy::Option(_) => &["Some", "None"],
        _ => return, // unknown scrutinee — can't judge
    };

    let has_wildcard = arms
        .iter()
        .any(|a| matches!(a.pattern, Pattern::Wildcard { .. } | Pattern::Binding { .. }));
    if has_wildcard {
        return;
    }

    let present: Vec<&str> = arms
        .iter()
        .filter_map(|a| match &a.pattern {
            Pattern::Ctor { name, .. } => Some(name.name.as_str()),
            _ => None,
        })
        .collect();
    let missing: Vec<&str> = required
        .iter()
        .copied()
        .filter(|r| !present.contains(r))
        .collect();
    if !missing.is_empty() {
        diags.push(
            Diag::new(file, scrutinee.span(), "E070", format!("non-exhaustive `match`: missing {}", missing.join(", ")), "add the missing arm(s)")
                .with_help("every variant must be handled — or add a `_ => { … }` wildcard arm"),
        );
    }
}

/// Check that a `loadOrCreate`/`create` record initialises every required field.
fn check_ctor_record(
    entity: &str,
    record: Option<&[(redstart_parser::Ident, Expr)]>,
    ctx: &BodyCtx,
    file: &str,
    value: &Expr,
    diags: &mut Vec<Diag>,
) {
    let Some(meta) = ctx.meta.get(entity) else {
        diags.push(Diag::new(
            file,
            value.span(),
            "E050",
            format!("unknown entity `{entity}`"),
            "no such entity",
        ));
        return;
    };
    let Some(record) = record else { return }; // load() has no record
    let present: Vec<&str> = record.iter().map(|(k, _)| k.name.as_str()).collect();

    let missing: Vec<&str> = meta
        .required()
        .into_iter()
        .filter(|r| !present.contains(r))
        .collect();
    if !missing.is_empty() {
        diags.push(
            Diag::new(
                file,
                value.span(),
                "E051",
                format!("`{entity}` is missing required field(s): {}", missing.join(", ")),
                "incomplete initializer",
            )
            .with_help("every non-optional field must be set when creating an entity"),
        );
    }

    for (k, _) in record {
        if !meta.has_field(&k.name) {
            diags.push(Diag::new(
                file,
                &k.span,
                "E052",
                format!("`{entity}` has no field `{}`", k.name),
                "unknown field",
            ));
        }
    }
}

/// Forbid assigning to a `derived` field.
fn check_assign_target(
    target: &Expr,
    ctx: &BodyCtx,
    locals: &HashMap<String, RTy>,
    file: &str,
    diags: &mut Vec<Diag>,
) {
    if let Expr::Field { base, field, .. } = target {
        if let RTy::Entity(entity) = infer(base, ctx, locals) {
            if ctx.meta.get(&entity).is_some_and(|m| m.is_derived(&field.name)) {
                diags.push(
                    Diag::new(file, &field.span, "E053", format!("cannot assign to derived field `{}`", field.name), "derived fields are read-only")
                        .with_help("`derived from` fields are computed from the other side of the relation"),
                );
            }
        }
    }
}

/// Walk an expression, flagging the contract-call and nullable-arithmetic
/// footguns.
fn check_expr(
    expr: &Expr,
    ctx: &BodyCtx,
    locals: &HashMap<String, RTy>,
    file: &str,
    diags: &mut Vec<Diag>,
) {
    match expr {
        Expr::Field { base, field, .. } => {
            match infer(base, ctx, locals) {
                RTy::Result(_) if field.name == "value" => diags.push(
                    Diag::new(file, &field.span, "E060", "cannot read `.value` of a contract call directly", "this call may have reverted")
                        .with_help("`match` on the result: `match call { Ok(v) => { … } Err(e) => { … } }`"),
                ),
                RTy::Entity(name) if field.name != "id" => {
                    if ctx.meta.get(&name).is_some_and(|m| !m.has_field(&field.name)) {
                        diags.push(Diag::new(
                            file,
                            &field.span,
                            "E054",
                            format!("`{name}` has no field `{}`", field.name),
                            "unknown field",
                        ));
                    }
                }
                _ => {}
            }
            check_expr(base, ctx, locals, file, diags);
        }
        Expr::Binary { op, lhs, rhs, .. } => {
            if is_arithmetic(*op) {
                for side in [lhs.as_ref(), rhs.as_ref()] {
                    if infer(side, ctx, locals).is_option() {
                        diags.push(
                            Diag::new(file, side.span(), "E061", "cannot do arithmetic on an `Option`", "unwrap this first")
                                .with_help("use `match` or `.unwrapOr(default)` before arithmetic"),
                        );
                    }
                }
            }
            check_expr(lhs, ctx, locals, file, diags);
            check_expr(rhs, ctx, locals, file, diags);
        }
        Expr::Call { callee, args, .. } => {
            // Calling a function that the contract's ABI doesn't have.
            if let Expr::Field { base, field, .. } = callee.as_ref() {
                if let RTy::Contract(abi) = infer(base, ctx, locals) {
                    if ctx.abis.readable(&abi) && !ctx.abis.is_function(&abi, &field.name) {
                        diags.push(
                            Diag::new(file, &field.span, "E071", format!("contract `{abi}` has no function `{}`", field.name), "no such function")
                                .with_help("check the function name against the ABI (only view/pure calls are supported)"),
                        );
                    }
                }
            }
            check_expr(callee, ctx, locals, file, diags);
            for a in args {
                check_expr(a, ctx, locals, file, diags);
            }
        }
        Expr::Unary { expr, .. } => check_expr(expr, ctx, locals, file, diags),
        Expr::Record { fields, .. } => {
            for (_, v) in fields {
                check_expr(v, ctx, locals, file, diags);
            }
        }
        _ => {}
    }
}

fn is_arithmetic(op: redstart_parser::ast::BinOp) -> bool {
    use redstart_parser::ast::BinOp;
    matches!(op, BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem)
}

// ---- inference (read-only; mirrors codegen's lowering view) ----

fn infer(expr: &Expr, ctx: &BodyCtx, locals: &HashMap<String, RTy>) -> RTy {
    match expr {
        Expr::Int { .. } => RTy::Int,
        Expr::Decimal { .. } => RTy::BigDecimal,
        Expr::Hex { .. } => RTy::Bytes,
        Expr::Str { .. } => RTy::String,
        Expr::Bool { .. } => RTy::Boolean,
        Expr::Path { segments, .. } => {
            if segments.len() == 1 {
                if segments[0].name == ctx.event_param {
                    return ctx.param_ty.clone();
                }
                if let Some(t) = locals.get(&segments[0].name) {
                    return t.clone();
                }
            }
            RTy::Unknown
        }
        Expr::Field { base, field, .. } => infer_field(base, &field.name, ctx, locals),
        Expr::Call { callee, .. } => infer_call(callee, ctx, locals),
        Expr::Binary { op, lhs, rhs, .. } => {
            use redstart_parser::ast::BinOp;
            if matches!(
                op,
                BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge | BinOp::And | BinOp::Or
            ) {
                RTy::Boolean
            } else {
                let lt = infer(lhs, ctx, locals);
                if lt == RTy::Unknown {
                    infer(rhs, ctx, locals)
                } else {
                    lt
                }
            }
        }
        Expr::Unary { expr, .. } => infer(expr, ctx, locals),
        _ => RTy::Unknown,
    }
}

fn infer_field(base: &Expr, field: &str, ctx: &BodyCtx, locals: &HashMap<String, RTy>) -> RTy {
    match infer(base, ctx, locals) {
        RTy::Event => match field {
            "params" => RTy::Params,
            "block" => RTy::Block,
            "transaction" => RTy::Transaction,
            "address" => RTy::Address,
            "id" => RTy::Bytes,
            _ => RTy::Unknown,
        },
        RTy::Params => ctx.event_params.get(field).cloned().unwrap_or(RTy::Unknown),
        RTy::Call => match field {
            "inputs" => RTy::CallInputs,
            "outputs" => RTy::CallOutputs,
            "block" => RTy::Block,
            "transaction" => RTy::Transaction,
            "from" | "to" => RTy::Address,
            _ => RTy::Unknown,
        },
        RTy::CallInputs => ctx.event_params.get(field).cloned().unwrap_or(RTy::Unknown),
        RTy::CallOutputs => ctx.call_outputs.get(field).cloned().unwrap_or(RTy::Unknown),
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
        RTy::Entity(name) => ctx
            .entities
            .get(&name)
            .and_then(|e| e.fields.get(field))
            .cloned()
            .unwrap_or(RTy::Unknown),
        _ => RTy::Unknown,
    }
}

fn infer_call(callee: &Expr, ctx: &BodyCtx, locals: &HashMap<String, RTy>) -> RTy {
    if let Expr::Field { base, field, .. } = callee {
        if field.name == "bind" {
            if let Expr::Path { segments, .. } = base.as_ref() {
                if segments.len() == 1 && ctx.abis.paths.contains_key(&segments[0].name) {
                    return RTy::Contract(segments[0].name.clone());
                }
            }
        }
        if let RTy::Contract(abi) = infer(base, ctx, locals) {
            if let Some(outputs) = ctx.abis.function_outputs(&abi, &field.name) {
                let ret = outputs.first().map_or(RTy::Unknown, |s| sol_to_rty(s));
                return RTy::Result(Box::new(ret));
            }
        }
        match field.name.as_str() {
            "toDecimal" | "toBigDecimal" => return RTy::BigDecimal,
            "toBigInt" => return RTy::BigInt,
            "abs" | "plus" | "minus" | "times" | "div" => return infer(base, ctx, locals),
            _ => {}
        }
    }
    RTy::Unknown
}

// ---- small AST helpers ----

/// A record literal's fields, as found in a constructor call's second argument.
type CtorRecord<'a> = Option<&'a [(redstart_parser::Ident, Expr)]>;

/// Detect an `Entity.loadOrCreate(id, {..})` / `Entity.create(id, {..})` /
/// `Entity.load(id)` call, returning the entity name and any record literal.
fn entity_ctor(value: &Expr) -> Option<(String, CtorRecord<'_>)> {
    let Expr::Call { callee, args, .. } = value else {
        return None;
    };
    let Expr::Field { base, field, .. } = callee.as_ref() else {
        return None;
    };
    if !matches!(field.name.as_str(), "loadOrCreate" | "create" | "load") {
        return None;
    }
    let Expr::Path { segments, .. } = base.as_ref() else {
        return None;
    };
    let entity = segments.last()?.name.clone();
    let record = match args.get(1) {
        Some(Expr::Record { fields, .. }) => Some(fields.as_slice()),
        _ => None,
    };
    Some((entity, record))
}

fn validate_type(ty: &TypeExpr, entity_names: &[String], file: &str, diags: &mut Vec<Diag>) {
    match ty {
        TypeExpr::List { elem, .. } => validate_type(elem, entity_names, file, diags),
        TypeExpr::Generic { base, args, .. } => {
            let name = base.simple_name().unwrap_or("");
            if !matches!(name, "Option" | "Id" | "List" | "Result") {
                diags.push(Diag::new(
                    file,
                    base.span(),
                    "E001",
                    format!("unknown generic type `{name}`"),
                    "not a known generic",
                ));
            }
            for a in args {
                validate_type(a, entity_names, file, diags);
            }
        }
        TypeExpr::Path { .. } => {
            let name = ty.simple_name().unwrap_or("");
            if !is_scalar(name) && !entity_names.iter().any(|n| n == name) {
                diags.push(
                    Diag::new(file, ty.span(), "E002", format!("unknown type `{name}`"), "not a scalar or entity")
                        .with_help("did you forget to declare this entity, or misspell a scalar?"),
                );
            }
        }
    }
}

fn entity_name_of(ty: &TypeExpr) -> Option<String> {
    match ty {
        TypeExpr::List { elem, .. } => entity_name_of(elem),
        TypeExpr::Path { .. } => ty.simple_name().map(str::to_string),
        TypeExpr::Generic { .. } => None,
    }
}

fn get_setting<'a>(settings: &'a [Setting], key: &str) -> Option<&'a Setting> {
    settings.iter().find(|s| s.key.name == key)
}

fn path_setting(settings: &[Setting], key: &str) -> Option<String> {
    get_setting(settings, key).and_then(|s| path_name(&s.value))
}

fn path_name(expr: &Expr) -> Option<String> {
    if let Expr::Path { segments, .. } = expr {
        segments.last().map(|s| s.name.clone())
    } else {
        None
    }
}

#[cfg(test)]
mod tests;
