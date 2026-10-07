//! Bounded call-time NAME guards. These are derived runtime evidence, not a
//! second function IR, parser trace, or whole-region specialization permission.
use crate::{
    Engine, Result, Value,
    frontend_context::{LookupObservation, NameGuardCheck, SimpleNameGuard},
    primitive::PrimitiveId,
    semantic::FunctionEntity,
};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct AliasRead {
    pub(crate) name: String,
    pub(crate) observation: LookupObservation,
    /// Shared immutable binding target; no operand copying or kernel execution.
    pub(crate) target: Arc<FunctionEntity>,
}

impl AliasRead {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn observation(&self) -> &LookupObservation {
        &self.observation
    }
    pub fn target(&self) -> &Arc<FunctionEntity> {
        &self.target
    }
}

#[derive(Clone, Debug)]
pub struct AliasCallGuard {
    pub(crate) context: Arc<crate::frontend_context::FrontendContext>,
    pub(crate) root: SimpleNameGuard,
    pub(crate) reads: Vec<AliasRead>,
    pub(crate) primitive: PrimitiveId,
}

impl AliasCallGuard {
    pub fn context(&self) -> &Arc<crate::frontend_context::FrontendContext> {
        &self.context
    }
    pub fn root(&self) -> &SimpleNameGuard {
        &self.root
    }
    pub fn reads(&self) -> &[AliasRead] {
        &self.reads
    }
    pub fn primitive(&self) -> PrimitiveId {
        self.primitive
    }

    pub(crate) fn call_function(&self) -> Option<&Arc<FunctionEntity>> {
        let usage = self.context.name_uses.get(self.root.origin().1.0)?;
        let node = self.context.items.get(usage.output.0)?.semantic?;
        match &self.context.nodes.get(node.0)?.kind {
            crate::frontend_context::NodeKind::Function(function) => Some(function),
            _ => None,
        }
    }

    /// Structural validation; actual binding validity is checked by Engine.
    pub fn verify(&self) -> std::result::Result<(), String> {
        use crate::{frontend_context::FoundScope, parser::ParseClass, semantic::FunctionHead};
        if self.reads.is_empty() || self.reads.len() > crate::semantic::MAX_EXPR_DEPTH + 1 {
            return Err("invalid alias read count".into());
        }
        let (unit, usage) = self.root.origin();
        if unit != self.context.unit
            || !self.context.complete
            || !self.context.name_uses.get(usage.0).is_some_and(|usage| {
                usage.lookup.as_ref() == Some(&self.root.expected)
                    && usage.policy == crate::frontend_context::NamePolicy::LateAtCall
                    && self
                        .context
                        .words
                        .get(usage.word.0)
                        .and_then(|word| word.name.as_deref())
                        == Some(self.root.name())
            })
        {
            return Err("alias context/origin mismatch".into());
        }
        if !self.call_function().is_some_and(|function| {
            matches!(&function.head, FunctionHead::NameRef(name) if name == self.root.name())
                && function.result_pos == crate::semantic::FunctionPartOfSpeech::Verb
        }) {
            return Err("alias original callee mismatch".into());
        }
        if self.reads[0].name != self.root.name || self.reads[0].observation != self.root.expected {
            return Err("alias root/origin mismatch".into());
        }
        let mut generations = std::collections::HashSet::new();
        for (i, read) in self.reads.iter().enumerate() {
            let lookup = &read.observation;
            if lookup.engine != self.root.expected.engine
                || lookup.frame != self.root.expected.frame
                || lookup.binding_class != Some(ParseClass::Verb)
                || lookup.binding_version.is_none()
                || !matches!(lookup.found, FoundScope::Local(_) | FoundScope::Global(_))
                || lookup
                    .binding_generation
                    .is_none_or(|generation| !generations.insert(generation))
                || read.target.result_pos != crate::semantic::FunctionPartOfSpeech::Verb
                || !read.target.operands.is_empty()
            {
                return Err("invalid alias dependency".into());
            }
            match &read.target.head {
                FunctionHead::NameRef(next)
                    if self.reads.get(i + 1).is_some_and(|read| read.name == *next) => {}
                FunctionHead::PrimitiveVerb(id)
                    if i + 1 == self.reads.len()
                        && *id == self.primitive
                        && matches!(
                            id,
                            PrimitiveId::Add
                                | PrimitiveId::Subtract
                                | PrimitiveId::Multiply
                                | PrimitiveId::Divide
                        ) => {}
                _ => return Err("alias target/next-read mismatch".into()),
            }
        }
        Ok(())
    }
}

/// Admission failures describe coverage/staleness, not J language errors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AliasGuardAdmission {
    InvalidOrigin(String),
    RootChanged(NameGuardCheck),
    UnsupportedTarget,
    UnboundTarget(String),
    WrongPartOfSpeech(String),
    Cycle(String),
    DepthLimit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AliasGuardCheck {
    InvalidRecipe,
    ValidAtCheck,
    Invalidated { read: usize, reason: NameGuardCheck },
}

/// Call-ready boundary: arguments already reflect their J read/effect order.
/// Owns logical values by move, sharing the original callee/context through the
/// guard. It is not a queue/stack continuation or a second executable IR.
pub struct AliasCall {
    pub(crate) guard: Arc<AliasCallGuard>,
    pub(crate) x: Option<Value>,
    pub(crate) y: Value,
}

impl AliasCall {
    pub fn new(guard: Arc<AliasCallGuard>, x: Option<Value>, y: Value) -> Self {
        Self { guard, x, y }
    }
    pub fn guard(&self) -> &Arc<AliasCallGuard> {
        &self.guard
    }
    pub fn left(&self) -> Option<&Value> {
        self.x.as_ref()
    }
    pub fn right(&self) -> &Value {
        &self.y
    }
}

pub struct AliasCallMiss {
    pub check: AliasGuardCheck,
    pub call: AliasCall,
}

pub enum AliasCallAttempt {
    /// A normal kernel/J outcome; errors are not guard misses.
    Executed(Result<Value>),
    /// Nothing called: the original callee and values are still owned here.
    Miss(AliasCallMiss),
}

/// A call-time lease for an ordinary alias chain ending in +, -, * or %.
/// Arguments must already have been evaluated in J semantic order. Borrowing
/// the Engine blocks mutation/frame changes until this one pure call completes.
/// No statement replay, automatic fallback or dispatcher is hidden here.
///
/// ```compile_fail
/// use rustj::{Engine, Value, frontend_context::NameUseId};
/// let mut engine = Engine::new();
/// engine.eval("f=:+").unwrap();
/// let observed = engine.eval_captured("f 1");
/// let guard = engine.prepare_alias_call_guard(
///     observed.capture.frontend.as_ref().unwrap(), NameUseId(0)).unwrap();
/// let lease = engine.validate_alias_call_guard(&guard).unwrap();
/// engine.eval("f=:*").unwrap(); // mutable access conflicts with the live lease
/// lease.apply_monad(Value::scalar(1)).unwrap();
/// ```
#[must_use]
pub struct ValidatedAliasTarget<'a> {
    pub(crate) _engine: &'a Engine,
    pub(crate) guard: &'a AliasCallGuard,
}

impl ValidatedAliasTarget<'_> {
    pub fn primitive(&self) -> PrimitiveId {
        self.guard.primitive
    }

    pub fn apply_monad(self, y: Value) -> Result<Value> {
        crate::kernels::monad(self.guard.primitive.spelling(), y)
    }

    pub fn apply_dyad(self, x: Value, y: Value) -> Result<Value> {
        crate::kernels::dyad(self.guard.primitive.spelling(), x, y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend_context::NameUseId;

    #[test]
    fn damaged_dependency_recipes_cannot_issue_call_leases() {
        let mut engine = Engine::new();
        engine.eval("f=:+").unwrap();
        engine.eval("g=:f").unwrap();
        let captured = engine.eval_captured("g 2");
        let valid = engine
            .prepare_alias_call_guard(captured.capture.frontend.as_ref().unwrap(), NameUseId(0))
            .unwrap();
        for corrupt in 0..6 {
            let mut bad = valid.clone();
            match corrupt {
                0 => {
                    bad.reads.clear();
                }
                1 => {
                    bad.reads.pop();
                }
                2 => {
                    bad.reads.swap(0, 1);
                }
                3 => {
                    bad.primitive = PrimitiveId::Multiply;
                }
                4 => {
                    bad.reads[1].observation.binding_generation = None;
                }
                _ => {
                    bad.context = Arc::new(Default::default());
                }
            }
            assert!(bad.verify().is_err());
            assert_eq!(
                engine.check_alias_call_guard(&bad),
                AliasGuardCheck::InvalidRecipe
            );
            assert!(engine.validate_alias_call_guard(&bad).is_err());
            let call = AliasCall::new(Arc::new(bad), None, Value::scalar(4));
            let AliasCallAttempt::Miss(miss) = engine.try_alias_call(call) else {
                panic!()
            };
            let AliasCallAttempt::Miss(rejected) = engine.resume_alias_call(miss) else {
                panic!()
            };
            assert_eq!(rejected.call.right().int_at(0).unwrap(), 4);
        }
    }
}
