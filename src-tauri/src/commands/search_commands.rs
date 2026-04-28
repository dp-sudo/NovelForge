use tauri::State;

use crate::errors::AppErrorDto;
use crate::state::AppState;

#[tauri::command]
pub async fn search_project(
    project_root: String,
    query: String,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<crate::services::search_service::SearchResult>, AppErrorDto> {
    let cap = limit.unwrap_or(20);
    let mut results = state.search_service.search(&project_root, &query, cap)?;
    let semantic_results = state.vector_service.search(&project_root, &query, cap)?;
    for row in semantic_results {
        if results.len() >= cap {
            break;
        }
        let duplicated = results.iter().any(|existing| {
            existing.entity_type == row.entity_type
                && existing.entity_id == row.entity_id
                && existing.body_snippet == row.body_snippet
        });
        if !duplicated {
            results.push(crate::services::search_service::SearchResult {
                entity_type: row.entity_type,
                entity_id: row.entity_id,
                title: row.title,
                body_snippet: row.body_snippet,
                rank: row.rank,
            });
        }
    }
    Ok(results)
}

#[tauri::command]
pub async fn rebuild_search_index(
    project_root: String,
    state: State<'_, AppState>,
) -> Result<usize, AppErrorDto> {
    let fulltext_indexed = state.search_service.rebuild_index(&project_root)?;
    let vector_indexed = state.vector_service.rebuild_index(&project_root)?;
    Ok(fulltext_indexed + vector_indexed)
}

#[tauri::command]
pub async fn search_project_semantic(
    project_root: String,
    query: String,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<crate::services::vector_service::VectorSearchResult>, AppErrorDto> {
    state
        .vector_service
        .search(&project_root, &query, limit.unwrap_or(20))
}

#[tauri::command]
pub async fn rebuild_vector_index(
    project_root: String,
    state: State<'_, AppState>,
) -> Result<usize, AppErrorDto> {
    state.vector_service.rebuild_index(&project_root)
}

#[tauri::command]
pub async fn check_project_integrity(
    project_root: String,
    state: State<'_, AppState>,
) -> Result<crate::services::integrity_service::IntegrityReport, AppErrorDto> {
    state.integrity_service.check_project(&project_root)
}
