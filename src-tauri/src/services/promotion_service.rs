use uuid::Uuid;

use crate::errors::AppErrorDto;
use crate::infra::app_database::{self, PromotionPolicyRecord};

#[derive(Clone, Default)]
pub struct PromotionService;

fn promotion_invalid_input_error(message: &'static str) -> AppErrorDto {
    AppErrorDto::new("PROMOTION_POLICY_INVALID", message, true)
}

fn default_policy(target_type: &str, source_kind: &str) -> PromotionPolicyRecord {
    PromotionPolicyRecord {
        id: format!(
            "default-{}-{}",
            target_type.to_ascii_lowercase(),
            source_kind.to_ascii_lowercase()
        ),
        target_type: target_type.to_string(),
        source_kind: source_kind.to_string(),
        policy_mode: "allow".to_string(),
        require_reason: false,
        enabled: true,
        notes: Some("fallback-default-policy".to_string()),
        created_at: None,
        updated_at: None,
    }
}

fn normalize_policy_mode(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "deny" | "block" | "forbid" => "deny".to_string(),
        _ => "allow".to_string(),
    }
}

fn source_kind_matches(policy_source: &str, source_kind: &str) -> bool {
    let normalized_policy = policy_source.trim().to_ascii_lowercase();
    if normalized_policy.is_empty() || normalized_policy == "any" || normalized_policy == "*" {
        return true;
    }
    normalized_policy == source_kind.trim().to_ascii_lowercase()
}

impl PromotionService {
    pub fn list_policies(&self) -> Result<Vec<PromotionPolicyRecord>, AppErrorDto> {
        let conn = app_database::open_or_create()?;
        app_database::load_promotion_policies(&conn)
    }

    pub fn save_policy(
        &self,
        mut policy: PromotionPolicyRecord,
    ) -> Result<PromotionPolicyRecord, AppErrorDto> {
        policy.target_type = policy.target_type.trim().to_ascii_lowercase();
        policy.source_kind = if policy.source_kind.trim().is_empty() {
            "any".to_string()
        } else {
            policy.source_kind.trim().to_ascii_lowercase()
        };
        policy.policy_mode = normalize_policy_mode(&policy.policy_mode);
        if policy.target_type.is_empty() {
            return Err(promotion_invalid_input_error("targetType 不能为空"));
        }

        let now = crate::infra::time::now_iso();
        if policy.id.trim().is_empty() {
            policy.id = Uuid::new_v4().to_string();
        } else {
            policy.id = policy.id.trim().to_string();
        }

        let conn = app_database::open_or_create()?;
        app_database::upsert_promotion_policy(&conn, &policy, &now)?;
        policy.created_at = Some(now.clone());
        policy.updated_at = Some(now);
        Ok(policy)
    }

    pub fn promote<T, F>(
        &self,
        target_type: &str,
        source_kind: &str,
        reason: Option<&str>,
        action: F,
    ) -> Result<T, AppErrorDto>
    where
        F: FnOnce() -> Result<T, AppErrorDto>,
    {
        let target_type = target_type.trim().to_ascii_lowercase();
        let source_kind = source_kind.trim().to_ascii_lowercase();
        if target_type.is_empty() {
            return Err(promotion_invalid_input_error("targetType 不能为空"));
        }
        if source_kind.is_empty() {
            return Err(promotion_invalid_input_error("sourceKind 不能为空"));
        }
        let policy = self.resolve_policy(&target_type, &source_kind);
        self.validate_policy(&policy, &target_type, &source_kind, reason)?;
        action()
    }

    fn resolve_policy(&self, target_type: &str, source_kind: &str) -> PromotionPolicyRecord {
        let conn = match app_database::open_or_create() {
            Ok(conn) => conn,
            Err(_) => return default_policy(target_type, source_kind),
        };
        let policies = match app_database::load_promotion_policies(&conn) {
            Ok(rows) => rows,
            Err(_) => return default_policy(target_type, source_kind),
        };
        if let Some(exact) = policies.iter().find(|policy| {
            policy.target_type.eq_ignore_ascii_case(target_type)
                && source_kind_matches(&policy.source_kind, source_kind)
        }) {
            return exact.clone();
        }
        default_policy(target_type, source_kind)
    }

    fn validate_policy(
        &self,
        policy: &PromotionPolicyRecord,
        target_type: &str,
        source_kind: &str,
        reason: Option<&str>,
    ) -> Result<(), AppErrorDto> {
        if !policy.enabled || policy.policy_mode.eq_ignore_ascii_case("deny") {
            return Err(
                AppErrorDto::new(
                    "PROMOTION_BLOCKED_BY_POLICY",
                    "晋升策略阻止了当前晋升操作",
                    true,
                )
                .with_detail(format!(
                    "targetType={}, sourceKind={}, policyId={}",
                    target_type, source_kind, policy.id
                ))
                .with_suggested_action("请在设置页调整晋升策略后重试"),
            );
        }
        if policy.require_reason
            && reason
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .is_none()
        {
            return Err(AppErrorDto::new(
                "PROMOTION_REASON_REQUIRED",
                "当前晋升策略要求填写审核理由",
                true,
            ));
        }
        Ok(())
    }
}
