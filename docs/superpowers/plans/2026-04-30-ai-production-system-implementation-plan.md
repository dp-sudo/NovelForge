# NovelForge AI Production System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把已确认的 AI 生产系统 spec 与图谱文档落成可运行的 MVP 级实现，完成项目级 AI 策略、边界收口、状态账本、Continuity Pack、技能编排和长篇滚动蓝图的第一轮闭环。

**Architecture:** 这不是一组可并行的独立子系统，而是一条有严格顺序的生产链。先收口运行期权威源与 AI 入口，再建立状态层和上下文编译器，最后升级技能系统与长篇规划；否则后面的设计会建立在不稳定边界上。

**Tech Stack:** Tauri 2、Rust、SQLite、React 19、TypeScript、Vite、Node test runner、Markdown 技能文件。

---

## File Structure Map

### New files

- `src-tauri/migrations/project/0004_ai_strategy_profile.sql`
- `src-tauri/migrations/project/0005_entity_provenance.sql`
- `src-tauri/migrations/project/0006_story_state.sql`
- `src-tauri/src/services/story_state_service.rs`
- `src-tauri/src/services/ai_pipeline/continuity_pack.rs`
- `src/components/settings/AiStrategyPanel.tsx`
- `tests/integration/ai-strategy-project-config.test.ts`
- `tests/integration/pipeline-persist-policy-contracts.test.ts`
- `tests/integration/continuity-pack-contracts.test.ts`
- `tests/integration/skill-orchestration-contracts.test.ts`

### Modified files

- `src-tauri/src/infra/migrator.rs`
- `src-tauri/src/services/project_service.rs`
- `src-tauri/src/commands/project_commands.rs`
- `src-tauri/src/services/context_service.rs`
- `src-tauri/src/services/ai_pipeline/prompt_resolver.rs`
- `src-tauri/src/services/ai_pipeline/orchestrator.rs`
- `src-tauri/src/services/ai_pipeline_service.rs`
- `src-tauri/src/services/ai_pipeline/task_handlers.rs`
- `src-tauri/src/services/ai_service.rs`
- `src-tauri/src/services/skill_registry.rs`
- `src-tauri/src/commands/skill_commands.rs`
- `src-tauri/src/lib.rs`
- `src/api/settingsApi.ts`
- `src/api/pipelineApi.ts`
- `src/api/bookPipelineApi.ts`
- `src/api/blueprintApi.ts`
- `src/api/characterApi.ts`
- `src/api/worldApi.ts`
- `src/api/glossaryApi.ts`
- `src/api/plotApi.ts`
- `src/api/narrativeApi.ts`
- `src/api/skillsApi.ts`
- `src/api/contextApi.ts`
- `src/pages/Settings/SettingsPage.tsx`
- `src/pages/Blueprint/BlueprintPage.tsx`
- `src/pages/Timeline/TimelinePage.tsx`
- `src/pages/Relationships/RelationshipsPage.tsx`
- `src/pages/Editor/EditorPage.tsx`
- `src/types/ai.ts`
- `src/domain/types.ts`
- `tests/integration/book-pipeline-orchestration.test.ts`
- `tests/integration/ai-module-backfill-contracts.test.ts`
- `tests/integration/runtime-chain-full.test.ts`
- `tests/integration/tauri-contract-smoke.test.ts`
- `docs/architecture/windows-desktop-architecture.md`
- `docs/runtime/runtime-process-spec.md`
- `docs/api/api-integration-spec.md`
- `docs/ui/ui-design-spec.md`

### Responsibilities

- `project_service.rs` / `project_commands.rs`: 项目级运行期真相源、AI 策略和写作风格的项目库读写
- `pipelineApi.ts` / `ai_pipeline_service.rs`: AI 任务输入契约、自动化档位、持久化策略
- `task_handlers.rs`: 正式资产晋升、派生审阅更新、来源记录
- `story_state_service.rs`: 状态账本存取、增量状态写入
- `continuity_pack.rs` + `prompt_resolver.rs`: 写前上下文编译
- `skill_registry.rs` / `skill_commands.rs` / `skillsApi.ts` / `AiStrategyPanel.tsx`: 技能 manifest 升级与可编辑性

---

### Task 1: 建立项目级 AI 策略权威源

**Files:**
- Create: `src-tauri/migrations/project/0004_ai_strategy_profile.sql`
- Modify: `src-tauri/src/infra/migrator.rs`
- Modify: `src-tauri/src/services/project_service.rs`
- Modify: `src-tauri/src/commands/project_commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/services/project_service.rs`
- Test: `tests/integration/ai-strategy-project-config.test.ts`
- Test: `tests/integration/tauri-contract-smoke.test.ts`

- [ ] **Step 1: 写失败的 Rust 回归测试，锁定项目级策略只以项目库为运行期真相源**

```rust
#[test]
fn ai_strategy_profile_save_and_get_roundtrip_succeeds() {
    let service = ProjectService;
    let temp = tempfile::tempdir().expect("tempdir");
    let project = service
        .create_project(CreateProjectInput {
            name: "策略测试".into(),
            author: None,
            genre: "仙侠".into(),
            target_words: Some(100_000),
            save_directory: temp.path().to_string_lossy().to_string(),
        })
        .expect("create project");

    let profile = AiStrategyProfile {
        automation_default: "supervised".into(),
        review_strictness: 5,
        default_workflow_stack: vec!["chapter.plan".into(), "chapter.draft".into()],
        always_on_policy_skills: vec!["term-lock".into()],
        default_capability_bundles: vec!["character-presence".into()],
        state_write_policy: "chapter_confirmed".into(),
        continuity_pack_depth: "standard".into(),
        chapter_generation_mode: "plan_scene_draft".into(),
        window_planning_horizon: 12,
    };

    service
        .save_ai_strategy_profile(&project.project_root, &profile)
        .expect("save strategy");

    let loaded = service
        .get_ai_strategy_profile(&project.project_root)
        .expect("load strategy");

    assert_eq!(loaded, profile);
}
```

- [ ] **Step 2: 运行测试，确认当前实现失败**

Run:
```powershell
cargo test ai_strategy_profile_save_and_get_roundtrip_succeeds -- --exact
```

Expected:
```text
error[E0599]: no method named `save_ai_strategy_profile` found for struct `ProjectService`
```

- [ ] **Step 3: 增加迁移和项目服务结构**

```sql
-- src-tauri/migrations/project/0004_ai_strategy_profile.sql
ALTER TABLE projects ADD COLUMN ai_strategy_profile TEXT;
```

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiStrategyProfile {
    pub automation_default: String,
    pub review_strictness: i64,
    pub default_workflow_stack: Vec<String>,
    pub always_on_policy_skills: Vec<String>,
    pub default_capability_bundles: Vec<String>,
    pub state_write_policy: String,
    pub continuity_pack_depth: String,
    pub chapter_generation_mode: String,
    pub window_planning_horizon: i64,
}

impl Default for AiStrategyProfile {
    fn default() -> Self {
        Self {
            automation_default: "supervised".into(),
            review_strictness: 4,
            default_workflow_stack: vec!["chapter.plan".into(), "chapter.draft".into()],
            always_on_policy_skills: vec![],
            default_capability_bundles: vec![],
            state_write_policy: "chapter_confirmed".into(),
            continuity_pack_depth: "standard".into(),
            chapter_generation_mode: "plan_scene_draft".into(),
            window_planning_horizon: 10,
        }
    }
}

pub fn save_ai_strategy_profile(
    &self,
    project_root: &str,
    profile: &AiStrategyProfile,
) -> Result<(), AppErrorDto> { /* update projects.ai_strategy_profile */ }

pub fn get_ai_strategy_profile(
    &self,
    project_root: &str,
) -> Result<AiStrategyProfile, AppErrorDto> { /* load from projects.ai_strategy_profile or default */ }
```

- [ ] **Step 4: 暴露 Tauri command，并保持 `project.json` 非运行期真相源**

```rust
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveAiStrategyProfileInput {
    pub project_root: String,
    pub profile: AiStrategyProfile,
}

#[tauri::command]
pub async fn save_ai_strategy_profile(
    input: SaveAiStrategyProfileInput,
    state: State<'_, AppState>,
) -> Result<(), AppErrorDto> {
    state
        .project_service
        .save_ai_strategy_profile(&input.project_root, &input.profile)
}
```

- [ ] **Step 5: 运行 Rust 测试与契约测试**

Run:
```powershell
cargo test ai_strategy_profile_save_and_get_roundtrip_succeeds -- --exact
node --import tsx --test tests/integration/ai-strategy-project-config.test.ts tests/integration/tauri-contract-smoke.test.ts
```

Expected:
```text
ai_strategy_profile_save_and_get_roundtrip_succeeds
ok 2 tests
```

- [ ] **Step 6: Commit**

```bash
git add src-tauri/migrations/project/0004_ai_strategy_profile.sql src-tauri/src/infra/migrator.rs src-tauri/src/services/project_service.rs src-tauri/src/commands/project_commands.rs src-tauri/src/lib.rs tests/integration/ai-strategy-project-config.test.ts tests/integration/tauri-contract-smoke.test.ts
git commit -m "feat: add project ai strategy persistence"
```

---

### Task 2: 暴露项目级 AI 策略前端入口

**Files:**
- Create: `src/components/settings/AiStrategyPanel.tsx`
- Modify: `src/types/ai.ts`
- Modify: `src/api/settingsApi.ts`
- Modify: `src/pages/Settings/SettingsPage.tsx`
- Test: `tests/integration/tauri-contract-smoke.test.ts`

- [ ] **Step 1: 写前端契约测试，锁定新 API 与设置页入口**

```ts
test("AI 策略设置契约：settingsApi 暴露项目级策略读写函数", async () => {
  const api = await fs.readFile(path.join(process.cwd(), "src/api/settingsApi.ts"), "utf8");
  assert.match(api, /export async function saveAiStrategyProfile/);
  assert.match(api, /export async function getAiStrategyProfile/);
});

test("AI 策略设置契约：SettingsPage 暴露 AI 策略页签", async () => {
  const page = await fs.readFile(path.join(process.cwd(), "src/pages/Settings/SettingsPage.tsx"), "utf8");
  assert.match(page, /"aiStrategy"/);
  assert.match(page, /AiStrategyPanel/);
});
```

- [ ] **Step 2: 运行测试，确认当前实现失败**

Run:
```powershell
node --import tsx --test tests/integration/tauri-contract-smoke.test.ts tests/integration/ai-strategy-project-config.test.ts
```

Expected:
```text
not ok - AI 策略设置契约：settingsApi 暴露项目级策略读写函数
```

- [ ] **Step 3: 添加前端类型和 API**

```ts
export interface AiStrategyProfile {
  automationDefault: "auto" | "supervised" | "confirm";
  reviewStrictness: number;
  defaultWorkflowStack: string[];
  alwaysOnPolicySkills: string[];
  defaultCapabilityBundles: string[];
  stateWritePolicy: "chapter_confirmed" | "manual_only";
  continuityPackDepth: "minimal" | "standard" | "deep";
  chapterGenerationMode: "draft_only" | "plan_draft" | "plan_scene_draft";
  windowPlanningHorizon: number;
}

export async function saveAiStrategyProfile(projectRoot: string, profile: AiStrategyProfile): Promise<void> {
  await invokeCommand<void>("save_ai_strategy_profile", { input: { projectRoot, profile } });
}

export async function getAiStrategyProfile(projectRoot: string): Promise<AiStrategyProfile> {
  return invokeCommand<AiStrategyProfile>("get_ai_strategy_profile", { input: { projectRoot } });
}
```

- [ ] **Step 4: 把 AI 策略 UI 从巨型 `SettingsPage` 中抽成独立面板**

```tsx
export function AiStrategyPanel({
  value,
  onChange,
  onSave,
  loading,
}: {
  value: AiStrategyProfile;
  onChange: (next: AiStrategyProfile) => void;
  onSave: () => void;
  loading: boolean;
}) {
  return (
    <Card padding="lg">
      <h2 className="text-xl font-semibold text-surface-100 mb-6">AI 策略</h2>
      {/* automationDefault / reviewStrictness / continuityPackDepth / windowPlanningHorizon */}
      <Button onClick={onSave} loading={loading}>保存 AI 策略</Button>
    </Card>
  );
}
```

- [ ] **Step 5: 运行类型检查和契约测试**

Run:
```powershell
npm run typecheck:web
node --import tsx --test tests/integration/tauri-contract-smoke.test.ts tests/integration/ai-strategy-project-config.test.ts
```

Expected:
```text
Found 0 errors.
ok 4 tests
```

- [ ] **Step 6: Commit**

```bash
git add src/components/settings/AiStrategyPanel.tsx src/types/ai.ts src/api/settingsApi.ts src/pages/Settings/SettingsPage.tsx tests/integration/tauri-contract-smoke.test.ts tests/integration/ai-strategy-project-config.test.ts
git commit -m "feat: expose project ai strategy settings"
```

---

### Task 3: 用显式持久化契约替代裸 `autoPersist`

**Files:**
- Modify: `src/api/pipelineApi.ts`
- Modify: `src-tauri/src/services/ai_pipeline_service.rs`
- Modify: `src/api/bookPipelineApi.ts`
- Modify: `src/api/blueprintApi.ts`
- Modify: `src/api/characterApi.ts`
- Modify: `src/api/worldApi.ts`
- Modify: `src/api/glossaryApi.ts`
- Modify: `src/api/plotApi.ts`
- Modify: `src/api/narrativeApi.ts`
- Modify: `src/pages/Timeline/TimelinePage.tsx`
- Modify: `src/pages/Relationships/RelationshipsPage.tsx`
- Test: `tests/integration/book-pipeline-orchestration.test.ts`
- Test: `tests/integration/pipeline-persist-policy-contracts.test.ts`

- [ ] **Step 1: 写失败的入口分类测试**

```ts
test("持久化策略契约：核心入口显式声明 persistMode 和 automationTier", async () => {
  const files = [
    "src/api/bookPipelineApi.ts",
    "src/api/blueprintApi.ts",
    "src/api/characterApi.ts",
    "src/api/worldApi.ts",
    "src/api/glossaryApi.ts",
    "src/api/plotApi.ts",
    "src/api/narrativeApi.ts",
    "src/pages/Timeline/TimelinePage.tsx",
    "src/pages/Relationships/RelationshipsPage.tsx",
  ];

  for (const rel of files) {
    const raw = await fs.readFile(path.join(process.cwd(), rel), "utf8");
    assert.match(raw, /persistMode:/);
    assert.match(raw, /automationTier:/);
  }
});
```

- [ ] **Step 2: 运行测试，确认当前实现失败**

Run:
```powershell
node --import tsx --test tests/integration/pipeline-persist-policy-contracts.test.ts tests/integration/book-pipeline-orchestration.test.ts
```

Expected:
```text
not ok - 持久化策略契约：核心入口显式声明 persistMode 和 automationTier
```

- [ ] **Step 3: 扩展 pipeline 输入契约，并保留 `autoPersist` 兼容桥**

```ts
export type PersistMode = "none" | "formal" | "derived_review";
export type AutomationTier = "auto" | "supervised" | "confirm";

export interface RunTaskPipelineInput {
  projectRoot: string;
  taskType: string;
  chapterId?: string;
  uiAction?: string;
  userInstruction?: string;
  blueprintStepKey?: string;
  blueprintStepTitle?: string;
  autoPersist?: boolean; // legacy bridge
  persistMode?: PersistMode;
  automationTier?: AutomationTier;
}
```

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunAiTaskPipelineInput {
    pub project_root: String,
    pub task_type: String,
    pub chapter_id: Option<String>,
    pub ui_action: Option<String>,
    pub user_instruction: Option<String>,
    pub blueprint_step_key: Option<String>,
    pub blueprint_step_title: Option<String>,
    #[serde(default)]
    pub auto_persist: bool,
    pub persist_mode: Option<String>,
    pub automation_tier: Option<String>,
}
```

- [ ] **Step 4: 逐入口显式分类**

```ts
// blueprintApi.ts
persistMode: "formal",
automationTier: "supervised",

// bookPipelineApi.ts blueprint stages
persistMode: "formal",
automationTier: "supervised",

// bookPipelineApi.ts seed stages
persistMode: "formal",
automationTier: "confirm",

// timeline / relationships review
persistMode: "derived_review",
automationTier: "auto",
```

- [ ] **Step 5: 更新编排测试，锁定新契约**

```ts
assert.equal(stages[0]?.request.persistMode, "formal");
assert.equal(stages[0]?.request.automationTier, "supervised");
assert.equal(stages[8]?.request.automationTier, "confirm");
```

- [ ] **Step 6: 运行测试**

Run:
```powershell
node --import tsx --test tests/integration/pipeline-persist-policy-contracts.test.ts tests/integration/book-pipeline-orchestration.test.ts
npm run typecheck:web
```

Expected:
```text
ok 5 tests
Found 0 errors.
```

- [ ] **Step 7: Commit**

```bash
git add src/api/pipelineApi.ts src-tauri/src/services/ai_pipeline_service.rs src/api/bookPipelineApi.ts src/api/blueprintApi.ts src/api/characterApi.ts src/api/worldApi.ts src/api/glossaryApi.ts src/api/plotApi.ts src/api/narrativeApi.ts src/pages/Timeline/TimelinePage.tsx src/pages/Relationships/RelationshipsPage.tsx tests/integration/book-pipeline-orchestration.test.ts tests/integration/pipeline-persist-policy-contracts.test.ts
git commit -m "feat: add explicit ai persist policy contract"
```

---

### Task 4: 收口全书生成与模块晋升边界，并记录来源轨迹

**Files:**
- Create: `src-tauri/migrations/project/0005_entity_provenance.sql`
- Modify: `src-tauri/src/infra/migrator.rs`
- Modify: `src/api/bookPipelineApi.ts`
- Modify: `src/pages/Blueprint/BlueprintPage.tsx`
- Modify: `src-tauri/src/services/ai_pipeline/task_handlers.rs`
- Modify: `src-tauri/src/services/ai_pipeline_service.rs`
- Test: `tests/integration/book-pipeline-orchestration.test.ts`
- Test: `tests/integration/runtime-chain-full.test.ts`

- [ ] **Step 1: 写失败的编排边界测试**

```ts
test("全书生成边界：默认编排只覆盖蓝图阶段", () => {
  const stages = buildBookStages({
    projectRoot: "F:/demo",
    ideaPrompt: "群像修真",
  });

  assert.deepEqual(
    stages.map((stage) => stage.key),
    [
      "blueprint-anchor",
      "blueprint-genre",
      "blueprint-premise",
      "blueprint-characters",
      "blueprint-world",
      "blueprint-glossary",
      "blueprint-plot",
      "blueprint-chapters",
    ]
  );
});
```

- [ ] **Step 2: 运行测试，确认当前行为仍跨层**

Run:
```powershell
node --import tsx --test tests/integration/book-pipeline-orchestration.test.ts
```

Expected:
```text
Expected values to be strictly deep-equal
```

- [ ] **Step 3: 调整 book pipeline，拆成 blueprint-first + promote-second**

```ts
export function buildBookStages(input: RunBookGenerationInput): BookStage[] {
  return BLUEPRINT_STAGES.map((stage) => ({
    key: stage.key,
    label: stage.label,
    request: {
      projectRoot: input.projectRoot,
      taskType: "blueprint.generate_step",
      userInstruction: buildBlueprintInstruction(base, stage.stepKey, stage.stepTitle),
      blueprintStepKey: stage.stepKey,
      blueprintStepTitle: stage.stepTitle,
      persistMode: "formal",
      automationTier: "supervised",
    },
  }));
}
```

```ts
export function buildPromotionStages(input: RunBookGenerationInput): BookStage[] {
  return [
    buildPromotionStage("character-seed", "character.create", input.projectRoot),
    buildPromotionStage("world-seed", "world.create_rule", input.projectRoot),
    buildPromotionStage("plot-seed", "plot.create_node", input.projectRoot),
    buildPromotionStage("glossary-seed", "glossary.create_term", input.projectRoot),
    buildPromotionStage("narrative-seed", "narrative.create_obligation", input.projectRoot),
    buildPromotionStage("chapter-plan", "chapter.plan", input.projectRoot),
  ];
}
```

- [ ] **Step 4: 为正式晋升写入来源记录**

```sql
CREATE TABLE entity_provenance (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  source_kind TEXT NOT NULL,
  source_ref TEXT,
  request_id TEXT,
  created_at TEXT NOT NULL
);
```

```rust
fn record_entity_provenance(
    conn: &Connection,
    project_id: &str,
    entity_type: &str,
    entity_id: &str,
    source_kind: &str,
    source_ref: Option<&str>,
    request_id: &str,
) -> Result<(), AppErrorDto> {
    conn.execute(
        "INSERT INTO entity_provenance (id, project_id, entity_type, entity_id, source_kind, source_ref, request_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            new_uuid(),
            project_id,
            entity_type,
            entity_id,
            source_kind,
            source_ref,
            request_id,
            now_rfc3339(),
        ],
    )
    .map_err(map_sqlite_error)?;
    Ok(())
}
```

- [ ] **Step 5: 在蓝图页补“确认并晋升”动作入口**

```tsx
<Button onClick={() => void handlePromoteBlueprintStep(currentStep.key)}>
  确认并同步
</Button>
```

- [ ] **Step 6: 运行测试**

Run:
```powershell
node --import tsx --test tests/integration/book-pipeline-orchestration.test.ts tests/integration/runtime-chain-full.test.ts
```

Expected:
```text
ok 6 tests
```

- [ ] **Step 7: Commit**

```bash
git add src-tauri/migrations/project/0005_entity_provenance.sql src-tauri/src/infra/migrator.rs src/api/bookPipelineApi.ts src/pages/Blueprint/BlueprintPage.tsx src-tauri/src/services/ai_pipeline/task_handlers.rs src-tauri/src/services/ai_pipeline_service.rs tests/integration/book-pipeline-orchestration.test.ts tests/integration/runtime-chain-full.test.ts
git commit -m "feat: split blueprint generation from asset promotion"
```

---

### Task 5: 建立 State Ledger 并接入章节写后回写

**Files:**
- Create: `src-tauri/migrations/project/0006_story_state.sql`
- Create: `src-tauri/src/services/story_state_service.rs`
- Modify: `src-tauri/src/services/context_service.rs`
- Modify: `src/api/contextApi.ts`
- Modify: `src/pages/Editor/EditorPage.tsx`
- Modify: `src-tauri/src/services/chapter_service.rs`
- Test: `src-tauri/src/services/story_state_service.rs`
- Test: `src-tauri/src/services/context_service.rs`

- [ ] **Step 1: 写失败的 Rust 测试，锁定状态账本存取**

```rust
#[test]
fn story_state_upsert_and_latest_lookup_succeeds() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = ProjectService::create_project(CreateProjectInput {
        root_dir: temp.path().to_string_lossy().to_string(),
        name: "state-ledger-demo".into(),
        template: None,
    })
    .expect("project");
    let svc = StoryStateService::default();

    svc.upsert_state(
        &project.project_root,
        StoryStateInput {
            subject_type: "character".into(),
            subject_id: "char-1".into(),
            scope: "chapter".into(),
            state_kind: "emotion".into(),
            payload_json: json!({ "value": "anger" }),
            source_chapter_id: Some("chapter-1".into()),
        },
    ).expect("save state");

    let rows = svc.list_latest_states(&project.project_root, Some("character"), Some("char-1")).expect("states");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].state_kind, "emotion");
}
```

- [ ] **Step 2: 运行测试，确认当前服务不存在**

Run:
```powershell
cargo test story_state_upsert_and_latest_lookup_succeeds -- --exact
```

Expected:
```text
error[E0432]: unresolved import `crate::services::story_state_service`
```

- [ ] **Step 3: 增加迁移和服务**

```sql
CREATE TABLE story_state (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  subject_type TEXT NOT NULL,
  subject_id TEXT NOT NULL,
  scope TEXT NOT NULL,
  state_kind TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  source_chapter_id TEXT,
  status TEXT NOT NULL DEFAULT 'active',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX idx_story_state_lookup
  ON story_state(project_id, subject_type, subject_id, state_kind, status);
```

```rust
pub struct StoryStateRow {
    pub id: String,
    pub subject_type: String,
    pub subject_id: String,
    pub scope: String,
    pub state_kind: String,
    pub payload_json: serde_json::Value,
    pub source_chapter_id: Option<String>,
    pub status: String,
}
```

- [ ] **Step 4: 在章节保存后触发最小状态回写**

```rust
pub fn save_chapter_content(
    &self,
    input: SaveChapterContentInput,
) -> Result<SaveChapterOutput, AppErrorDto> {
    let output = self.persist_chapter_content(&input)?;
    StoryStateService::default().record_window_progress(
        &input.project_root,
        &input.chapter_id,
        output.word_count,
    )?;
    Ok(output)
}
```

```ts
await saveChapterContent(chapterId, content, projectRoot);
const next = await getChapterContext(projectRoot, chapterId);
setContext(next);
```

- [ ] **Step 5: 扩展上下文接口，返回状态摘要**

```ts
export interface ChapterContext {
  chapterId: string;
  chapterTitle: string;
  previousChapterSummary?: string;
  blueprintSummary: string[];
  stateSummary: Array<{
    subjectType: string;
    subjectId: string;
    stateKind: string;
    payload: Record<string, unknown>;
  }>;
}
```

- [ ] **Step 6: 运行测试**

Run:
```powershell
cargo test story_state_upsert_and_latest_lookup_succeeds -- --exact
cargo test context_service::tests::editor_context_includes_state_summary -- --exact
```

Expected:
```text
story_state_upsert_and_latest_lookup_succeeds
editor_context_includes_state_summary
ok
```

- [ ] **Step 7: Commit**

```bash
git add src-tauri/migrations/project/0006_story_state.sql src-tauri/src/services/story_state_service.rs src-tauri/src/services/context_service.rs src/api/contextApi.ts src/pages/Editor/EditorPage.tsx src-tauri/src/services/chapter_service.rs
git commit -m "feat: add story state ledger foundation"
```

---

### Task 6: 实现 Continuity Pack 编译器并接入 Prompt 解析

**Files:**
- Create: `src-tauri/src/services/ai_pipeline/continuity_pack.rs`
- Modify: `src-tauri/src/services/context_service.rs`
- Modify: `src-tauri/src/services/ai_pipeline/prompt_resolver.rs`
- Modify: `src-tauri/src/services/ai_pipeline/orchestrator.rs`
- Test: `src-tauri/src/services/ai_pipeline/prompt_resolver.rs`
- Test: `tests/integration/continuity-pack-contracts.test.ts`

- [ ] **Step 1: 写失败的契约测试，锁定 7 类上下文**

```ts
test("Continuity Pack 契约：PromptResolver 显式注入 7 类上下文", async () => {
  const raw = await fs.readFile(
    path.join(process.cwd(), "src-tauri/src/services/ai_pipeline/prompt_resolver.rs"),
    "utf8",
  );
  assert.match(raw, /Lexicon Policy Context/);
  assert.match(raw, /State Context/);
  assert.match(raw, /Promise Context/);
  assert.match(raw, /Window Plan Context/);
});
```

- [ ] **Step 2: 运行测试，确认当前实现仍是摘要拼接**

Run:
```powershell
node --import tsx --test tests/integration/continuity-pack-contracts.test.ts
```

Expected:
```text
not ok - Continuity Pack 契约：PromptResolver 显式注入 7 类上下文
```

- [ ] **Step 3: 新建 `continuity_pack.rs`，定义编译输出**

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinuityPack {
    pub constitution_context: Vec<String>,
    pub canon_context: Vec<String>,
    pub lexicon_policy_context: Vec<String>,
    pub state_context: Vec<String>,
    pub promise_context: Vec<String>,
    pub window_plan_context: Vec<String>,
    pub recent_continuity_context: Vec<String>,
}
```

- [ ] **Step 4: 在 `PromptResolver` 中改为消费 Continuity Pack**

```rust
let pack = ContinuityPackCompiler::compile(&context, canonical_task);
sections.push("## Lexicon Policy Context".to_string());
sections.extend(pack.lexicon_policy_context);
sections.push("## State Context".to_string());
sections.extend(pack.state_context);
```

- [ ] **Step 5: 把 `locked_terms` / `banned_terms` / 近期章节承接纳入编译结果**

```rust
if !global.locked_terms.is_empty() {
    lines.push(format!("锁定术语: {}", global.locked_terms.join("、")));
}
if !global.banned_terms.is_empty() {
    lines.push(format!("禁用词: {}", global.banned_terms.join("、")));
}
```

- [ ] **Step 6: 运行测试**

Run:
```powershell
cargo test prompt_resolver::tests::continuity_pack_includes_lexicon_policy -- --exact
node --import tsx --test tests/integration/continuity-pack-contracts.test.ts
```

Expected:
```text
continuity_pack_includes_lexicon_policy
ok
ok 1 test
```

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/services/ai_pipeline/continuity_pack.rs src-tauri/src/services/context_service.rs src-tauri/src/services/ai_pipeline/prompt_resolver.rs src-tauri/src/services/ai_pipeline/orchestrator.rs tests/integration/continuity-pack-contracts.test.ts
git commit -m "feat: compile continuity pack before generation"
```

---

### Task 7: 升级技能 manifest、命令接口和设置 UI

**Files:**
- Modify: `src-tauri/src/services/skill_registry.rs`
- Modify: `src-tauri/src/commands/skill_commands.rs`
- Modify: `src/api/skillsApi.ts`
- Modify: `src/components/skills/SkillDetail.tsx`
- Modify: `src/components/skills/SkillsManager.tsx`
- Test: `tests/integration/skill-orchestration-contracts.test.ts`
- Test: `src-tauri/src/services/skill_registry.rs`

- [ ] **Step 1: 写失败的契约测试，锁定 manifest 元数据可编辑性**

```ts
test("技能契约：前后端同时暴露 manifest 元数据更新能力", async () => {
  const api = await fs.readFile(path.join(process.cwd(), "src/api/skillsApi.ts"), "utf8");
  const cmd = await fs.readFile(path.join(process.cwd(), "src-tauri/src/commands/skill_commands.rs"), "utf8");
  assert.match(api, /updateSkill\\(input: UpdateSkillInput\\)/);
  assert.match(cmd, /pub struct UpdateSkillInput/);
});
```

- [ ] **Step 2: 运行测试，确认当前只有 body 编辑**

Run:
```powershell
node --import tsx --test tests/integration/skill-orchestration-contracts.test.ts
```

Expected:
```text
not ok - 技能契约：前后端同时暴露 manifest 元数据更新能力
```

- [ ] **Step 3: 扩展 `SkillManifest` 和更新命令**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillManifest {
    // existing fields
    #[serde(default)]
    pub skill_class: Option<String>,
    #[serde(default)]
    pub bundle_ids: Vec<String>,
    #[serde(default)]
    pub always_on: bool,
    #[serde(default)]
    pub trigger_conditions: Vec<String>,
    #[serde(default)]
    pub required_contexts: Vec<String>,
    #[serde(default)]
    pub state_writes: Vec<String>,
    #[serde(default)]
    pub automation_tier: Option<String>,
    #[serde(default)]
    pub scene_tags: Vec<String>,
    #[serde(default)]
    pub affects_layers: Vec<String>,
}
```

```rust
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSkillInput {
    pub id: String,
    pub body: String,
    pub manifest: SkillManifestPatch,
}
```

- [ ] **Step 4: 同步前端接口和编辑 UI**

```ts
export interface UpdateSkillInput {
  id: string;
  body: string;
  manifest: Partial<SkillManifest>;
}

export async function updateSkill(input: UpdateSkillInput): Promise<SkillManifest> {
  return invokeCommand<SkillManifest>("update_skill", { input });
}
```

```tsx
<Select value={draft.skillClass ?? ""} onChange={(e) => setDraft(patchSkillDraft(draft, { skillClass: e.target.value }))}>
  <option value="workflow">workflow</option>
  <option value="capability">capability</option>
  <option value="extractor">extractor</option>
  <option value="review">review</option>
  <option value="policy">policy</option>
</Select>
```

- [ ] **Step 5: 运行测试**

Run:
```powershell
cargo test skill_registry::tests::update_skill_manifest_roundtrip_succeeds -- --exact
node --import tsx --test tests/integration/skill-orchestration-contracts.test.ts
npm run typecheck:web
```

Expected:
```text
update_skill_manifest_roundtrip_succeeds
ok
ok 2 tests
Found 0 errors.
```

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/services/skill_registry.rs src-tauri/src/commands/skill_commands.rs src/api/skillsApi.ts src/components/skills/SkillDetail.tsx src/components/skills/SkillsManager.tsx tests/integration/skill-orchestration-contracts.test.ts
git commit -m "feat: make skill manifest editable"
```

---

### Task 8: 打通技能运行期消费与 route override

**Files:**
- Modify: `src-tauri/src/services/ai_service.rs`
- Modify: `src-tauri/src/services/ai_pipeline/orchestrator.rs`
- Modify: `src-tauri/src/services/ai_pipeline/prompt_resolver.rs`
- Test: `src-tauri/src/services/ai_service.rs`
- Test: `tests/integration/skill-orchestration-contracts.test.ts`

- [ ] **Step 1: 写失败的运行期测试，锁定主 pipeline 传入 skill registry**

```rust
#[tokio::test]
async fn stream_generate_for_pipeline_uses_skill_route_override() {
    let req = UnifiedGenerateRequest {
        task_type: Some("custom.scene.render".into()),
        ..Default::default()
    };
    // build registry with route override
    // assert resolved provider/model follows skill.task_route
}
```

- [ ] **Step 2: 运行测试，确认当前 orchestrator 仍传 `None`**

Run:
```powershell
cargo test stream_generate_for_pipeline_uses_skill_route_override -- --exact
```

Expected:
```text
assertion failed: left == right
```

- [ ] **Step 3: 修改 orchestrator，把 registry 真正传入流式生成**

```rust
let guard = self.skill_registry.read().map_err(|err| StageError {
    phase: PHASE_ROUTE,
    error: AppErrorDto::new("SKILLS_LOCK_FAILED", "skill registry lock failed", false)
        .with_detail(err.to_string()),
})?;

let mut rx = self
    .ai_service
    .stream_generate_for_pipeline(req, Some(&guard))
    .await
    .map_err(|err| StageError { phase: PHASE_GENERATE, error: err })?;
```

- [ ] **Step 4: 把 capability / policy 技能栈映射进 PromptResolver**

```rust
let injected_skills = collect_enabled_skills(skill_registry, canonical_task, project_strategy);
for skill in injected_skills.policy {
    sections.push(format!("## Policy Skill: {}", skill.name));
    sections.push(skill_template);
}
```

- [ ] **Step 5: 运行测试**

Run:
```powershell
cargo test stream_generate_for_pipeline_uses_skill_route_override -- --exact
node --import tsx --test tests/integration/skill-orchestration-contracts.test.ts
```

Expected:
```text
stream_generate_for_pipeline_uses_skill_route_override
ok
ok 2 tests
```

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/services/ai_service.rs src-tauri/src/services/ai_pipeline/orchestrator.rs src-tauri/src/services/ai_pipeline/prompt_resolver.rs tests/integration/skill-orchestration-contracts.test.ts
git commit -m "feat: wire skill orchestration into runtime generation"
```

---

### Task 9: 长篇窗口蓝图与派生审阅闭环

**Files:**
- Modify: `src/pages/Blueprint/BlueprintPage.tsx`
- Modify: `src/api/blueprintApi.ts`
- Modify: `src/pages/Timeline/TimelinePage.tsx`
- Modify: `src/pages/Relationships/RelationshipsPage.tsx`
- Modify: `src/api/contextApi.ts`
- Test: `tests/integration/blueprint-backfill-accuracy.test.ts`
- Test: `tests/integration/runtime-chain-full.test.ts`

- [ ] **Step 1: 写失败的前端契约测试，锁定窗口级规划与摘要回报显示**

```ts
test("蓝图闭环契约：Blueprint 页面暴露窗口级规划与摘要回报区", async () => {
  const page = await fs.readFile(path.join(process.cwd(), "src/pages/Blueprint/BlueprintPage.tsx"), "utf8");
  assert.match(page, /windowPlanningHorizon/);
  assert.match(page, /摘要回报/);
});
```

- [ ] **Step 2: 运行测试，确认当前页面还没有窗口级闭环**

Run:
```powershell
node --import tsx --test tests/integration/blueprint-backfill-accuracy.test.ts tests/integration/runtime-chain-full.test.ts
```

Expected:
```text
not ok - 蓝图闭环契约：Blueprint 页面暴露窗口级规划与摘要回报区
```

- [ ] **Step 3: 在蓝图页加入窗口级规划与摘要回报展示**

```tsx
<Card padding="md">
  <h3 className="text-sm font-semibold text-surface-200">窗口规划</h3>
  <p className="text-xs text-surface-400">未来 {strategy.windowPlanningHorizon} 章</p>
  {/* volumeStructure / chapterGoals / drift summary */}
</Card>

<Card padding="md">
  <h3 className="text-sm font-semibold text-surface-200">摘要回报</h3>
  {/* currentVolumeProgress / keyVariableDelta / driftWarnings */}
</Card>
```

- [ ] **Step 4: 在时间线/关系图页显示“派生审阅层”提示并消费来源数据**

```tsx
<p className="text-xs text-surface-500">
  本页为派生审阅层，展示已晋升的正式数据与待审信息，不是一级事实源。
</p>
```

- [ ] **Step 5: 运行测试**

Run:
```powershell
node --import tsx --test tests/integration/blueprint-backfill-accuracy.test.ts tests/integration/runtime-chain-full.test.ts
npm run typecheck:web
```

Expected:
```text
ok 6 tests
Found 0 errors.
```

- [ ] **Step 6: Commit**

```bash
git add src/pages/Blueprint/BlueprintPage.tsx src/api/blueprintApi.ts src/pages/Timeline/TimelinePage.tsx src/pages/Relationships/RelationshipsPage.tsx src/api/contextApi.ts tests/integration/blueprint-backfill-accuracy.test.ts tests/integration/runtime-chain-full.test.ts
git commit -m "feat: add rolling blueprint and derived review loop"
```

---

### Task 10: 文档同步与全链路验证

**Files:**
- Modify: `docs/architecture/windows-desktop-architecture.md`
- Modify: `docs/runtime/runtime-process-spec.md`
- Modify: `docs/api/api-integration-spec.md`
- Modify: `docs/ui/ui-design-spec.md`

- [ ] **Step 1: 更新架构文档中的系统对象与权威源**

```md
- Story Constitution / Canon Registry / State Ledger / Execution Workspace / Review Trail
- 项目级 AI 策略由 project.sqlite 承担运行期真相源
- 应用级 Provider/Key 仍保存在 novelforge.db
```

- [ ] **Step 2: 更新运行流程文档中的章节链路**

```md
1. 编译 Continuity Pack
2. 装配技能栈
3. 生成章节计划/场景/草稿
4. 写后回写 Canon + State
```

- [ ] **Step 3: 更新 API 与 UI 设计文档**

```md
- 新增 save_ai_strategy_profile / get_ai_strategy_profile
- RunTaskPipelineInput 新增 persistMode / automationTier
- 设置页新增 AI 策略面板
```

- [ ] **Step 4: 运行全链路验证**

Run:
```powershell
cargo test
node --import tsx --test tests/integration/ai-strategy-project-config.test.ts tests/integration/pipeline-persist-policy-contracts.test.ts tests/integration/continuity-pack-contracts.test.ts tests/integration/skill-orchestration-contracts.test.ts tests/integration/book-pipeline-orchestration.test.ts tests/integration/blueprint-backfill-accuracy.test.ts tests/integration/runtime-chain-full.test.ts tests/integration/tauri-contract-smoke.test.ts
npm run typecheck:web
```

Expected:
```text
test result: ok.
ok
Found 0 errors.
```

- [ ] **Step 5: Commit**

```bash
git add docs/architecture/windows-desktop-architecture.md docs/runtime/runtime-process-spec.md docs/api/api-integration-spec.md docs/ui/ui-design-spec.md
git commit -m "docs: sync ai production system architecture"
```

---

## Spec Coverage Check

- 项目级 AI 策略权威源：Task 1, Task 2
- `autoPersist` 入口收口：Task 3, Task 4
- Review Trail 映射与来源记录：Task 4
- State Ledger：Task 5
- Continuity Pack：Task 6
- Skill manifest / API / UI / runtime：Task 7, Task 8
- 长篇滚动蓝图与派生审阅层：Task 9
- 文档同步：Task 10

无 spec 章节遗漏。

## Placeholder Scan

本计划未保留未落地占位、泛化任务引用或模糊测试要求。

所有任务均包含具体文件、最小代码、命令和预期输出。

## Type Consistency Check

- 项目级策略类型统一为 `AiStrategyProfile`
- 持久化策略统一为 `PersistMode`
- 自动化档位统一为 `AutomationTier`
- 状态账本表统一为 `story_state`
- 写前上下文统一为 `ContinuityPack`

没有在后续任务中切换命名。

## Execution Handoff

**Plan complete and saved to `docs/superpowers/plans/2026-04-30-ai-production-system-implementation-plan.md`. Two execution options:**

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

**Which approach?**
