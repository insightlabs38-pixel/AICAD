//! [`Rule`]/[`RuleSet`]/[`evaluate`] — typed declarative configuration
//! validity rules (`AICAD-150`, `project/OWNER_DECISIONS.md#D29`).
//!
//! `docs/plan/07` §11 sketches `configuration_rules { require ...; forbid
//! ...; }` as `.aicad` surface syntax, but `DL-31` defers exact surface
//! syntax entirely and no such declaration exists yet (see
//! `crate::overlay`'s own "overlay content, not overlay syntax" note). This
//! module is the Rust-owned semantic IR that syntax would eventually
//! compile into: a small typed boolean-combinator [`Rule`] value —
//! `Equals`/`NotEquals`/`And`/`Or`/`Not`/`Implies`/`Ref` — evaluated over a
//! [`Configuration`]'s own [`Configuration::named_value`] environment.
//!
//! This is deliberately *not* a parsed textual DSL with its own
//! lexer/grammar (the "bespoke hidden rule language" `AICAD-150`'s own
//! `project/TASKS.yaml` objective warns against): every `Rule` is an
//! ordinary, fully inspectable Rust value, exactly the same "plain typed
//! IR, no separate grammar" shape `cad_assemblies::mate`/`joint` already
//! use for relations that likewise have no source syntax yet. `require X`
//! is `Rule::Equals`/`And`-shaped; `forbid X` is `Rule::Not(X)`; `require X
//! in [a, b, c]` is `Rule::Or([Equals(a), Equals(b), Equals(c)])` — no
//! dedicated "in" variant is needed.
//!
//! [`Rule::Ref`] lets one named rule reuse another (the natural way a large
//! rule set avoids duplicating a shared sub-condition); [`evaluate`]
//! detects a cyclic `Ref` chain with the same `on_stack` depth-first
//! technique `cad_assemblies::graph::expand` already uses for definition
//! cycles, rather than inventing a second cycle-detection idiom.

use cad_assemblies::ParameterValue;
use cad_diagnostics::json::Json;
use cad_diagnostics::{Diagnostic, DiagnosticCode, Severity, SeverityLetter};
use std::collections::BTreeMap;

use crate::overlay::Configuration;

/// Stable identity of one declared rule, mirroring
/// [`cad_assemblies::MateId`]'s "name a stable source-level binding"
/// precedent.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RuleName(String);

impl RuleName {
    pub fn named(name: impl Into<String>) -> RuleName {
        RuleName(name.into())
    }

    pub fn name(&self) -> &str {
        &self.0
    }

    pub fn to_json(&self) -> Json {
        Json::object([
            ("kind".to_string(), Json::str("configuration_rule")),
            ("name".to_string(), Json::str(self.0.clone())),
        ])
    }
}

/// A typed boolean-combinator validity condition — see this module's own
/// doc comment for why this is not a bespoke textual rule language.
#[derive(Debug, Clone, PartialEq)]
pub enum Rule {
    /// The named configuration value equals exactly this value (missing
    /// is never treated as equal to anything).
    Equals(String, ParameterValue),
    /// The named configuration value is present and differs from this
    /// value (missing never satisfies `NotEquals` either — a rule should
    /// never be vacuously satisfied by an absent value it never checked).
    NotEquals(String, ParameterValue),
    And(Vec<Rule>),
    /// Never vacuously true: an empty `Or` is unsatisfiable, matching
    /// ordinary logical convention (a disjunction over zero alternatives
    /// has no way to be true).
    Or(Vec<Rule>),
    Not(Box<Rule>),
    /// `docs/plan/07`'s `forbid A && B` is `Not(And([A, B]))`; a
    /// conditional requirement (`if condition, then requirement`) is
    /// `Implies(condition, requirement)`.
    Implies(Box<Rule>, Box<Rule>),
    /// Reuses another named rule from the same [`RuleSet`].
    Ref(RuleName),
}

/// Every way [`RuleSet::define`] can fail.
#[derive(Debug, Clone, PartialEq)]
pub enum RuleSetError {
    /// `name` was already defined with different content — mirrors
    /// [`cad_assemblies::RegistryError::Conflicting`]'s own "same name,
    /// different content" fail-closed precedent; identical redefinition is
    /// accepted.
    Conflicting(RuleName),
}

/// Stores each named [`Rule`] exactly once, in a `BTreeMap` so iteration
/// (and therefore [`evaluate`]'s own violation order) depends only on
/// [`RuleName`]'s `Ord`, never on declaration/insertion order.
#[derive(Debug, Clone, Default)]
pub struct RuleSet {
    rules: BTreeMap<RuleName, Rule>,
}

impl RuleSet {
    pub fn new() -> RuleSet {
        RuleSet {
            rules: BTreeMap::new(),
        }
    }

    pub fn define(&mut self, name: RuleName, rule: Rule) -> Result<(), RuleSetError> {
        match self.rules.get(&name) {
            Some(existing) if existing == &rule => Ok(()),
            Some(_) => Err(RuleSetError::Conflicting(name)),
            None => {
                self.rules.insert(name, rule);
                Ok(())
            }
        }
    }

    pub fn get(&self, name: &RuleName) -> Option<&Rule> {
        self.rules.get(name)
    }

    /// Every declared rule, in deterministic (`RuleName` `Ord`) order.
    pub fn iter(&self) -> impl Iterator<Item = (&RuleName, &Rule)> {
        self.rules.iter()
    }

    pub fn len(&self) -> usize {
        self.rules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

/// Every way [`evaluate`] can fail closed rather than reporting a
/// (possibly wrong) pass/fail verdict.
#[derive(Debug, Clone, PartialEq)]
pub enum RuleError {
    /// A [`Rule::Ref`] named a rule `RuleSet` has no entry for.
    UndefinedRule(RuleName),
    /// Evaluating `chain[0]` transitively required evaluating `chain[0]`
    /// again before finishing — root-to-closing-repeat order, mirroring
    /// [`cad_assemblies::AssemblyGraphError::CyclicDefinition`]'s own
    /// chain-reporting shape exactly.
    CyclicRule { chain: Vec<RuleName> },
}

impl RuleError {
    pub fn to_diagnostic(&self) -> Diagnostic {
        match self {
            RuleError::UndefinedRule(name) => Diagnostic::new(
                DiagnosticCode::new("ASM", SeverityLetter::Error, 11)
                    .expect("ASM-E011 is a valid diagnostic code"),
                Severity::Error,
                "assembly",
                "Undefined configuration rule",
                format!(
                    "configuration rule '{}' is referenced but never defined",
                    name.name()
                ),
            )
            .expect("severity matches code letter")
            .with_entity(name.name()),
            RuleError::CyclicRule { chain } => {
                let names: Vec<&str> = chain.iter().map(RuleName::name).collect();
                Diagnostic::new(
                    DiagnosticCode::new("ASM", SeverityLetter::Error, 12)
                        .expect("ASM-E012 is a valid diagnostic code"),
                    Severity::Error,
                    "assembly",
                    "Cyclic configuration rule reference",
                    format!(
                        "cyclic configuration rule reference: {}",
                        names.join(" -> ")
                    ),
                )
                .expect("severity matches code letter")
            }
        }
    }
}

/// One rule that failed to hold under a particular [`Configuration`],
/// carried as structured evidence rather than a bare boolean.
#[derive(Debug, Clone, PartialEq)]
pub struct RuleViolation {
    rule: RuleName,
}

impl RuleViolation {
    pub fn rule(&self) -> &RuleName {
        &self.rule
    }

    pub fn to_diagnostic(&self) -> Diagnostic {
        Diagnostic::new(
            DiagnosticCode::new("ASM", SeverityLetter::Error, 10)
                .expect("ASM-E010 is a valid diagnostic code"),
            Severity::Error,
            "assembly",
            "Configuration rule violated",
            format!("configuration rule '{}' is not satisfied", self.rule.name()),
        )
        .expect("severity matches code letter")
        .with_entity(self.rule.name())
    }
}

/// [`evaluate`]'s result: every rule in `rules`, evaluated in deterministic
/// [`RuleSet`] order, that did not hold.
#[derive(Debug, Clone, PartialEq)]
pub struct RuleEvaluation {
    violations: Vec<RuleViolation>,
}

impl RuleEvaluation {
    pub fn is_valid(&self) -> bool {
        self.violations.is_empty()
    }

    pub fn violations(&self) -> &[RuleViolation] {
        &self.violations
    }
}

/// Evaluates every rule in `rules` against `configuration`, in
/// [`RuleSet::iter`]'s own deterministic order, fail-closed the moment a
/// `Ref` is undefined or cyclic.
pub fn evaluate(
    rules: &RuleSet,
    configuration: &Configuration,
) -> Result<RuleEvaluation, RuleError> {
    let mut violations = Vec::new();
    for (name, rule) in rules.iter() {
        // Seed `on_stack` with the rule currently being evaluated, exactly
        // as `cad_assemblies::graph::expand_into` seeds it with the
        // definition currently being expanded, so a `Ref` chain that
        // eventually points back to `name` itself is detected as a cycle
        // rooted at `name` rather than at whichever `Ref` happened to
        // close the loop.
        let mut on_stack = vec![name.clone()];
        if !eval(rule, rules, configuration, &mut on_stack)? {
            violations.push(RuleViolation { rule: name.clone() });
        }
    }
    Ok(RuleEvaluation { violations })
}

fn eval(
    rule: &Rule,
    rules: &RuleSet,
    configuration: &Configuration,
    on_stack: &mut Vec<RuleName>,
) -> Result<bool, RuleError> {
    match rule {
        Rule::Equals(key, expected) => Ok(configuration.named_value(key) == Some(*expected)),
        Rule::NotEquals(key, expected) => Ok(configuration
            .named_value(key)
            .is_some_and(|value| value != *expected)),
        Rule::And(rules_) => {
            for r in rules_ {
                if !eval(r, rules, configuration, on_stack)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        Rule::Or(rules_) => {
            for r in rules_ {
                if eval(r, rules, configuration, on_stack)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Rule::Not(r) => Ok(!eval(r, rules, configuration, on_stack)?),
        Rule::Implies(condition, requirement) => {
            if eval(condition, rules, configuration, on_stack)? {
                eval(requirement, rules, configuration, on_stack)
            } else {
                Ok(true)
            }
        }
        Rule::Ref(name) => {
            if let Some(start) = on_stack.iter().position(|n| n == name) {
                let mut chain: Vec<RuleName> = on_stack[start..].to_vec();
                chain.push(name.clone());
                return Err(RuleError::CyclicRule { chain });
            }
            let target = rules
                .get(name)
                .ok_or_else(|| RuleError::UndefinedRule(name.clone()))?;
            on_stack.push(name.clone());
            let result = eval(target, rules, configuration, on_stack);
            on_stack.pop();
            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_assemblies::ParameterValue;
    use cad_types::PrimitiveType;
    use cad_units::OperandType;

    fn int(n: f64) -> ParameterValue {
        ParameterValue::new(n, OperandType::Scalar(PrimitiveType::Int))
    }

    fn configuration_with_ratio(ratio: f64) -> Configuration {
        let mut configuration = Configuration::new(crate::ConfigurationId::named("Product"));
        configuration.set_named_value("ratio", int(ratio));
        configuration
    }

    #[test]
    fn a_satisfied_equals_rule_produces_no_violation() {
        let mut rules = RuleSet::new();
        rules
            .define(
                RuleName::named("ratio_is_ten"),
                Rule::Equals("ratio".into(), int(10.0)),
            )
            .unwrap();
        let evaluation = evaluate(&rules, &configuration_with_ratio(10.0)).unwrap();
        assert!(evaluation.is_valid());
    }

    #[test]
    fn a_violated_rule_is_reported_by_name() {
        let mut rules = RuleSet::new();
        rules
            .define(
                RuleName::named("ratio_is_ten"),
                Rule::Equals("ratio".into(), int(10.0)),
            )
            .unwrap();
        let evaluation = evaluate(&rules, &configuration_with_ratio(20.0)).unwrap();
        assert!(!evaluation.is_valid());
        assert_eq!(evaluation.violations().len(), 1);
        assert_eq!(
            evaluation.violations()[0].rule(),
            &RuleName::named("ratio_is_ten")
        );
    }

    #[test]
    fn require_ratio_in_a_set_is_expressed_with_or() {
        let mut rules = RuleSet::new();
        rules
            .define(
                RuleName::named("valid_ratio"),
                Rule::Or(vec![
                    Rule::Equals("ratio".into(), int(5.0)),
                    Rule::Equals("ratio".into(), int(10.0)),
                    Rule::Equals("ratio".into(), int(20.0)),
                ]),
            )
            .unwrap();
        assert!(
            evaluate(&rules, &configuration_with_ratio(10.0))
                .unwrap()
                .is_valid()
        );
        assert!(
            !evaluate(&rules, &configuration_with_ratio(7.0))
                .unwrap()
                .is_valid()
        );
    }

    #[test]
    fn forbid_a_combination_is_expressed_with_not_and() {
        // forbid motor == NEMA23 && material == Plastic
        let motor_key = "motor";
        let material_key = "material";
        let nema23 = int(23.0);
        let plastic = int(1.0);
        let mut rules = RuleSet::new();
        rules
            .define(
                RuleName::named("no_nema23_with_plastic"),
                Rule::Not(Box::new(Rule::And(vec![
                    Rule::Equals(motor_key.into(), nema23),
                    Rule::Equals(material_key.into(), plastic),
                ]))),
            )
            .unwrap();

        let mut forbidden = Configuration::new(crate::ConfigurationId::named("Product"));
        forbidden.set_named_value(motor_key, nema23);
        forbidden.set_named_value(material_key, plastic);
        assert!(!evaluate(&rules, &forbidden).unwrap().is_valid());

        let mut allowed = Configuration::new(crate::ConfigurationId::named("Product"));
        allowed.set_named_value(motor_key, nema23);
        allowed.set_named_value(material_key, int(2.0));
        assert!(evaluate(&rules, &allowed).unwrap().is_valid());
    }

    #[test]
    fn implies_only_requires_the_consequent_when_the_condition_holds() {
        let mut rules = RuleSet::new();
        rules
            .define(
                RuleName::named("encoder_requires_high_ratio"),
                Rule::Implies(
                    Box::new(Rule::Equals("has_encoder".into(), int(1.0))),
                    Box::new(Rule::Equals("ratio".into(), int(20.0))),
                ),
            )
            .unwrap();

        let mut no_encoder = configuration_with_ratio(5.0);
        no_encoder.set_named_value("has_encoder", int(0.0));
        assert!(evaluate(&rules, &no_encoder).unwrap().is_valid());

        let mut with_encoder_low_ratio = configuration_with_ratio(5.0);
        with_encoder_low_ratio.set_named_value("has_encoder", int(1.0));
        assert!(
            !evaluate(&rules, &with_encoder_low_ratio)
                .unwrap()
                .is_valid()
        );
    }

    #[test]
    fn a_missing_named_value_never_satisfies_equals_or_not_equals() {
        let mut rules = RuleSet::new();
        rules
            .define(
                RuleName::named("r1"),
                Rule::Equals("missing".into(), int(1.0)),
            )
            .unwrap();
        rules
            .define(
                RuleName::named("r2"),
                Rule::NotEquals("missing".into(), int(1.0)),
            )
            .unwrap();
        let evaluation = evaluate(
            &rules,
            &Configuration::new(crate::ConfigurationId::named("Empty")),
        )
        .unwrap();
        assert_eq!(
            evaluation.violations().len(),
            2,
            "neither rule is vacuously satisfied"
        );
    }

    #[test]
    fn a_rule_can_reuse_another_named_rule_via_ref() {
        let mut rules = RuleSet::new();
        rules
            .define(
                RuleName::named("base"),
                Rule::Equals("ratio".into(), int(10.0)),
            )
            .unwrap();
        rules
            .define(
                RuleName::named("derived"),
                Rule::Ref(RuleName::named("base")),
            )
            .unwrap();
        assert!(
            evaluate(&rules, &configuration_with_ratio(10.0))
                .unwrap()
                .is_valid()
        );
        assert!(
            !evaluate(&rules, &configuration_with_ratio(1.0))
                .unwrap()
                .is_valid()
        );
    }

    #[test]
    fn a_reference_to_an_undefined_rule_fails_closed() {
        let mut rules = RuleSet::new();
        rules
            .define(
                RuleName::named("derived"),
                Rule::Ref(RuleName::named("missing")),
            )
            .unwrap();
        let err = evaluate(&rules, &configuration_with_ratio(10.0)).unwrap_err();
        assert_eq!(err, RuleError::UndefinedRule(RuleName::named("missing")));
        assert_eq!(err.to_diagnostic().code.as_string(), "ASM-E011");
    }

    #[test]
    fn a_cyclic_rule_reference_fails_closed() {
        let mut rules = RuleSet::new();
        rules
            .define(RuleName::named("a"), Rule::Ref(RuleName::named("b")))
            .unwrap();
        rules
            .define(RuleName::named("b"), Rule::Ref(RuleName::named("a")))
            .unwrap();
        let err = evaluate(&rules, &configuration_with_ratio(10.0)).unwrap_err();
        assert_eq!(
            err,
            RuleError::CyclicRule {
                chain: vec![
                    RuleName::named("a"),
                    RuleName::named("b"),
                    RuleName::named("a")
                ]
            }
        );
        let diagnostic = err.to_diagnostic();
        assert_eq!(diagnostic.code.as_string(), "ASM-E012");
        assert!(diagnostic.message.contains("a -> b -> a"));
    }

    #[test]
    fn evaluation_order_is_deterministic_regardless_of_declaration_order() {
        let mut forward = RuleSet::new();
        forward
            .define(
                RuleName::named("a"),
                Rule::Equals("ratio".into(), int(999.0)),
            )
            .unwrap();
        forward
            .define(
                RuleName::named("b"),
                Rule::Equals("ratio".into(), int(999.0)),
            )
            .unwrap();

        let mut backward = RuleSet::new();
        backward
            .define(
                RuleName::named("b"),
                Rule::Equals("ratio".into(), int(999.0)),
            )
            .unwrap();
        backward
            .define(
                RuleName::named("a"),
                Rule::Equals("ratio".into(), int(999.0)),
            )
            .unwrap();

        let forward_names: Vec<_> = evaluate(&forward, &configuration_with_ratio(1.0))
            .unwrap()
            .violations()
            .iter()
            .map(RuleViolation::rule)
            .cloned()
            .collect();
        let backward_names: Vec<_> = evaluate(&backward, &configuration_with_ratio(1.0))
            .unwrap()
            .violations()
            .iter()
            .map(RuleViolation::rule)
            .cloned()
            .collect();
        assert_eq!(forward_names, backward_names);
    }

    #[test]
    fn redefining_a_rule_with_different_content_under_the_same_name_is_rejected() {
        let mut rules = RuleSet::new();
        rules
            .define(
                RuleName::named("r"),
                Rule::Equals("ratio".into(), int(10.0)),
            )
            .unwrap();
        assert_eq!(
            rules.define(
                RuleName::named("r"),
                Rule::Equals("ratio".into(), int(20.0))
            ),
            Err(RuleSetError::Conflicting(RuleName::named("r")))
        );
    }

    #[test]
    fn serialization_is_deterministic() {
        let name = RuleName::named("ratio_is_ten");
        assert_eq!(
            name.to_json().to_canonical_string(),
            name.to_json().to_canonical_string()
        );
    }
}
