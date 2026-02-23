use crate::ast::ScenarioDef;
use std::collections::HashMap;

#[cfg(test)]
mod tests;

/// The result of running a scenario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScenarioResult {
    Passed,
    Failed { failures: Vec<ScenarioFailure> },
}

/// A single assertion failure in a scenario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioFailure {
    pub key: String,
    pub expected: String,
    pub actual: Option<String>,
}

/// Runs scenario tests by checking expected outcomes against actual values.
pub struct ScenarioRunner {
    name: String,
    given: HashMap<String, String>,
    expectations: Vec<(String, String)>,
}

impl ScenarioRunner {
    /// Create from a parsed scenario definition.
    #[must_use]
    pub fn from_def(def: &ScenarioDef) -> Self {
        Self {
            name: def.name.clone(),
            given: def.given.iter().cloned().collect(),
            expectations: def.expect.clone(),
        }
    }

    /// Get the scenario name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the "given" context as a map.
    #[must_use]
    pub fn context(&self) -> &HashMap<String, String> {
        &self.given
    }

    /// Run the scenario against actual results.
    /// `actuals` maps expectation keys to actual string values.
    #[must_use]
    pub fn evaluate(&self, actuals: &HashMap<String, String>) -> ScenarioResult {
        let mut failures = Vec::new();

        for (key, expected) in &self.expectations {
            match actuals.get(key) {
                Some(actual) if actual == expected => {}
                Some(actual) => {
                    failures.push(ScenarioFailure {
                        key: key.clone(),
                        expected: expected.clone(),
                        actual: Some(actual.clone()),
                    });
                }
                None => {
                    failures.push(ScenarioFailure {
                        key: key.clone(),
                        expected: expected.clone(),
                        actual: None,
                    });
                }
            }
        }

        if failures.is_empty() {
            ScenarioResult::Passed
        } else {
            ScenarioResult::Failed { failures }
        }
    }
}

/// Check if an expected value is present in an agent's response text.
#[must_use]
pub fn check_expectation(response: &str, expected_value: &str) -> bool {
    response
        .to_lowercase()
        .contains(&expected_value.to_lowercase())
}
