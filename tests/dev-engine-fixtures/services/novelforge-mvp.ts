// 问题1修复(双运行时收敛): Node 业务实现迁入 tests/dev-engine-fixtures，仅供测试夹具使用。
import { AiService } from "./ai-service.js";
import { BlueprintService } from "./blueprint-service.js";
import { ChapterService } from "./chapter-service.js";
import { CharacterService } from "./character-service.js";
import { ConsistencyService } from "./consistency-service.js";
import { ExportService } from "./export-service.js";
import { GlossaryService } from "./glossary-service.js";
import { PlotService } from "./plot-service.js";
import { ProjectService } from "./project-service.js";
import { SettingsService } from "./settings-service.js";
import { WorldService } from "./world-service.js";

export class NovelForgeMvp {
  public readonly project = new ProjectService();
  public readonly blueprint = new BlueprintService();
  public readonly character = new CharacterService();
  public readonly world = new WorldService();
  public readonly glossary = new GlossaryService();
  public readonly plot = new PlotService();
  public readonly chapter = new ChapterService();
  public readonly settings = new SettingsService();
  public readonly ai = new AiService();
  public readonly consistency = new ConsistencyService();
  public readonly export = new ExportService();
}
