use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStage {
    Draft,
    Revision,
    Polish,
}

impl ProjectStage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Revision => "revision",
            Self::Polish => "polish",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "revision" => Self::Revision,
            "polish" => Self::Polish,
            _ => Self::Draft,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

impl RiskLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "low" => Self::Low,
            "high" => Self::High,
            _ => Self::Medium,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutingStrategyTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub project_stage: ProjectStage,
    pub task_risk_level: RiskLevel,
    pub recommended_pools: HashMap<String, String>,
}
