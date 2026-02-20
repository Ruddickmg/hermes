use agent_client_protocol::{Plan, Result};
use nvim_oxi::Dictionary;

pub fn plan_event(plan: Plan) -> Result<Dictionary> {
    let mut data: nvim_oxi::Dictionary = nvim_oxi::Dictionary::new();
    data.insert("entries", format!("{:?}", plan.entries));

    if let Some(meta) = plan.meta {
        data.insert("meta", format!("{:?}", meta));
    }

    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_client_protocol::{PlanEntry, PlanEntryPriority, PlanEntryStatus};

    #[nvim_oxi::test]
    fn test_plan_event_ok() {
        let entry = PlanEntry::new(
            "Analyze codebase".to_string(),
            PlanEntryPriority::High,
            PlanEntryStatus::Pending,
        );
        let plan = Plan::new(vec![entry]);

        let result = plan_event(plan);
        assert!(result.is_ok());
    }

    #[nvim_oxi::test]
    fn test_plan_event_contains_entries() {
        let entry = PlanEntry::new(
            "Analyze codebase".to_string(),
            PlanEntryPriority::High,
            PlanEntryStatus::Pending,
        );
        let plan = Plan::new(vec![entry]);

        let result = plan_event(plan).unwrap();
        assert!(result.get("entries").is_some());
    }

    #[nvim_oxi::test]
    fn test_plan_event_with_meta() {
        let entry = PlanEntry::new(
            "Refactor code".to_string(),
            PlanEntryPriority::Medium,
            PlanEntryStatus::InProgress,
        );
        let meta: serde_json::Map<String, serde_json::Value> = serde_json::json!({"source": "llm"})
            .as_object()
            .unwrap()
            .clone();
        let plan = Plan::new(vec![entry]).meta(meta);

        let result = plan_event(plan);
        assert!(result.is_ok());
    }
}
