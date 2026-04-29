use std::borrow::Cow;

pub const CORE_TASK_ROUTE_TYPES: &[&str] = &[
    "chapter.draft",
    "chapter.continue",
    "chapter.rewrite",
    "chapter.plan",
    "prose.naturalize",
    "character.create",
    "world.create_rule",
    "consistency.scan",
    "blueprint.generate_step",
    "plot.create_node",
    "glossary.create_term",
    "narrative.create_obligation",
    "timeline.review",
    "relationship.review",
    "dashboard.review",
    "export.review",
];

pub const TASK_ROUTE_TYPES_WITH_CUSTOM: &[&str] = &[
    "chapter.draft",
    "chapter.continue",
    "chapter.rewrite",
    "chapter.plan",
    "prose.naturalize",
    "character.create",
    "world.create_rule",
    "consistency.scan",
    "blueprint.generate_step",
    "plot.create_node",
    "glossary.create_term",
    "narrative.create_obligation",
    "timeline.review",
    "relationship.review",
    "dashboard.review",
    "export.review",
    "custom",
];

pub fn canonical_task_type<'a>(task_type: &'a str) -> Cow<'a, str> {
    let task_type = task_type.trim();
    match task_type {
        "chapter_draft" | "generate_chapter_draft" | "draft" | "chapter.draft" => {
            Cow::Borrowed("chapter.draft")
        }
        "chapter_continue" | "continue_chapter" | "continue_draft" | "chapter.continue" => {
            Cow::Borrowed("chapter.continue")
        }
        "chapter_rewrite" | "rewrite_selection" | "chapter.rewrite" => {
            Cow::Borrowed("chapter.rewrite")
        }
        "chapter_plan" | "plan_chapter" | "chapter.plan" => Cow::Borrowed("chapter.plan"),
        "prose_naturalize" | "deai_text" | "prose.naturalize" => Cow::Borrowed("prose.naturalize"),
        "character_create" | "character.create" => Cow::Borrowed("character.create"),
        "world.generate" | "world_create_rule" | "world.create_rule" => {
            Cow::Borrowed("world.create_rule")
        }
        "plot.generate" | "plot_create_node" | "plot.create_node" => {
            Cow::Borrowed("plot.create_node")
        }
        "glossary_create_term" | "glossary.create" | "glossary.create_term" => {
            Cow::Borrowed("glossary.create_term")
        }
        "narrative_create_obligation"
        | "narrative.create"
        | "narrative.create_obligation" => Cow::Borrowed("narrative.create_obligation"),
        "timeline_review" | "timeline.scan" | "timeline.review" => {
            Cow::Borrowed("timeline.review")
        }
        "relationship_review" | "relationships.review" | "relationship.review" => {
            Cow::Borrowed("relationship.review")
        }
        "dashboard_review" | "dashboard.analyze" | "dashboard.review" => {
            Cow::Borrowed("dashboard.review")
        }
        "export_review" | "export.check" | "export.review" => Cow::Borrowed("export.review"),
        "scan_consistency" | "consistency_scan" | "consistency.scan" => {
            Cow::Borrowed("consistency.scan")
        }
        "generate_blueprint_step" | "blueprint_generate" | "blueprint.generate_step" => {
            Cow::Borrowed("blueprint.generate_step")
        }
        "custom" => Cow::Borrowed("custom"),
        _ => Cow::Borrowed(task_type),
    }
}

pub fn is_core_task_type(task_type: &str) -> bool {
    let canonical = canonical_task_type(task_type);
    CORE_TASK_ROUTE_TYPES.contains(&canonical.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalize_legacy_aliases() {
        assert_eq!(
            canonical_task_type("generate_chapter_draft"),
            "chapter.draft"
        );
        assert_eq!(canonical_task_type("scan_consistency"), "consistency.scan");
        assert_eq!(
            canonical_task_type("generate_blueprint_step"),
            "blueprint.generate_step"
        );
        assert_eq!(canonical_task_type("glossary.create"), "glossary.create_term");
        assert_eq!(
            canonical_task_type("narrative_create_obligation"),
            "narrative.create_obligation"
        );
        assert_eq!(canonical_task_type("relationships.review"), "relationship.review");
        assert_eq!(canonical_task_type("timeline.scan"), "timeline.review");
        assert_eq!(canonical_task_type("dashboard.analyze"), "dashboard.review");
        assert_eq!(canonical_task_type("export.check"), "export.review");
    }

    #[test]
    fn canonicalize_keeps_unknown() {
        assert_eq!(canonical_task_type("my.custom.skill"), "my.custom.skill");
    }

    #[test]
    fn core_task_type_check() {
        assert!(is_core_task_type("chapter_draft"));
        assert!(is_core_task_type("chapter.draft"));
        assert!(!is_core_task_type("custom"));
        assert!(!is_core_task_type("my.custom.skill"));
    }
}
