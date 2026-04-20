use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Message {
    pub role: Role,
    pub content: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl Message {
    pub fn system(s: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: s.into(),
            tool_calls: Vec::new(),
            tool_call_id: None,
        }
    }

    pub fn user(s: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: s.into(),
            tool_calls: Vec::new(),
            tool_call_id: None,
        }
    }

    pub fn assistant(s: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: s.into(),
            tool_calls: Vec::new(),
            tool_call_id: None,
        }
    }

    pub fn tool_result(call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: content.into(),
            tool_calls: Vec::new(),
            tool_call_id: Some(call_id.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_constructor_sets_role_and_content() {
        let m = Message::user("hi");
        assert_eq!(m.role, Role::User);
        assert_eq!(m.content, "hi");
        assert!(m.tool_calls.is_empty());
        assert!(m.tool_call_id.is_none());
    }

    #[test]
    fn tool_result_sets_call_id() {
        let m = Message::tool_result("abc", "done");
        assert_eq!(m.role, Role::Tool);
        assert_eq!(m.content, "done");
        assert_eq!(m.tool_call_id.as_deref(), Some("abc"));
    }
}
