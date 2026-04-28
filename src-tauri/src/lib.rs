mod adapters;
mod commands;
mod domain;
mod errors;
mod infra;
mod services;
mod state;

use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // ── Logging: stdout with Debug level ──
            // File logging is handled by infra::logger::log_to_file() for key events
            app.handle().plugin(
                tauri_plugin_log::Builder::default()
                    .level(log::LevelFilter::Debug)
                    .timezone_strategy(tauri_plugin_log::TimezoneStrategy::UseLocal)
                    .build(),
            )?;

            let version = app.package_info().version.to_string();
            crate::infra::logger::log_startup(&version);
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;

            // ══ Deferred provider preload (best-effort, never blocks startup) ══
            let ai_service = app.state::<AppState>().ai_service.clone();
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                match crate::infra::app_database::open_or_create() {
                    Ok(conn) => {
                        match crate::infra::app_database::load_all_providers(&conn) {
                            Ok(providers) => {
                                let count = providers.len();
                                for provider in &providers {
                                    if let Err(e) = ai_service.reload_provider(&provider.id).await {
                                        log::warn!(
                                            "[PRELOAD] Failed to reload provider '{}': {}",
                                            provider.id,
                                            e.message
                                        );
                                    }
                                }
                                if count > 0 {
                                    log::info!("[PRELOAD] Pre-loaded {} provider adapter(s)", count);
                                }
                            }
                            Err(e) => {
                                log::warn!("[PRELOAD] Cannot list providers: {}", e.message);
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("[PRELOAD] Cannot open app database: {}", e.message);
                    }
                }
            });

            Ok(())
        })
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::project_commands::validate_project,
            commands::project_commands::create_project,
            commands::project_commands::open_project,
            commands::project_commands::list_recent_projects,
            commands::project_commands::init_project_repository,
            commands::project_commands::get_project_repository_status,
            commands::project_commands::commit_project_snapshot,
            commands::project_commands::list_project_history,
            commands::chapter_commands::list_chapters,
            commands::chapter_commands::list_timeline_entries,
            commands::chapter_commands::reorder_chapters,
            commands::chapter_commands::create_chapter,
            commands::chapter_commands::save_chapter_content,
            commands::chapter_commands::autosave_draft,
            commands::chapter_commands::recover_draft,
            commands::chapter_commands::delete_chapter,
            commands::export_commands::export_chapter,
            commands::export_commands::export_book,
            commands::blueprint_commands::list_blueprint_steps,
            commands::blueprint_commands::save_blueprint_step,
            commands::blueprint_commands::mark_blueprint_completed,
            commands::blueprint_commands::reset_blueprint_step,
            commands::character_commands::list_characters,
            commands::character_commands::create_character,
            commands::character_commands::update_character,
            commands::character_commands::delete_character,
            commands::character_commands::list_character_relationships,
            commands::character_commands::create_character_relationship,
            commands::character_commands::delete_character_relationship,
            commands::world_commands::list_world_rules,
            commands::world_commands::create_world_rule,
            commands::world_commands::delete_world_rule,
            commands::glossary_commands::list_glossary_terms,
            commands::glossary_commands::create_glossary_term,
            commands::plot_commands::list_plot_nodes,
            commands::plot_commands::create_plot_node,
            commands::plot_commands::reorder_plot_nodes,
            commands::settings_commands::list_providers,
            commands::settings_commands::get_license_status,
            commands::settings_commands::activate_license,
            commands::settings_commands::check_app_update,
            commands::settings_commands::install_app_update,
            commands::settings_commands::save_provider,
            commands::settings_commands::load_provider,
            commands::settings_commands::delete_provider,
            commands::settings_commands::refresh_provider_models,
            commands::settings_commands::get_provider_models,
            commands::settings_commands::get_refresh_logs,
            commands::settings_commands::list_task_routes,
            commands::settings_commands::save_task_route,
            commands::settings_commands::delete_task_route,
            commands::settings_commands::check_remote_registry,
            commands::settings_commands::apply_registry_update,
            commands::settings_commands::load_provider_config,
            commands::settings_commands::save_provider_config,
            commands::settings_commands::test_provider_connection,
            commands::settings_commands::load_editor_settings,
            commands::settings_commands::save_editor_settings,
            commands::ai_commands::generate_ai_preview,
            commands::ai_commands::stream_ai_generate,
            commands::ai_commands::stream_ai_chapter_task,
            commands::ai_commands::register_ai_provider,
            commands::ai_commands::test_ai_connection,
            commands::ai_commands::list_skills,
            commands::ai_commands::generate_blueprint_suggestion,
            commands::ai_commands::ai_generate_character,
            commands::ai_commands::ai_scan_consistency,
            commands::ai_commands::ai_generate_world_rule,
            commands::ai_commands::ai_generate_plot_node,
            commands::consistency_commands::scan_chapter_consistency,
            commands::consistency_commands::list_consistency_issues,
            commands::consistency_commands::update_issue_status,
            commands::context_commands::get_chapter_context,
            commands::dashboard_commands::get_dashboard_stats,
            commands::search_commands::search_project,
            commands::search_commands::search_project_semantic,
            commands::search_commands::rebuild_search_index,
            commands::search_commands::rebuild_vector_index,
            commands::search_commands::check_project_integrity,
            commands::narrative_commands::list_narrative_obligations,
            commands::narrative_commands::create_narrative_obligation,
            commands::narrative_commands::update_obligation_status,
            commands::narrative_commands::delete_narrative_obligation,
            commands::chapter_commands::create_snapshot,
            commands::chapter_commands::list_snapshots,
            commands::chapter_commands::read_snapshot_content,
            commands::chapter_commands::list_volumes,
            commands::chapter_commands::create_volume,
            commands::chapter_commands::delete_volume,
            commands::chapter_commands::assign_chapter_volume,
            commands::import_commands::import_chapter_files,
            commands::import_commands::create_backup,
            commands::import_commands::list_backups,
            commands::import_commands::restore_backup,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
