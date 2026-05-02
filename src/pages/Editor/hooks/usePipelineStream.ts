import { useCallback, useEffect, useRef } from "react";
import { cancelTaskPipeline, streamTaskPipeline } from "../../../api/pipelineApi.js";
import type { AiStreamStatus } from "../../../stores/editorStore.js";

const PIPELINE_PHASE_LABEL: Record<string, string> = {
  validate: "参数校验",
  context: "上下文聚合",
  route: "任务路由",
  prompt: "提示词构建",
  generate: "模型生成",
  postprocess: "结果整理",
  persist: "结果落库",
  done: "完成",
  run: "任务启动"
};

const PIPELINE_SUGGESTION_BY_ERROR_CODE: Record<string, string> = {
  PIPELINE_SELECTED_TEXT_REQUIRED: "先在正文中选中一段文本，再重试该任务。",
  PIPELINE_USER_INSTRUCTION_REQUIRED: "先输入任务描述，再重新执行。",
  PIPELINE_CHAPTER_ID_REQUIRED: "先选择目标章节，再执行该任务。",
  PIPELINE_CHAPTER_CONTENT_REQUIRED: "先保存或填写章节内容，再执行一致性扫描。",
  TASK_ROUTE_NOT_FOUND: "前往 设置 > 任务路由，为该任务配置供应商和模型ID。",
  MODEL_NOT_CONFIGURED: "前往 设置 > 模型配置，补齐该供应商的默认模型。",
  PROVIDER_NOT_FOUND: "前往 设置 > 模型配置，确认供应商已创建且可用。",
  LLM_ADAPTER_NOT_FOUND: "前往 设置 > 模型配置，先测试并重新加载该供应商。",
  LLM_NO_PROVIDER: "前往 设置 > 任务路由或模型配置，补齐可用模型后重试。",
  PIPELINE_START_TIMEOUT: "任务启动超时，可能是开发热重载或后端回调中断，重试前先确认页面未重载。",
  PIPELINE_FIRST_EVENT_TIMEOUT: "任务已启动但未收到事件，可能存在事件监听竞态或开发热重载，请重试。",
  PIPELINE_EVENT_TIMEOUT: "模型响应超时，请检查网络/API 密钥或切换模型后重试。",
  PIPELINE_CANCELLED: "任务已取消，可重新发起。",
  PIPELINE_FREEZE_CONFLICT: "请到 蓝图 > 章节路线 > 确定性分区 调整冻结区，或修改指令避免改写冻结事实。"
};

const PIPELINE_SUGGESTION_BY_PHASE: Record<string, string> = {
  validate: "检查输入参数（章节、选区、任务描述）后重试。",
  context: "检查项目目录和章节数据是否可读，必要时重开项目。",
  route: "检查任务路由的供应商与模型ID是否已配置。",
  prompt: "检查技能模板或提示词参数是否完整。",
  generate: "检查 API 密钥、模型可用性与网络状态，必要时切换模型。",
  postprocess: "模型返回格式异常，请重试；若持续失败可更换模型。",
  persist: "检查项目目录写权限和数据库状态后重试。",
  run: "检查控制台日志，确认后端命令是否执行成功。"
};

export interface PipelineErrorEvent {
  phase?: string;
  errorCode?: string;
  message?: string;
  recoverable?: boolean;
}

export interface StartPipelineInput {
  projectRoot: string;
  chapterId: string;
  taskType: string;
  userInstruction: string;
  selectedText?: string;
  chapterContent?: string;
}

interface UsePipelineStreamOptions {
  setAiRequestId: (id: string | null) => void;
  setAiStreamStatus: (status: AiStreamStatus) => void;
  setAiStreamError: (message: string | null) => void;
  appendAiPreviewContent: (delta: string) => void;
}

interface UsePipelineStreamResult {
  startPipeline: (input: StartPipelineInput) => Promise<void>;
  cancelActivePipeline: (reason?: string) => Promise<void>;
  formatPipelineError: (event: PipelineErrorEvent) => string;
}

export function usePipelineStream(options: UsePipelineStreamOptions): UsePipelineStreamResult {
  const {
    setAiRequestId,
    setAiStreamStatus,
    setAiStreamError,
    appendAiPreviewContent,
  } = options;

  const activeAiRequestIdRef = useRef<string | null>(null);
  const aiRunTokenRef = useRef(0);

  const formatPipelineError = useCallback((event: PipelineErrorEvent) => {
    const suggestion = (() => {
      if (event.errorCode && PIPELINE_SUGGESTION_BY_ERROR_CODE[event.errorCode]) {
        return PIPELINE_SUGGESTION_BY_ERROR_CODE[event.errorCode];
      }
      if (event.phase && PIPELINE_SUGGESTION_BY_PHASE[event.phase]) {
        return PIPELINE_SUGGESTION_BY_PHASE[event.phase];
      }
      if (event.recoverable === false) {
        return "建议查看控制台日志，并在确认模型/路由配置后再重试。";
      }
      return "请检查控制台日志与模型配置后重试。";
    })();

    const parts: string[] = [];
    if (event.phase) {
      parts.push(`阶段: ${PIPELINE_PHASE_LABEL[event.phase] ?? event.phase}`);
    }
    if (event.errorCode) {
      parts.push(`错误码: ${event.errorCode}`);
    }
    if (event.message) {
      parts.push(event.message);
    }
    parts.push(`建议: ${suggestion}`);
    return parts.join(" | ") || "AI 生成异常，请检查控制台日志";
  }, []);

  const cancelActivePipeline = useCallback(async (reason: string = "manual") => {
    const requestId = activeAiRequestIdRef.current;
    if (!requestId) return;
    activeAiRequestIdRef.current = null;
    aiRunTokenRef.current += 1;
    setAiRequestId(null);
    await cancelTaskPipeline(requestId, reason).catch(() => undefined);
  }, [setAiRequestId]);

  const startPipeline = useCallback(async (input: StartPipelineInput) => {
    await cancelActivePipeline("new_request");
    const runToken = aiRunTokenRef.current + 1;
    aiRunTokenRef.current = runToken;

    try {
      const stream = streamTaskPipeline({
        projectRoot: input.projectRoot,
        taskType: input.taskType,
        chapterId: input.chapterId,
        uiAction: `editor.ai.${input.taskType}`,
        userInstruction: input.userInstruction,
        selectedText: input.selectedText,
        chapterContent: input.chapterContent,
        persistMode: "none",
        automationTier: "supervised",
      });

      for await (const event of stream) {
        if (runToken !== aiRunTokenRef.current) {
          break;
        }
        if (activeAiRequestIdRef.current !== event.requestId) {
          activeAiRequestIdRef.current = event.requestId;
          setAiRequestId(event.requestId);
        }
        if (event.type === "delta" && event.delta) {
          appendAiPreviewContent(event.delta);
          continue;
        }
        if (event.type === "done") {
          setAiStreamStatus("completed");
          continue;
        }
        if (event.type === "error") {
          setAiStreamError(formatPipelineError(event));
          setAiStreamStatus("error");
        }
      }
    } catch (err) {
      if (runToken !== aiRunTokenRef.current) {
        return;
      }
      const fallback = (() => {
        if (err && typeof err === "object") {
          const candidate = err as { code?: unknown; message?: unknown };
          return formatPipelineError({
            phase: "run",
            errorCode: typeof candidate.code === "string" ? candidate.code : undefined,
            message: typeof candidate.message === "string" ? candidate.message : undefined,
          });
        }
        return formatPipelineError({ phase: "run", message: "AI 生成异常，请检查控制台日志" });
      })();
      setAiStreamError(fallback);
      setAiStreamStatus("error");
    } finally {
      if (runToken === aiRunTokenRef.current) {
        activeAiRequestIdRef.current = null;
        setAiRequestId(null);
      }
    }
  }, [
    appendAiPreviewContent,
    cancelActivePipeline,
    formatPipelineError,
    setAiRequestId,
    setAiStreamError,
    setAiStreamStatus,
  ]);

  useEffect(() => {
    return () => {
      void cancelActivePipeline("unmount");
    };
  }, [cancelActivePipeline]);

  return {
    startPipeline,
    cancelActivePipeline,
    formatPipelineError,
  };
}
