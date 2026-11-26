//! # The requirements of the elements of a build
//!
//! The decisions, the business knowledge models and the decision services of the models that are
//! built together, with what each requires. An element whose requirements are not well formed is
//! not built, and neither is an element that requires one that is not built; every other element
//! is built and evaluates. Each element that is not built has a diagnostic that says why, and its
//! evaluation is null with that reason.
//!
//! Checked before anything is built: every reference of a requirement leads to an element of the
//! kind that the requirement takes (DMN 1.6 §6.2.3, Table 2), and the requirements form no cycle
//! (§6.2: a decision requirements graph is acyclic). The build of a model follows its
//! requirements, so a cycle is found here, where it is a walk of a graph with a stack of its own.

use crate::model_definitions::{DefDefinitions, DefHRef, DefKey};
use std::collections::{BTreeMap, HashMap};
use std::fmt;

/// How bad what a diagnostic of a build says is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildSeverity {
  /// The element is not built: its evaluation is null with the message.
  Error,
  /// The element is built and evaluates.
  Warning,
}

/// The kind of an element of a decision requirements graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ElementKind {
  Decision,
  BusinessKnowledgeModel,
  DecisionService,
  InputData,
}

impl fmt::Display for ElementKind {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(match self {
      ElementKind::Decision => "decision",
      ElementKind::BusinessKnowledgeModel => "business knowledge model",
      ElementKind::DecisionService => "decision service",
      ElementKind::InputData => "input data",
    })
  }
}

/// What the build of the models says of one of their elements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildDiagnostic {
  pub severity: BuildSeverity,
  /// What it is about, for a program: stable, where the message can change.
  pub code: &'static str,
  pub kind: ElementKind,
  /// The namespace of the model of the element.
  pub namespace: String,
  pub id: String,
  pub name: String,
  pub message: String,
}

/// The requirement of an element: the element it leads to, and whether the evaluation of the
/// element evaluates that one (an input decision of a decision service is given, not evaluated).
struct Requirement {
  target: DefKey,
  evaluated: bool,
}

/// An element with what it requires.
struct Element {
  kind: ElementKind,
  name: String,
  requirements: Vec<Requirement>,
}

/// Why an element is not built.
struct Reason {
  code: &'static str,
  message: String,
  /// The message of the element where the reason began: its own for that element.
  root: String,
}

/// The elements of a build, and those of them that are not built.
#[derive(Default)]
pub struct Requirements {
  /// By namespace and id, in that order: every answer is in one order.
  elements: BTreeMap<(String, String), Element>,
  unbuilt: HashMap<DefKey, Reason>,
  warnings: Vec<BuildDiagnostic>,
}

fn place(key: &DefKey) -> (String, String) {
  (key.namespace().to_string(), key.id().to_string())
}

impl Requirements {
  /// The elements of `definitions` with their requirements, checked: the elements whose
  /// references lead nowhere or to an element of another kind, those in a cycle of requirements,
  /// and those that require one of these are not built.
  pub fn new(definitions: &DefDefinitions) -> Self {
    let mut requirements = Requirements::default();
    let kind_of = |href: &DefHRef| -> Option<(ElementKind, String)> {
      let (namespace, id) = (href.namespace(), href.id());
      None
        .or_else(|| definitions.decision_by_key(namespace, id).map(|found| (ElementKind::Decision, found.name().to_string())))
        .or_else(|| {
          definitions
            .business_knowledge_model_by_key(namespace, id)
            .map(|found| (ElementKind::BusinessKnowledgeModel, found.name().to_string()))
        })
        .or_else(|| {
          definitions
            .decision_service_by_id(namespace, id)
            .map(|found| (ElementKind::DecisionService, found.name().to_string()))
        })
        .or_else(|| definitions.input_data_by_key(namespace, id).map(|found| (ElementKind::InputData, found.name().to_string())))
    };
    // a reference of the element `key` that is to lead to an element of one of `kinds`
    let refer = |requirements: &mut Requirements, key: &DefKey, element: &mut Element, what: &str, href: &DefHRef, kinds: &[ElementKind], evaluated: bool| {
      let expected = match kinds {
        [ElementKind::InputData] => "input data".to_string(),
        kinds => format!("a {}", kinds.iter().map(ToString::to_string).collect::<Vec<_>>().join(" or a ")),
      };
      let message = match kind_of(href) {
        Some((kind, _)) if kinds.contains(&kind) => {
          if kind != ElementKind::InputData {
            element.requirements.push(Requirement { target: href.into(), evaluated });
          }
          return;
        }
        Some((kind, name)) => format!(
          "{} '{}': expected its {what} '{}' to be {expected}, found the {kind} '{name}'",
          element.kind,
          element.name,
          DefKey::from(href)
        ),
        None => format!(
          "{} '{}': expected its {what} '{}' to be {expected} of the models, found no element of that id",
          element.kind,
          element.name,
          DefKey::from(href)
        ),
      };
      requirements.unbuilt.entry(key.clone()).or_insert_with(|| Reason {
        code: "unknown-reference",
        root: message.clone(),
        message,
      });
    };
    use ElementKind::{BusinessKnowledgeModel, Decision, DecisionService, InputData};
    for decision in definitions.decisions() {
      let key = DefKey::new(decision.namespace(), decision.id());
      let mut element = Element {
        kind: Decision,
        name: decision.name().to_string(),
        requirements: vec![],
      };
      for requirement in decision.information_requirements() {
        if let Some(href) = requirement.required_decision() {
          refer(&mut requirements, &key, &mut element, "required decision", href, &[Decision], true);
        }
        if let Some(href) = requirement.required_input() {
          refer(&mut requirements, &key, &mut element, "required input", href, &[InputData], true);
        }
      }
      for requirement in decision.knowledge_requirements() {
        let kinds = [BusinessKnowledgeModel, DecisionService];
        refer(&mut requirements, &key, &mut element, "required knowledge", requirement.required_knowledge(), &kinds, true);
      }
      requirements.elements.insert(place(&key), element);
    }
    for knowledge in definitions.business_knowledge_models() {
      let key = DefKey::new(knowledge.namespace(), knowledge.id());
      let mut element = Element {
        kind: BusinessKnowledgeModel,
        name: knowledge.name().to_string(),
        requirements: vec![],
      };
      for requirement in knowledge.knowledge_requirements() {
        let kinds = [BusinessKnowledgeModel, DecisionService];
        refer(&mut requirements, &key, &mut element, "required knowledge", requirement.required_knowledge(), &kinds, true);
      }
      requirements.elements.insert(place(&key), element);
    }
    for service in definitions.decision_services() {
      let key = DefKey::new(service.namespace(), service.id());
      let mut element = Element {
        kind: DecisionService,
        name: service.name().to_string(),
        requirements: vec![],
      };
      for href in service.output_decisions() {
        refer(&mut requirements, &key, &mut element, "output decision", href, &[Decision], true);
      }
      for href in service.encapsulated_decisions() {
        refer(&mut requirements, &key, &mut element, "encapsulated decision", href, &[Decision], true);
      }
      for href in service.input_decisions() {
        refer(&mut requirements, &key, &mut element, "input decision", href, &[Decision], false);
      }
      for href in service.input_data() {
        refer(&mut requirements, &key, &mut element, "input data", href, &[InputData], false);
      }
      let name = element.name.clone();
      requirements.elements.insert(place(&key), element);
      // "at least one of the encapsulatedDecisions and inputDecisions SHALL be specified"
      // (DMN 1.6 §6.3.10); services of the official tests have neither and are to evaluate
      if service.encapsulated_decisions().is_empty() && service.input_decisions().is_empty() {
        let message = format!("decision service '{name}': expected an encapsulated decision or an input decision, found neither");
        requirements.warn(&key, "service-without-decisions", message);
      }
    }
    requirements.cycles();
    requirements.spread();
    requirements
  }

  /// The elements that an evaluation of the element at `at` evaluates.
  fn evaluated(&self, at: &(String, String)) -> impl Iterator<Item = (String, String)> + '_ {
    let requirements = self.elements.get(at).map(|element| element.requirements.as_slice()).unwrap_or_default();
    requirements
      .iter()
      .filter(|requirement| requirement.evaluated)
      .map(|requirement| place(&requirement.target))
      .filter(|target| self.elements.contains_key(target))
  }

  /// Marks the elements that are in a cycle of requirements: each with one cycle through it.
  /// A walk with its own stack, each element entered once.
  fn cycles(&mut self) {
    #[derive(Clone, Copy, PartialEq)]
    enum State {
      New,
      Open,
      Done,
    }
    let places: Vec<(String, String)> = self.elements.keys().cloned().collect();
    let index: HashMap<&(String, String), usize> = places.iter().enumerate().map(|(index, place)| (place, index)).collect();
    let edges: Vec<Vec<usize>> = places
      .iter()
      .map(|place| self.evaluated(place).filter_map(|target| index.get(&target).copied()).collect())
      .collect();
    let mut state = vec![State::New; places.len()];
    let mut found: Vec<(usize, String)> = vec![];
    for start in 0..places.len() {
      if state[start] != State::New {
        continue;
      }
      // the elements being walked, each with the next of its requirements to follow
      let mut path: Vec<(usize, usize)> = vec![(start, 0)];
      state[start] = State::Open;
      while let Some(&(at, next)) = path.last() {
        let Some(&target) = edges[at].get(next) else {
          state[at] = State::Done;
          path.pop();
          continue;
        };
        if let Some(last) = path.last_mut() {
          last.1 += 1;
        }
        match state[target] {
          State::New => {
            state[target] = State::Open;
            path.push((target, 0));
          }
          // back to an element of the path: the cycle is the path from there
          State::Open => {
            let from = path.iter().position(|(member, _)| *member == target).unwrap_or(0);
            let members: Vec<usize> = path[from..].iter().map(|(member, _)| *member).collect();
            let mut names: Vec<&str> = members.iter().map(|member| self.elements[&places[*member]].name.as_str()).collect();
            names.push(names[0]);
            let cycle = names.join(" -> ");
            found.extend(members.into_iter().map(|member| (member, cycle.clone())));
          }
          State::Done => {}
        }
      }
    }
    for (member, cycle) in found {
      let element = &self.elements[&places[member]];
      let message = format!("{} '{}' is in a cycle of requirements: {cycle}", element.kind, element.name);
      let key = DefKey::new(&places[member].0, &places[member].1);
      self.unbuilt.entry(key).or_insert_with(|| Reason {
        code: "requirement-cycle",
        root: message.clone(),
        message,
      });
    }
  }

  /// Marks the elements that evaluate an element that is not built, and so on up.
  pub fn spread(&mut self) {
    // the elements that evaluate each element
    let mut dependents: HashMap<(String, String), Vec<(String, String)>> = HashMap::new();
    for at in self.elements.keys() {
      for target in self.evaluated(at) {
        dependents.entry(target).or_default().push(at.clone());
      }
    }
    let mut open: Vec<(String, String)> = self.elements.keys().filter(|at| self.unbuilt.contains_key(&DefKey::new(&at.0, &at.1))).cloned().collect();
    open.reverse();
    while let Some(at) = open.pop() {
      let (Some(required), Some(reason)) = (self.elements.get(&at), self.unbuilt.get(&DefKey::new(&at.0, &at.1))) else {
        continue;
      };
      let (kind, name, root) = (required.kind, required.name.clone(), reason.root.clone());
      for dependent in dependents.get(&at).map(Vec::as_slice).unwrap_or_default() {
        let key = DefKey::new(&dependent.0, &dependent.1);
        if self.unbuilt.contains_key(&key) {
          continue;
        }
        let element = &self.elements[dependent];
        let message = format!("{} '{}' requires the {kind} '{name}', which can not be evaluated: {root}", element.kind, element.name);
        self.unbuilt.insert(
          key,
          Reason {
            code: "unusable-requirement",
            message,
            root: root.clone(),
          },
        );
        open.push(dependent.clone());
      }
    }
  }

  /// Whether the element `key` is not built.
  pub fn is_unbuilt(&self, key: &DefKey) -> bool {
    self.unbuilt.contains_key(key)
  }

  /// Why the element `key` is not built, None for an element that is.
  pub fn reason(&self, key: &DefKey) -> Option<&str> {
    self.unbuilt.get(key).map(|reason| reason.message.as_str())
  }

  /// The element `key` could not be built, for `error`: it is not built.
  pub fn failed(&mut self, key: &DefKey, code: &'static str, error: impl fmt::Display) {
    let Some(element) = self.elements.get(&place(key)) else {
      return;
    };
    let message = format!("{} '{}' can not be built: {error}", element.kind, element.name);
    self.unbuilt.entry(key.clone()).or_insert_with(|| Reason {
      code,
      root: message.clone(),
      message,
    });
  }

  /// The element `key` is built, and `message` is to be said of it.
  pub fn warn(&mut self, key: &DefKey, code: &'static str, message: String) {
    if let Some(element) = self.elements.get(&place(key)) {
      self.warnings.push(BuildDiagnostic {
        severity: BuildSeverity::Warning,
        code,
        kind: element.kind,
        namespace: key.namespace().to_string(),
        id: key.id().to_string(),
        name: element.name.clone(),
        message,
      });
    }
  }

  /// What the build says of its elements: the errors in the order of the namespaces and the ids
  /// of their elements, then the warnings.
  pub fn diagnostics(&self) -> Vec<BuildDiagnostic> {
    let mut diagnostics = vec![];
    for (at, element) in &self.elements {
      if let Some(reason) = self.unbuilt.get(&DefKey::new(&at.0, &at.1)) {
        diagnostics.push(BuildDiagnostic {
          severity: BuildSeverity::Error,
          code: reason.code,
          kind: element.kind,
          namespace: at.0.clone(),
          id: at.1.clone(),
          name: element.name.clone(),
          message: reason.message.clone(),
        });
      }
    }
    diagnostics.extend(self.warnings.iter().cloned());
    diagnostics
  }
}
