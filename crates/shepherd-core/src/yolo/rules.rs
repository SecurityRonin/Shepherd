use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSet {
    #[serde(default)]
    pub deny: Vec<Rule>,
    #[serde(default)]
    pub allow: Vec<Rule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    #[serde(default)]
    pub tool: Option<String>,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ruleset_deserialize_yaml() {
        let yaml = r#"
deny:
  - tool: Bash
    pattern: "rm -rf"
  - pattern: "sudo"
allow:
  - tool: Read
  - tool: Write
    path: "src/**"
"#;
        let rules: RuleSet = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(rules.deny.len(), 2);
        assert_eq!(rules.allow.len(), 2);
        assert_eq!(rules.deny[0].tool.as_deref(), Some("Bash"));
        assert_eq!(rules.deny[0].pattern.as_deref(), Some("rm -rf"));
        assert!(rules.deny[1].tool.is_none());
        assert_eq!(rules.allow[1].path.as_deref(), Some("src/**"));
    }

    #[test]
    fn ruleset_deserialize_empty() {
        let yaml = "{}";
        let rules: RuleSet = serde_yaml::from_str(yaml).unwrap();
        assert!(rules.deny.is_empty());
        assert!(rules.allow.is_empty());
    }

    #[test]
    fn rule_all_none_fields() {
        let yaml = "{}";
        let rule: Rule = serde_yaml::from_str(yaml).unwrap();
        assert!(rule.tool.is_none());
        assert!(rule.pattern.is_none());
        assert!(rule.path.is_none());
    }

    #[test]
    fn ruleset_serialize_roundtrip() {
        let rules = RuleSet {
            deny: vec![Rule {
                tool: Some("Bash".into()),
                pattern: Some("curl".into()),
                path: None,
            }],
            allow: vec![Rule {
                tool: Some("Read".into()),
                pattern: None,
                path: None,
            }],
        };
        let yaml = serde_yaml::to_string(&rules).unwrap();
        let parsed: RuleSet = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.deny.len(), 1);
        assert_eq!(parsed.allow.len(), 1);
        assert_eq!(parsed.deny[0].tool.as_deref(), Some("Bash"));
    }

    #[test]
    fn rule_json_roundtrip() {
        let rule = Rule {
            tool: Some("Write".into()),
            pattern: None,
            path: Some("tests/**".into()),
        };
        let json = serde_json::to_string(&rule).unwrap();
        let parsed: Rule = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.tool.as_deref(), Some("Write"));
        assert!(parsed.pattern.is_none());
        assert_eq!(parsed.path.as_deref(), Some("tests/**"));
    }
}
