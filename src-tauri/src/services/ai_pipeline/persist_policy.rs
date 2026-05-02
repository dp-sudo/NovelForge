#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersistMode {
    None,
    Formal,
    DerivedReview,
}

impl PersistMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Formal => "formal",
            Self::DerivedReview => "derived_review",
        }
    }
}

pub fn parse_persist_mode(raw: &str) -> Option<PersistMode> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "none" => Some(PersistMode::None),
        "formal" => Some(PersistMode::Formal),
        "derived_review" => Some(PersistMode::DerivedReview),
        _ => None,
    }
}

pub fn should_persist_task_output(canonical_task: &str, mode: PersistMode) -> bool {
    match mode {
        PersistMode::None => false,
        PersistMode::Formal => true,
        PersistMode::DerivedReview => is_derived_review_task(canonical_task),
    }
}

pub fn is_derived_review_task(canonical_task: &str) -> bool {
    matches!(
        canonical_task,
        "consistency.scan"
            | "timeline.review"
            | "relationship.review"
            | "dashboard.review"
            | "export.review"
    ) || canonical_task.ends_with(".review")
}

#[cfg(test)]
mod tests {
    use super::{
        is_derived_review_task, parse_persist_mode, should_persist_task_output, PersistMode,
    };

    #[test]
    fn parse_persist_mode_accepts_contract_values() {
        assert_eq!(parse_persist_mode("none"), Some(PersistMode::None));
        assert_eq!(parse_persist_mode("formal"), Some(PersistMode::Formal));
        assert_eq!(
            parse_persist_mode("derived_review"),
            Some(PersistMode::DerivedReview)
        );
        assert_eq!(
            parse_persist_mode(" derived_review "),
            Some(PersistMode::DerivedReview)
        );
    }

    #[test]
    fn parse_persist_mode_rejects_unknown_values() {
        assert_eq!(parse_persist_mode(""), None);
        assert_eq!(parse_persist_mode("legacy"), None);
        assert_eq!(parse_persist_mode("formal_review"), None);
    }

    #[test]
    fn derived_review_mode_only_persists_review_like_tasks() {
        assert!(is_derived_review_task("timeline.review"));
        assert!(is_derived_review_task("dashboard.review"));
        assert!(!is_derived_review_task("chapter.draft"));

        assert!(should_persist_task_output(
            "timeline.review",
            PersistMode::DerivedReview
        ));
        assert!(!should_persist_task_output(
            "chapter.plan",
            PersistMode::DerivedReview
        ));
        assert!(!should_persist_task_output(
            "chapter.plan",
            PersistMode::None
        ));
        assert!(should_persist_task_output(
            "chapter.plan",
            PersistMode::Formal
        ));
    }
}
