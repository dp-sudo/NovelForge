# 写作风格方向功能 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a configurable writing style system across the full stack — data model, backend context injection, frontend UI — so AI generation respects the author's chosen style parameters.

**Architecture:** Per-project writing style stored in the `projects` SQLite table, loaded by `ContextService.collect_global_context()` and injected by `PromptBuilder` into every skill prompt's "fixed context" section. A new Settings tab provides the UI with 7-point scale controls and enum selectors.

**Tech Stack:** Rust (Tauri command + service + context_service + prompt_builder), TypeScript (React settings UI + API), SQLite (projects table column)

---

## File Manifest

| File | Responsibility | Action |
|------|----------------|--------|
| `src-tauri/src/services/context_service.rs` | `GlobalContext` struct + `collect_global_context` reads writing_style column | Modify |
| `src-tauri/src/services/project_service.rs` | `ProjectSettings` struct update | Modify |
| `src-tauri/src/commands/project_commands.rs` | New Tauri commands `save_writing_style` / `get_writing_style` | Modify |
| `src-tauri/src/infra/database.rs` | `ensure_compatible_schema` adds `writing_style` column | Modify |
| `src-tauri/src/services/prompt_builder.rs` | All `build_*` methods emit formatted style block | Modify |
| `src/domain/types.ts` | `WritingStyle` interface | Modify |
| `src/api/settingsApi.ts` | `saveWritingStyle` / `getWritingStyle` functions | Modify |
| `src/pages/Settings/SettingsPage.tsx` | New "writing" tab + UI | Modify |

---

### Task 1: Add writing_style column to projects table

**Files:**
- Modify: `src-tauri/src/infra/database.rs:44`

- [ ] **Step 1: Add column migration**

```rust
// In ensure_compatible_schema(), add after the last ensure_column call:
ensure_column(conn, "projects", "writing_style", "TEXT")?;
```

- [ ] **Step 2: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: No errors

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/infra/database.rs
git commit -m "feat: add writing_style column to projects table"
```

---

### Task 2: Add WritingStyle structs to Rust side

**Files:**
- Modify: `src-tauri/src/services/project_service.rs:45`

- [ ] **Step 1: Add WritingStyle struct**

```rust
// In project_service.rs, add before or after ProjectSettings:

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WritingStyle {
    pub language_style: String,       // "plain" | "balanced" | "ornate" | "colloquial"
    pub description_density: i64,     // 1-7
    pub dialogue_ratio: i64,          // 1-7
    pub sentence_rhythm: String,      // "short" | "long" | "mixed"
    pub atmosphere: String,           // "warm" | "cold" | "humorous" | "serious" | "suspenseful" | "neutral"
    pub psychological_depth: i64,     // 1-7
}

impl Default for WritingStyle {
    fn default() -> Self {
        Self {
            language_style: "balanced".to_string(),
            description_density: 4,
            dialogue_ratio: 4,
            sentence_rhythm: "mixed".to_string(),
            atmosphere: "neutral".to_string(),
            psychological_depth: 4,
        }
    }
}
```

- [ ] **Step 2: Add writing_style to GlobalContext in context_service.rs**

```rust
// In GlobalContext struct, add field:
pub writing_style: Option<WritingStyle>,

// In collect_global_context(), add after reading narrative_pov:
let writing_style: Option<WritingStyle> = conn
    .query_row(
        "SELECT writing_style FROM projects WHERE id = ?1",
        params![project_id],
        |row| row.get::<_, Option<String>>(0),
    )
    .ok()
    .flatten()
    .and_then(|json| serde_json::from_str(&json).ok())
    .unwrap_or(None);

// Add to the returned GlobalContext:
writing_style,
```

Import must be added:
```rust
use crate::services::project_service::WritingStyle;
```

- [ ] **Step 3: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: No errors

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/services/project_service.rs src-tauri/src/services/context_service.rs
git commit -m "feat: add WritingStyle struct and load from projects table"
```

---

### Task 3: Add Tauri commands for save/get writing style

**Files:**
- Modify: `src-tauri/src/commands/project_commands.rs`
- Modify: `src-tauri/src/state.rs` (if project_service needs exposing — check existing pattern)

- [ ] **Step 1: Check existing AppState access pattern**

```rust
// project_commands.rs already uses State<'_, AppState>, and
// AppState has pub project_service: ProjectService
// Existing open_project handler uses `state.project_service.create_project(...)`
```

- [ ] **Step 2: Add save_writing_style Tauri command**

```rust
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveWritingStyleInput {
    pub project_root: String,
    pub writing_style: crate::services::project_service::WritingStyle,
}

#[tauri::command]
pub async fn save_writing_style(
    input: SaveWritingStyleInput,
    state: State<'_, AppState>,
) -> Result<(), AppErrorDto> {
    state.project_service.save_writing_style(
        &input.project_root,
        &input.writing_style,
    )
}
```

- [ ] **Step 3: Add save_writing_style method to ProjectService**

Add to `project_service.rs`:

```rust
pub fn save_writing_style(
    &self,
    project_root: &str,
    style: &WritingStyle,
) -> Result<(), AppErrorDto> {
    let conn = open_database(std::path::Path::new(project_root)).map_err(|err| {
        AppErrorDto::new("DB_OPEN_FAILED", "无法打开项目数据库", false)
            .with_detail(err.to_string())
    })?;
    let project_id = get_project_id(&conn)?;
    let json = serde_json::to_string(style).map_err(|err| {
        AppErrorDto::new("SERIALIZE_FAILED", "无法序列化写作风格配置", false)
            .with_detail(err.to_string())
    })?;
    let now = crate::infra::time::now_iso();
    conn.execute(
        "UPDATE projects SET writing_style = ?1, updated_at = ?2 WHERE id = ?3",
        params![json, now, project_id],
    ).map_err(|err| {
        AppErrorDto::new("DB_WRITE_FAILED", "保存写作风格失败", true)
            .with_detail(err.to_string())
    })?;
    Ok(())
}
```

Add import if not present:
```rust
use crate::infra::database::open_database;
```

- [ ] **Step 4: Register command in lib.rs**

Add to the invoke_handler list in `src-tauri/src/lib.rs`:
```rust
commands::project_commands::save_writing_style,
```

- [ ] **Step 5: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: No errors

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/commands/project_commands.rs src-tauri/src/services/project_service.rs src-tauri/src/lib.rs
git commit -m "feat: add save_writing_style Tauri command"
```

---

### Task 4: Inject writing style into prompt_builder outputs

**Files:**
- Modify: `src-tauri/src/services/prompt_builder.rs`

- [ ] **Step 1: Add a helper method to format writing style as text**

```rust
/// Format writing style into a human-readable block for prompt injection.
fn format_writing_style(style: &crate::services::project_service::WritingStyle) -> String {
    let lang_label = match style.language_style.as_str() {
        "plain" => "平实",
        "balanced" => "适中",
        "ornate" => "华丽",
        "colloquial" => "口语化",
        _ => "适中",
    };

    let rhythm_label = match style.sentence_rhythm.as_str() {
        "short" => "短句为主",
        "long" => "长句为主",
        "mixed" => "混合",
        _ => "混合",
    };

    let atmos_label = match style.atmosphere.as_str() {
        "warm" => "温暖",
        "cold" => "冷峻",
        "humorous" => "幽默",
        "serious" => "严肃",
        "suspenseful" => "悬疑",
        "neutral" => "中性",
        _ => "中性",
    };

    format!(
        "写作风格：
- 语言风格：{}
- 描写密度：{}（1=点到为止，7=详细刻画）
- 对话比例：{}（1=偏叙述，7=偏对话）
- 句子节奏：{}
- 氛围基调：{}
- 心理描写深度：{}（1=仅外部行为，7=深入内心）",
        lang_label,
        style.description_density,
        style.dialogue_ratio,
        rhythm_label,
        atmos_label,
        style.psychological_depth,
    )
}
```

- [ ] **Step 2: Inject style block into build_chapter_draft**

In `build_chapter_draft`, after the "固定上下文" section ends (after line 48, before "# 当前章节信息"), add:

```rust
// Writing style
if let Some(ref ws) = global.writing_style {
    parts.push(Self::format_writing_style(ws));
    parts.push(String::new());
}
```

- [ ] **Step 3: Inject into build_rewrite**

In `build_rewrite`, after the "项目上下文" section (after `parts.push(format!("题材：{}", global.genre));` line 172), add:

```rust
if let Some(ref ws) = global.writing_style {
    parts.push(Self::format_writing_style(ws));
}
```

- [ ] **Step 4: Inject into build_chapter_plan**

In `build_chapter_plan`, after the "项目上下文" section (after `parts.push(format!("题材：{}", global.genre));` around line 440), add:

```rust
if let Some(ref ws) = global.writing_style {
    parts.push(Self::format_writing_style(ws));
}
```

- [ ] **Step 5: Inject into build_consistency_scan**

In `build_consistency_scan`, after the project context area (after genre/banned terms), add:

```rust
if let Some(ref ws) = global.writing_style {
    parts.push("## 写作风格约束".to_string());
    parts.push(Self::format_writing_style(ws));
    parts.push("一致性扫描应检查文本是否偏离设定的写作风格。".to_string());
    parts.push(String::new());
}
```

- [ ] **Step 6: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: No errors

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/services/prompt_builder.rs
git commit -m "feat: inject writing style context into all skill prompts"
```

---

### Task 5: Add WritingStyle frontend type and API

**Files:**
- Modify: `src/types/ai.ts`
- Modify: `src/api/settingsApi.ts`

- [ ] **Step 1: Add WritingStyle interface to types**

In `src/types/ai.ts`, add:

```typescript
export interface WritingStyle {
  languageStyle: "plain" | "balanced" | "ornate" | "colloquial";
  descriptionDensity: number;   // 1-7
  dialogueRatio: number;        // 1-7
  sentenceRhythm: "short" | "long" | "mixed";
  atmosphere: "warm" | "cold" | "humorous" | "serious" | "suspenseful" | "neutral";
  psychologicalDepth: number;   // 1-7
}
```

- [ ] **Step 2: Add default WritingStyle factory**

In `src/types/ai.ts`, add:

```typescript
export function defaultWritingStyle(): WritingStyle {
  return {
    languageStyle: "balanced",
    descriptionDensity: 4,
    dialogueRatio: 4,
    sentenceRhythm: "mixed",
    atmosphere: "neutral",
    psychologicalDepth: 4,
  };
}
```

- [ ] **Step 3: Add API functions**

In `src/api/settingsApi.ts`, add:

```typescript
import type { WritingStyle } from "../types/ai.js";
export type { WritingStyle } from "../types/ai.js";

export async function saveWritingStyle(
  projectRoot: string,
  writingStyle: WritingStyle,
): Promise<void> {
  return invokeCommand<void>("save_writing_style", {
    input: { projectRoot, writingStyle },
  });
}
```

- [ ] **Step 4: Run typecheck**

Run: `pnpm typecheck`
Expected: No errors

- [ ] **Step 5: Commit**

```bash
git add src/types/ai.ts src/api/settingsApi.ts
git commit -m "feat: add WritingStyle frontend types and API"
```

---

### Task 6: Build the Writing Style Settings UI

**Files:**
- Modify: `src/pages/Settings/SettingsPage.tsx`

- [ ] **Step 1: Add TabKey and tab entry**

```typescript
// Change TabKey to add "writing":
type TabKey = "model" | "routing" | "skills" | "editor" | "writing" | "backup" | "about";

// Add tab button between "编辑器" and "数据与备份":
{ key: "writing", label: "写作风格" },
```

- [ ] **Step 2: Add writing style state to SettingsPage**

```typescript
// Add imports:
import { defaultWritingStyle, type WritingStyle } from "../../types/ai.js";

// Add state alongside existing editor state:
const [writingStyle, setWritingStyle] = useState<WritingStyle>(defaultWritingStyle());
const [writingStyleLoaded, setWritingStyleLoaded] = useState(false);
const [writingStyleSaving, setWritingStyleSaving] = useState(false);
const [writingStyleSaved, setWritingStyleSaved] = useState(false);
```

- [ ] **Step 3: Load writing style from current project**

Add to the existing useEffect (alongside editor settings load):

```typescript
// Load writing style from current project
if (projectRoot) {
  // We need a get_writing_style command or we can extract it from the current project
  // Since store already has currentProject?.settings, we read writingStyle from there
  const project = useProjectStore.getState().currentProject;
  if (project?.settings?.writingStyle) {
    setWritingStyle(project.settings.writingStyle);
  }
  setWritingStyleLoaded(true);
}
```

- [ ] **Step 4: Add the writing style tab content**

Before the `{activeTab === "backup"` block, add the new tab:

```tsx
{activeTab === "writing" && (
  <Card padding="lg" className="space-y-6">
    <h2 className="text-base font-semibold text-surface-100">写作风格</h2>
    <p className="text-sm text-surface-400">
      设定作品的默认写作风格参数。AI 生成时将遵循这些参数输出相应风格的文本。
    </p>

    {/* Language Style */}
    <div>
      <label className="text-sm text-surface-200 block mb-2">语言风格</label>
      <div className="flex gap-2">
        {(["plain", "balanced", "ornate", "colloquial"] as const).map((opt) => (
          <button
            key={opt}
            onClick={() => setWritingStyle({ ...writingStyle, languageStyle: opt })}
            className={`px-4 py-2 text-sm rounded-lg border transition-colors ${
              writingStyle.languageStyle === opt
                ? "bg-primary text-white border-primary"
                : "bg-surface-800 text-surface-300 border-surface-600 hover:border-surface-500"
            }`}
          >
            {{ plain: "平实", balanced: "适中", ornate: "华丽", colloquial: "口语化" }[opt]}
          </button>
        ))}
      </div>
    </div>

    {/* Description Density slider */}
    <SliderControl
      label="描写密度"
      value={writingStyle.descriptionDensity}
      minLabel="点到为止"
      maxLabel="详细刻画"
      onChange={(val) => setWritingStyle({ ...writingStyle, descriptionDensity: val })}
    />

    {/* Dialogue Ratio slider */}
    <SliderControl
      label="对话比例"
      value={writingStyle.dialogueRatio}
      minLabel="偏叙述"
      maxLabel="偏对话"
      onChange={(val) => setWritingStyle({ ...writingStyle, dialogueRatio: val })}
    />

    {/* Sentence Rhythm */}
    <div>
      <label className="text-sm text-surface-200 block mb-2">句子节奏</label>
      <div className="flex gap-2">
        {(["short", "mixed", "long"] as const).map((opt) => (
          <button
            key={opt}
            onClick={() => setWritingStyle({ ...writingStyle, sentenceRhythm: opt })}
            className={`px-4 py-2 text-sm rounded-lg border transition-colors ${
              writingStyle.sentenceRhythm === opt
                ? "bg-primary text-white border-primary"
                : "bg-surface-800 text-surface-300 border-surface-600 hover:border-surface-500"
            }`}
          >
            {{ short: "短句为主", long: "长句为主", mixed: "混合" }[opt]}
          </button>
        ))}
      </div>
    </div>

    {/* Atmosphere */}
    <div>
      <label className="text-sm text-surface-200 block mb-2">氛围基调</label>
      <div className="flex flex-wrap gap-2">
        {(["warm", "cold", "humorous", "serious", "suspenseful", "neutral"] as const).map((opt) => (
          <button
            key={opt}
            onClick={() => setWritingStyle({ ...writingStyle, atmosphere: opt })}
            className={`px-4 py-2 text-sm rounded-lg border transition-colors ${
              writingStyle.atmosphere === opt
                ? "bg-primary text-white border-primary"
                : "bg-surface-800 text-surface-300 border-surface-600 hover:border-surface-500"
            }`}
          >
            {{ warm: "温暖", cold: "冷峻", humorous: "幽默", serious: "严肃", suspenseful: "悬疑", neutral: "中性" }[opt]}
          </button>
        ))}
      </div>
    </div>

    {/* Psychological Depth slider */}
    <SliderControl
      label="心理描写深度"
      value={writingStyle.psychologicalDepth}
      minLabel="仅外部行为"
      maxLabel="深入内心"
      onChange={(val) => setWritingStyle({ ...writingStyle, psychologicalDepth: val })}
    />

    {/* Save button */}
    <div className="flex items-center gap-3 pt-3 border-t border-surface-700">
      <Button
        variant="primary"
        onClick={async () => {
          if (!projectRoot) return;
          setWritingStyleSaving(true);
          try {
            await saveWritingStyle(projectRoot, writingStyle);
            setWritingStyleSaved(true);
            setTimeout(() => setWritingStyleSaved(false), 2000);
          } catch (err: unknown) {
            const msg = typeof err === "object" && err && "message" in err
              ? String((err as { message: string }).message)
              : "保存失败";
            setWritingStyleSaved(false);
            console.error("Save writing style failed:", msg);
          } finally {
            setWritingStyleSaving(false);
          }
        }}
        disabled={writingStyleSaving || !projectRoot}
      >
        {writingStyleSaving ? "保存中..." : writingStyleSaved ? "已保存 ✓" : "保存风格设置"}
      </Button>
      {!projectRoot && (
        <span className="text-xs text-warning">请先打开项目以设置写作风格</span>
      )}
    </div>
  </Card>
)}
```

- [ ] **Step 5: Add SliderControl component**

Add before the SettingsPage component or in a separate file `src/components/settings/SliderControl.tsx`:

```tsx
interface SliderControlProps {
  label: string;
  value: number;
  minLabel: string;
  maxLabel: string;
  min?: number;
  max?: number;
  onChange: (val: number) => void;
}

function SliderControl({ label, value, minLabel, maxLabel, min = 1, max = 7, onChange }: SliderControlProps) {
  return (
    <div>
      <label className="text-sm text-surface-200 block mb-2">{label}</label>
      <div className="flex items-center gap-3">
        <span className="text-xs text-surface-400 w-16 text-right shrink-0">{minLabel}</span>
        <div className="flex gap-1.5 flex-1 justify-center">
          {Array.from({ length: max - min + 1 }, (_, i) => min + i).map((n) => (
            <button
              key={n}
              onClick={() => onChange(n)}
              className={`w-8 h-8 rounded-full text-xs font-medium transition-colors ${
                n === value
                  ? "bg-primary text-white"
                  : n < value
                    ? "bg-primary/20 text-primary border border-primary/30"
                    : "bg-surface-800 text-surface-500 border border-surface-600"
              }`}
            >
              {n}
            </button>
          ))}
        </div>
        <span className="text-xs text-surface-400 w-16 shrink-0">{maxLabel}</span>
      </div>
    </div>
  );
}
```

- [ ] **Step 6: Run typecheck**

Run: `pnpm typecheck`
Expected: No errors

- [ ] **Step 7: Commit**

```bash
git add src/pages/Settings/SettingsPage.tsx src/components/settings/SliderControl.tsx src/api/settingsApi.ts
git commit -m "feat: add writing style settings tab with 7-point scale controls"
```

---

### Task 7: Inject writing style into built-in skill prompts

**Files:**
- Modify: `resources/builtin-skills/*.md`

The style parameters are now injected via `{projectContext}` in `prompt_builder.rs` (Task 4). No changes needed to `.md` files — they already reference `{projectContext}` which will now contain the writing style block.

Verify: each `.md` file's prompt includes `{projectContext}` and the style info will appear there. Files that don't use `{projectContext}` (like `prose.naturalize.md` which only receives `{selectedText}`) won't get style injection — this is correct since those skills operate on existing text.

- [ ] **Step 1: Audit built-in skills for style coverage**

Check which skills receive `{projectContext}`:
- `chapter.draft.md` — ✓ uses {projectContext} + {chapterContext}
- `chapter.continue.md` — ✓ uses {projectContext}
- `chapter.rewrite.md` — ✓ uses {projectContext}  
- `chapter.plan.md` — ✓ uses {projectContext} + {chapterContext}
- `character.create.md` — ✓ uses {projectContext}
- `consistency.scan.md` — ✓ uses {projectContext} + {chapterContext}
- `blueprint.generate_step.md` — ✓ uses {projectContext}
- `world.create_rule.md` — ✓ uses {projectContext}
- `plot.create_node.md` — ✓ uses {projectContext}
- `prose.naturalize.md` — does NOT use {projectContext} (operates on selectedText only)

For prose.naturalize, the style is less relevant (it's cleaning existing text). No change needed.

- [ ] **Step 2: Verify Rust compilation**

Run: `cd src-tauri && cargo check`
Expected: No errors

- [ ] **Step 3: Verify TypeScript typecheck**

Run: `pnpm typecheck`
Expected: No errors

- [ ] **Step 4: Commit**

```bash
git commit -m "chore: confirm built-in skills receive writing style via projectContext"
```
