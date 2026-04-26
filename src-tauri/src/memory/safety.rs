use crate::models::{FactType, UpsertInput};

pub fn scan_for_injection(input: &UpsertInput) -> Result<(), String> {
    let text = input.value_json.to_string().to_lowercase();
    if contains_dangerous_content(&text) {
        return Err("potential_prompt_injection".to_string());
    }

    if input.fact_type == FactType::AgentRole && contains_role_injection(&text) {
        return Err("role_injection_pattern".to_string());
    }

    Ok(())
}

fn contains_dangerous_content(text: &str) -> bool {
    let patterns = [
        "ignore previous instructions",
        "do not tell the user",
        "<system>",
        "</memory-context>",
        "drop ",
        "delete from ",
        "alter ",
        "pragma ",
        "cat .env",
        "sk-",
    ];
    patterns.iter().any(|p| text.contains(p))
}

fn contains_role_injection(text: &str) -> bool {
    let patterns = [
        "you are now",
        "from now on you are",
        "forget all previous",
        "bypass",
        "developer mode",
        "dan mode",
        "越狱",
        "开发者模式",
    ];
    patterns.iter().any(|p| text.contains(p))
}
