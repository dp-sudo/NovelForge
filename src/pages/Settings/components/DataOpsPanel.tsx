import { Card } from "../../../components/cards/Card";
import { Input } from "../../../components/forms/Input";
import { Button } from "../../../components/ui/Button";
import type {
  BackupResult,
  IntegrityReport,
  SearchResult,
} from "../../../api/chapterApi";
import type { GitCommitRecord, GitRepositoryStatus } from "../../../api/settingsApi";

interface DataOpsPanelProps {
  projectRoot: string | null;
  backupCreating: boolean;
  integrityChecking: boolean;
  backupRestoring: boolean;
  backupMessage: string | null;
  backupList: BackupResult[];
  integrityReport: IntegrityReport | null;
  semanticQuery: string;
  semanticSearching: boolean;
  semanticRebuilding: boolean;
  semanticMessage: string | null;
  semanticResults: SearchResult[];
  gitStatus: GitRepositoryStatus | null;
  gitHistory: GitCommitRecord[];
  gitBusy: boolean;
  gitMessage: string | null;
  snapshotMessage: string;
  onCreateBackup: () => Promise<void>;
  onCheckIntegrity: () => Promise<void>;
  onRestoreBackup: (backupPath: string) => Promise<void>;
  onSemanticQueryChange: (value: string) => void;
  onSemanticSearch: () => Promise<void>;
  onRebuildVector: () => Promise<void>;
  onInitGitRepo: () => Promise<void>;
  onCommitSnapshot: () => Promise<void>;
  onRefreshGitData: () => Promise<void>;
  onSnapshotMessageChange: (value: string) => void;
}

function getBackupMessageClassName(message: string): string {
  if (message.startsWith("备份成功") || message.startsWith("恢复成功")) {
    return "bg-success/10 text-success border border-success/20";
  }
  if (message.startsWith("备份失败") || message.startsWith("恢复失败")) {
    return "bg-error/10 text-error border border-error/20";
  }
  return "bg-info/10 text-info border border-info/20";
}

export function DataOpsPanel(props: DataOpsPanelProps) {
  const {
    projectRoot,
    backupCreating,
    integrityChecking,
    backupRestoring,
    backupMessage,
    backupList,
    integrityReport,
    semanticQuery,
    semanticSearching,
    semanticRebuilding,
    semanticMessage,
    semanticResults,
    gitStatus,
    gitHistory,
    gitBusy,
    gitMessage,
    snapshotMessage,
    onCreateBackup,
    onCheckIntegrity,
    onRestoreBackup,
    onSemanticQueryChange,
    onSemanticSearch,
    onRebuildVector,
    onInitGitRepo,
    onCommitSnapshot,
    onRefreshGitData,
    onSnapshotMessageChange,
  } = props;

  return (
    <Card padding="lg" className="space-y-4">
      <h2 className="text-base font-semibold text-surface-100">数据与备份</h2>
      <p className="text-sm text-surface-400">项目数据默认保存在本地。支持手动备份和恢复。</p>
      <p className="text-xs text-surface-500">备份包含 project.json、数据库、章节正文和蓝图文件。API 密钥不会进入备份包。</p>
      <div className="flex gap-3">
        <Button variant="secondary" loading={backupCreating} onClick={() => void onCreateBackup()}>
          {backupCreating ? "备份中..." : "创建备份"}
        </Button>
        <Button variant="secondary" loading={integrityChecking} onClick={() => void onCheckIntegrity()}>
          {integrityChecking ? "检查中..." : "完整性检查"}
        </Button>
      </div>
      {backupMessage && (
        <div className={`px-3 py-2 rounded-lg text-sm ${getBackupMessageClassName(backupMessage)}`}>
          {backupMessage}
        </div>
      )}
      {backupList.length > 0 && (
        <div>
          <h3 className="text-sm font-semibold text-surface-200 mb-2">历史备份</h3>
          <div className="space-y-2 max-h-48 overflow-y-auto">
            {backupList.map((backup, index) => (
              <div key={index} className="flex items-center justify-between p-2 bg-surface-800 rounded-lg">
                <div className="text-xs text-surface-300 truncate flex-1">
                  {backup.filePath.split(/[/\\]/).pop()}
                  <span className="text-surface-500 ml-2">({(backup.fileSize / 1024).toFixed(0)} KB)</span>
                </div>
                <Button
                  variant="ghost"
                  size="sm"
                  loading={backupRestoring}
                  onClick={() => void onRestoreBackup(backup.filePath)}
                >
                  恢复
                </Button>
              </div>
            ))}
          </div>
        </div>
      )}
      {integrityReport && (
        <div className="space-y-2">
          <h3 className="text-sm font-semibold text-surface-200">完整性报告</h3>
          <div className="text-xs text-surface-400">
            状态:{" "}
            <span
              className={
                integrityReport.status === "healthy"
                  ? "text-success"
                  : integrityReport.status === "issues_found"
                    ? "text-warning"
                    : "text-error"
              }
            >
              {integrityReport.status}
            </span>
            {" · "}
            schema: {integrityReport.summary.schemaVersion}
            {" · "}
            章节正常: {integrityReport.summary.chaptersOk}
            {" · "}
            缺失: {integrityReport.summary.chaptersMissing}
            {" · "}
            孤立草稿: {integrityReport.summary.orphanDrafts}
          </div>
          {integrityReport.issues.length > 0 && (
            <div className="max-h-40 overflow-y-auto space-y-2 pr-1">
              {integrityReport.issues.map((issue, index) => (
                <div key={`${issue.category}-${index}`} className="text-xs px-3 py-2 rounded-lg bg-surface-800 border border-surface-700">
                  <div
                    className={
                      issue.severity === "error"
                        ? "text-error"
                        : issue.severity === "warning"
                          ? "text-warning"
                          : "text-info"
                    }
                  >
                    [{issue.severity}] {issue.message}
                  </div>
                  {issue.detail && <div className="text-surface-500 mt-1">{issue.detail}</div>}
                </div>
              ))}
            </div>
          )}
        </div>
      )}
      <div className="pt-4 border-t border-surface-700 space-y-3">
        <h3 className="text-sm font-semibold text-surface-200">高级检索诊断</h3>
        <p className="text-xs text-surface-400">
          该入口直接调用语义检索与向量重建命令，仅用于排查检索链路，不影响默认聚合搜索行为。
        </p>
        <Input
          label="语义检索关键词"
          value={semanticQuery}
          onChange={(e) => onSemanticQueryChange(e.target.value)}
          placeholder="例如：主角身世伏笔"
        />
        <div className="flex gap-2">
          <Button
            variant="secondary"
            loading={semanticSearching}
            onClick={() => void onSemanticSearch()}
            disabled={!projectRoot}
          >
            {semanticSearching ? "检索中..." : "语义检索"}
          </Button>
          <Button
            variant="ghost"
            loading={semanticRebuilding}
            onClick={() => void onRebuildVector()}
            disabled={!projectRoot}
          >
            {semanticRebuilding ? "重建中..." : "重建向量索引"}
          </Button>
        </div>
        {semanticMessage && (
          <div className="px-3 py-2 rounded-lg text-xs bg-info/10 text-info border border-info/20">
            {semanticMessage}
          </div>
        )}
        {semanticResults.length > 0 && (
          <div className="max-h-40 overflow-y-auto space-y-2 pr-1">
            {semanticResults.map((item) => (
              <div
                key={`${item.entityType}:${item.entityId}`}
                className="text-xs px-3 py-2 rounded-lg bg-surface-800 border border-surface-700"
              >
                <div className="text-surface-200">
                  [{item.entityType}] {item.title}
                </div>
                <div className="text-surface-500 mt-1">{item.bodySnippet}</div>
              </div>
            ))}
          </div>
        )}
      </div>
      {!projectRoot && (
        <p className="text-xs text-warning">请先打开项目以使用备份功能</p>
      )}
      <div className="pt-4 border-t border-surface-700 space-y-3">
        <h3 className="text-sm font-semibold text-surface-200">Git 快照</h3>
        <p className="text-xs text-surface-400">
          支持初始化仓库、提交项目快照并查看最近历史记录。
        </p>
        <div className="text-xs text-surface-400">
          状态：{gitStatus?.initialized ? `已初始化（${gitStatus.branch}）` : "未初始化"} / {gitStatus?.hasChanges ? "有未提交变更" : "工作区干净"}
        </div>
        <Input
          label="提交说明（可选）"
          value={snapshotMessage}
          onChange={(e) => onSnapshotMessageChange(e.target.value)}
          placeholder="例如：完成第 10 章初稿"
        />
        <div className="flex gap-2">
          <Button variant="secondary" onClick={() => void onInitGitRepo()} disabled={gitBusy}>
            {gitBusy ? "处理中..." : "初始化仓库"}
          </Button>
          <Button variant="primary" onClick={() => void onCommitSnapshot()} disabled={gitBusy || !projectRoot}>
            {gitBusy ? "处理中..." : "提交快照"}
          </Button>
          <Button variant="ghost" onClick={() => void onRefreshGitData()} disabled={gitBusy || !projectRoot}>
            刷新历史
          </Button>
        </div>
        {gitMessage && (
          <div className="px-3 py-2 rounded-lg text-xs bg-info/10 text-info border border-info/20">
            {gitMessage}
          </div>
        )}
        {gitHistory.length > 0 && (
          <div className="space-y-2 max-h-48 overflow-y-auto pr-1">
            {gitHistory.map((row) => (
              <div key={row.commitId} className="text-xs px-3 py-2 rounded-lg bg-surface-800 border border-surface-700">
                <div className="text-surface-200 break-all">{row.commitId.slice(0, 10)} · {row.summary}</div>
                <div className="text-surface-500 mt-1">{new Date(row.committedAt).toLocaleString("zh-CN")}</div>
              </div>
            ))}
          </div>
        )}
      </div>
    </Card>
  );
}
