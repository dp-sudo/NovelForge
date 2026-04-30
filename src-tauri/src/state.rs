use std::sync::Arc;
use std::sync::RwLock;

use crate::services::ai_pipeline_service::AiPipelineService;
use crate::services::ai_service::AiService;
use crate::services::backup_service::BackupService;
use crate::services::blueprint_service::BlueprintService;
use crate::services::chapter_service::{ChapterService, VolumeService};
use crate::services::character_service::{CharacterService, RelationshipService};
use crate::services::consistency_service::ConsistencyService;
use crate::services::context_service::ContextService;
use crate::services::dashboard_service::DashboardService;
use crate::services::export_service::ExportService;
use crate::services::git_service::GitService;
use crate::services::glossary_service::GlossaryService;
use crate::services::import_service::ImportService;
use crate::services::integrity_service::IntegrityService;
use crate::services::license_service::LicenseService;
use crate::services::model_registry_service::ModelRegistryService;
use crate::services::narrative_service::NarrativeService;
use crate::services::plot_service::PlotService;
use crate::services::project_service::ProjectService;
use crate::services::search_service::SearchService;
use crate::services::settings_service::SettingsService;
use crate::services::skill_registry::SkillRegistry;
use crate::services::vector_service::VectorService;
use crate::services::world_service::WorldService;

pub struct AppState {
    pub ai_pipeline_service: AiPipelineService,
    pub ai_service: AiService,
    pub backup_service: BackupService,
    pub blueprint_service: BlueprintService,
    pub chapter_service: ChapterService,
    pub volume_service: VolumeService,
    pub character_service: CharacterService,
    pub import_service: ImportService,
    pub relationship_service: RelationshipService,
    pub consistency_service: ConsistencyService,
    pub context_service: ContextService,
    pub dashboard_service: DashboardService,
    pub export_service: ExportService,
    pub git_service: GitService,
    pub glossary_service: GlossaryService,
    pub integrity_service: IntegrityService,
    pub license_service: LicenseService,
    pub model_registry_service: ModelRegistryService,
    pub narrative_service: NarrativeService,
    pub plot_service: PlotService,
    pub project_service: ProjectService,
    pub search_service: SearchService,
    pub settings_service: SettingsService,
    pub skill_registry: Arc<RwLock<SkillRegistry>>,
    pub vector_service: VectorService,
    pub world_service: WorldService,
}

impl AppState {
    pub fn new(skill_registry: SkillRegistry) -> Self {
        Self {
            skill_registry: Arc::new(RwLock::new(skill_registry)),
            ai_pipeline_service: AiPipelineService::default(),
            ai_service: AiService::default(),
            backup_service: BackupService,
            blueprint_service: BlueprintService,
            chapter_service: ChapterService,
            volume_service: VolumeService,
            character_service: CharacterService,
            import_service: ImportService,
            relationship_service: RelationshipService,
            consistency_service: ConsistencyService,
            context_service: ContextService,
            dashboard_service: DashboardService,
            export_service: ExportService,
            git_service: GitService,
            glossary_service: GlossaryService,
            integrity_service: IntegrityService,
            license_service: LicenseService,
            model_registry_service: ModelRegistryService,
            narrative_service: NarrativeService,
            plot_service: PlotService,
            project_service: ProjectService,
            search_service: SearchService,
            settings_service: SettingsService,
            vector_service: VectorService,
            world_service: WorldService,
        }
    }
}
