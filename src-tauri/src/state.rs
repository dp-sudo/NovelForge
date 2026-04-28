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

#[derive(Default)]
pub struct AppState {
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
    pub skill_registry: SkillRegistry,
    pub vector_service: VectorService,
    pub world_service: WorldService,
}
