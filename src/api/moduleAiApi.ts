import { streamTaskPipeline, type RunTaskPipelineInput } from "./pipelineApi.js";

export async function runModuleAiTask(input: RunTaskPipelineInput): Promise<string> {
  // 问题2修复(命令面收敛): 模块 AI 统一走 run_ai_task_pipeline + ai:pipeline:event。
  let output = "";
  for await (const event of streamTaskPipeline(input, { timeoutMs: 180000 })) {
    if (event.type === "delta" && event.delta) {
      output += event.delta;
      continue;
    }
    if (event.type === "error") {
      const message = event.message?.trim();
      const errorCode = event.errorCode?.trim();
      if (errorCode && message) {
        throw new Error(`[${errorCode}] ${message}`);
      }
      throw new Error(message || errorCode || "AI 任务执行失败");
    }
  }
  return output.trim();
}
