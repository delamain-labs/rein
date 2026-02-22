//! Role-based access control.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admin_has_all_permissions() {
        let rbac = RbacEngine::with_defaults();
        assert!(rbac.check("admin", "workflows", "write"));
        assert!(rbac.check("admin", "agents", "delete"));
    }

    #[test]
    fn viewer_read_only() {
        let rbac = RbacEngine::with_defaults();
        assert!(rbac.check("viewer", "workflows", "read"));
        assert!(!rbac.check("viewer", "workflows", "write"));
    }

    #[test]
    fn operator_can_execute() {
        let rbac = RbacEngine::with_defaults();
        assert!(rbac.check("operator", "workflows", "execute"));
        assert!(rbac.check("operator", "workflows", "read"));
        assert!(!rbac.check("operator", "workflows", "delete"));
    }

    #[test]
    fn assign_role() {
        let mut rbac = RbacEngine::with_defaults();
        rbac.assign_role("user1", "operator");
        assert_eq!(rbac.user_roles("user1"), vec!["operator"]);
        assert!(rbac.user_can("user1", "workflows", "execute"));
    }
}

/// A role with a set of permissions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub name: String,
    /// Permissions as `resource:action` pairs.
    pub permissions: HashSet<String>,
}

/// RBAC engine.
#[derive(Debug, Default)]
pub struct RbacEngine {
    roles: HashMap<String, Role>,
    /// User ID -> role names.
    user_roles: HashMap<String, Vec<String>>,
}

impl RbacEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an engine with default roles (admin, operator, viewer).
    pub fn with_defaults() -> Self {
        let mut engine = Self::new();
        engine.add_role(Role {
            name: "admin".to_string(),
            permissions: HashSet::from([
                "*:*".to_string(),
            ]),
        });
        engine.add_role(Role {
            name: "operator".to_string(),
            permissions: HashSet::from([
                "workflows:read".to_string(),
                "workflows:execute".to_string(),
                "agents:read".to_string(),
                "audit:read".to_string(),
            ]),
        });
        engine.add_role(Role {
            name: "viewer".to_string(),
            permissions: HashSet::from([
                "workflows:read".to_string(),
                "agents:read".to_string(),
                "audit:read".to_string(),
            ]),
        });
        engine
    }

    pub fn add_role(&mut self, role: Role) {
        self.roles.insert(role.name.clone(), role);
    }

    pub fn assign_role(&mut self, user_id: &str, role: &str) {
        self.user_roles
            .entry(user_id.to_string())
            .or_default()
            .push(role.to_string());
    }

    /// Check if a role has a specific permission.
    pub fn check(&self, role: &str, resource: &str, action: &str) -> bool {
        let Some(role_def) = self.roles.get(role) else {
            return false;
        };
        let perm = format!("{resource}:{action}");
        role_def.permissions.contains("*:*") || role_def.permissions.contains(&perm)
    }

    /// Check if a user has a permission (via any assigned role).
    pub fn user_can(&self, user_id: &str, resource: &str, action: &str) -> bool {
        self.user_roles
            .get(user_id)
            .is_some_and(|roles| roles.iter().any(|r| self.check(r, resource, action)))
    }

    /// Get roles for a user.
    pub fn user_roles(&self, user_id: &str) -> Vec<&str> {
        self.user_roles
            .get(user_id)
            .map_or_else(Vec::new, |roles| roles.iter().map(String::as_str).collect())
    }
}
