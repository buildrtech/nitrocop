use std::collections::HashMap;

use super::Cop;

pub struct CopRegistry {
    cops: Vec<Box<dyn Cop>>,
    names: Vec<&'static str>,
    supports_autocorrect: Vec<bool>,
    uses_node_check: Vec<bool>,
    index: HashMap<&'static str, usize>,
}

impl CopRegistry {
    // Default impl would hide the intentional empty-registry construction.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            cops: Vec::new(),
            names: Vec::new(),
            supports_autocorrect: Vec::new(),
            uses_node_check: Vec::new(),
            index: HashMap::new(),
        }
    }

    /// Build the default registry with all built-in cops.
    pub fn default_registry() -> Self {
        let mut registry = Self::new();
        super::bundler::register_all(&mut registry);
        super::factory_bot::register_all(&mut registry);
        super::gemspec::register_all(&mut registry);
        super::layout::register_all(&mut registry);
        super::migration::register_all(&mut registry);
        super::lint::register_all(&mut registry);
        super::metrics::register_all(&mut registry);
        super::naming::register_all(&mut registry);
        super::performance::register_all(&mut registry);
        super::rails::register_all(&mut registry);
        super::rspec::register_all(&mut registry);
        super::rspec_rails::register_all(&mut registry);
        super::security::register_all(&mut registry);
        super::standard::register_all(&mut registry);
        super::style::register_all(&mut registry);
        registry
    }

    pub fn register(&mut self, cop: Box<dyn Cop>) {
        let name = cop.name();
        let supports_autocorrect = cop.supports_autocorrect();
        let uses_node_check = cop.uses_node_check();
        let idx = self.cops.len();
        self.names.push(name);
        self.supports_autocorrect.push(supports_autocorrect);
        self.uses_node_check.push(uses_node_check);
        self.cops.push(cop);
        self.index.insert(name, idx);
    }

    pub fn cops(&self) -> &[Box<dyn Cop>] {
        &self.cops
    }

    pub fn get(&self, name: &str) -> Option<&dyn Cop> {
        self.index.get(name).map(|&idx| &*self.cops[idx])
    }

    pub fn cop_index(&self, name: &str) -> Option<usize> {
        self.index.get(name).copied()
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.names.clone()
    }

    pub fn cop_name(&self, idx: usize) -> &'static str {
        self.names[idx]
    }

    pub fn cop_supports_autocorrect(&self, idx: usize) -> bool {
        self.supports_autocorrect[idx]
    }

    pub fn cop_uses_node_check(&self, idx: usize) -> bool {
        self.uses_node_check[idx]
    }

    pub fn len(&self) -> usize {
        self.cops.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cops.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cop::Cop;
    use crate::diagnostic::Severity;

    struct FakeCop;

    impl Cop for FakeCop {
        fn name(&self) -> &'static str {
            "Style/Fake"
        }

        fn default_severity(&self) -> Severity {
            Severity::Warning
        }
    }

    #[test]
    fn default_registry_has_cops() {
        let reg = CopRegistry::default_registry();
        assert!(!reg.is_empty());
        assert_eq!(reg.len(), 916);
        // Spot-check cops from each department
        assert!(reg.get("Layout/TrailingWhitespace").is_some());
        assert!(reg.get("Layout/LineLength").is_some());
        assert!(reg.get("Style/FrozenStringLiteralComment").is_some());
        assert!(reg.get("Metrics/MethodLength").is_some());
        assert!(reg.get("Metrics/AbcSize").is_some());
        assert!(reg.get("Naming/MethodName").is_some());
        assert!(reg.get("Naming/FileName").is_some());
        // Performance department spot-checks
        assert!(reg.get("Performance/Detect").is_some());
        assert!(reg.get("Performance/FlatMap").is_some());
        assert!(reg.get("Performance/ReverseEach").is_some());
        assert!(reg.get("Performance/OpenStruct").is_some());
        assert!(reg.get("Performance/Count").is_some());
        assert!(reg.get("Style/EmptyMethod").is_some());
        assert!(reg.get("Lint/BooleanSymbol").is_some());
        assert!(reg.get("Lint/UnifiedInteger").is_some());
    }

    #[test]
    fn register_and_get() {
        let mut reg = CopRegistry::new();
        reg.register(Box::new(FakeCop));
        assert_eq!(reg.len(), 1);
        assert!(!reg.is_empty());

        let cop = reg.get("Style/Fake").unwrap();
        assert_eq!(cop.name(), "Style/Fake");
        assert_eq!(cop.default_severity(), Severity::Warning);
    }

    #[test]
    fn get_nonexistent() {
        let reg = CopRegistry::new();
        assert!(reg.get("Style/Nope").is_none());
    }

    #[test]
    fn names() {
        let mut reg = CopRegistry::new();
        reg.register(Box::new(FakeCop));
        assert_eq!(reg.names(), vec!["Style/Fake"]);
    }

    #[test]
    fn cops_slice() {
        let mut reg = CopRegistry::new();
        reg.register(Box::new(FakeCop));
        assert_eq!(reg.cops().len(), 1);
        assert_eq!(reg.cops()[0].name(), "Style/Fake");
    }
}
