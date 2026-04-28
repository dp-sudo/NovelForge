import type {
  AiPreviewRequest,
  ChapterInput,
  CreateProjectInput,
  ExportOptions,
  GlossaryTermInput,
  PlotNodeInput,
  ProviderConfigInput,
  WorldRuleInput
} from "../domain/types.js";
import { NovelForgeMvp } from "../services/novelforge-mvp.js";
import type { BlueprintStepKey } from "../domain/constants.js";

const mvp = new NovelForgeMvp();

export async function createProject(input: CreateProjectInput) {
  return mvp.project.createProject(input);
}

export async function openProject(projectRoot: string) {
  return mvp.project.openProject(projectRoot);
}

export async function saveBlueprintStep(
  projectRoot: string,
  stepKey: BlueprintStepKey,
  content: string,
  aiGenerated = false
) {
  return mvp.blueprint.saveStep(projectRoot, stepKey, content, aiGenerated);
}

export async function createCharacter(projectRoot: string, input: Parameters<typeof mvp.character.create>[1]) {
  return mvp.character.create(projectRoot, input);
}

export async function createWorldRule(projectRoot: string, input: WorldRuleInput) {
  return mvp.world.create(projectRoot, input);
}

export async function createGlossaryTerm(projectRoot: string, input: GlossaryTermInput) {
  return mvp.glossary.create(projectRoot, input);
}

export async function createPlotNode(projectRoot: string, input: PlotNodeInput) {
  return mvp.plot.create(projectRoot, input);
}

export async function createChapter(projectRoot: string, input: ChapterInput) {
  return mvp.chapter.createChapter(projectRoot, input);
}

export async function saveChapter(projectRoot: string, chapterId: string, content: string) {
  return mvp.chapter.saveChapterContent(projectRoot, chapterId, content);
}

export async function autosaveChapter(projectRoot: string, chapterId: string, content: string) {
  return mvp.chapter.autosaveDraft(projectRoot, chapterId, content);
}

export async function recoverChapterDraft(projectRoot: string, chapterId: string) {
  return mvp.chapter.recoverDraft(projectRoot, chapterId);
}

export async function configureProvider(projectRoot: string, input: ProviderConfigInput) {
  return mvp.settings.saveProviderConfig(projectRoot, input);
}

export async function generateAiPreview(projectRoot: string, input: AiPreviewRequest) {
  return mvp.ai.generatePreview(projectRoot, input);
}

export async function scanConsistency(projectRoot: string, chapterId: string) {
  return mvp.consistency.scanChapter(projectRoot, chapterId);
}

export async function exportChapter(
  projectRoot: string,
  chapterId: string,
  format: "txt" | "md" | "docx" | "pdf" | "epub",
  outputPath: string,
  options?: ExportOptions
) {
  return mvp.export.exportChapter(projectRoot, chapterId, format, outputPath, options);
}

export async function exportBook(
  projectRoot: string,
  format: "txt" | "md" | "docx" | "pdf" | "epub",
  outputPath: string,
  options?: ExportOptions
) {
  return mvp.export.exportBook(projectRoot, format, outputPath, options);
}
