import { useEffect, useMemo, useState } from "react";
import { Card } from "../../components/cards/Card";
import { Button } from "../../components/ui/Button";
import { Modal } from "../../components/dialogs/Modal";
import { Textarea } from "../../components/forms/Textarea";
import { getRelationshipGraphData, type CharacterRelationship, type CharacterRow } from "../../api/characterApi";
import { runModuleAiTask } from "../../api/moduleAiApi";
import { useProjectStore } from "../../stores/projectStore";
import { useUiStore } from "../../stores/uiStore";

interface NodePosition {
  x: number;
  y: number;
}

const GRAPH_SIZE = 560;
const NODE_WIDTH = 116;
const NODE_HEIGHT = 48;

export function RelationshipsPage() {
  const projectRoot = useProjectStore((s) => s.currentProjectPath);
  const setActiveRoute = useUiStore((s) => s.setActiveRoute);

  const [characters, setCharacters] = useState<CharacterRow[]>([]);
  const [relationships, setRelationships] = useState<CharacterRelationship[]>([]);
  const [focusedCharacterId, setFocusedCharacterId] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showAiReview, setShowAiReview] = useState(false);
  const [aiPrompt, setAiPrompt] = useState("");
  const [aiResult, setAiResult] = useState<string | null>(null);
  const [aiError, setAiError] = useState<string | null>(null);
  const [aiLoading, setAiLoading] = useState(false);

  const loadGraphData = async () => {
    if (!projectRoot) {
      setCharacters([]);
      setRelationships([]);
      setFocusedCharacterId(null);
      return;
    }

    setLoading(true);
    setError(null);
    try {
      const { characters: characterRows, relationships: relationRows } =
        await getRelationshipGraphData(projectRoot);
      setCharacters(characterRows);
      setRelationships(relationRows);
      setFocusedCharacterId((current) =>
        current && characterRows.some((item) => item.id === current)
          ? current
          : characterRows[0]?.id || null
      );
    } catch (err) {
      setError(err instanceof Error ? err.message : "加载关系图失败");
      setCharacters([]);
      setRelationships([]);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void loadGraphData();
  }, [projectRoot]);

  const nodePositions = useMemo(() => {
    const map = new Map<string, NodePosition>();
    const count = characters.length;
    if (count === 0) return map;

    const center = GRAPH_SIZE / 2;
    const radius = Math.max(130, Math.min(220, 120 + count * 10));
    characters.forEach((character, index) => {
      const angle = (Math.PI * 2 * index) / count - Math.PI / 2;
      map.set(character.id, {
        x: center + radius * Math.cos(angle),
        y: center + radius * Math.sin(angle)
      });
    });
    return map;
  }, [characters]);

  const focusedCharacter = characters.find((c) => c.id === focusedCharacterId) ?? null;
  const focusedLinks = useMemo(
    () =>
      relationships.filter((link) =>
        focusedCharacterId
          ? link.sourceCharacterId === focusedCharacterId || link.targetCharacterId === focusedCharacterId
          : true
      ),
    [relationships, focusedCharacterId]
  );

  function getCharacterName(id: string): string {
    return characters.find((character) => character.id === id)?.name ?? "未知角色";
  }

  return (
    <div className="max-w-6xl mx-auto">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold text-surface-100">角色关系图</h1>
          <p className="text-sm text-surface-400 mt-1">可视化查看角色连接并跳转到角色页面处理细节</p>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="ghost" size="sm" onClick={() => { setShowAiReview(true); setAiError(null); setAiResult(null); }}>
            AI 审阅
          </Button>
          <Button variant="secondary" size="sm" onClick={() => setActiveRoute("characters")}>
            前往角色页
          </Button>
        </div>
      </div>

      <div className="mb-4 rounded-lg border border-surface-700 bg-surface-800/70 px-3 py-2">
        <p className="text-xs text-surface-400">
          本页为派生审阅层，展示已晋升的正式数据与待审信息，不是一级事实源。
        </p>
      </div>

      {loading ? (
        <Card padding="lg">
          <p className="text-sm text-surface-400">加载中...</p>
        </Card>
      ) : error ? (
        <Card padding="lg" className="border-error/30 bg-error/5">
          <p className="text-sm text-error">{error}</p>
        </Card>
      ) : characters.length === 0 ? (
        <Card padding="lg">
          <p className="text-sm text-surface-400">暂无角色，先在角色页创建角色后再查看关系图。</p>
        </Card>
      ) : (
        <div className="grid grid-cols-1 xl:grid-cols-[1fr_320px] gap-4">
          <Card padding="md" className="relative overflow-hidden">
            <div className="relative mx-auto" style={{ width: GRAPH_SIZE, height: GRAPH_SIZE }}>
              <svg width={GRAPH_SIZE} height={GRAPH_SIZE} className="absolute inset-0">
                {relationships.map((link) => {
                  const source = nodePositions.get(link.sourceCharacterId);
                  const target = nodePositions.get(link.targetCharacterId);
                  if (!source || !target) return null;
                  return (
                    <g key={link.id}>
                      <line
                        x1={source.x}
                        y1={source.y}
                        x2={target.x}
                        y2={target.y}
                        stroke="#475569"
                        strokeWidth={1.5}
                        strokeDasharray={focusedCharacterId && (focusedCharacterId === link.sourceCharacterId || focusedCharacterId === link.targetCharacterId) ? "0" : "4 4"}
                      />
                    </g>
                  );
                })}
              </svg>

              {characters.map((character) => {
                const point = nodePositions.get(character.id);
                if (!point) return null;
                const active = focusedCharacterId === character.id;
                return (
                  <button
                    key={character.id}
                    onClick={() => setFocusedCharacterId(character.id)}
                    className={`absolute px-3 py-2 rounded-lg border text-left shadow-sm transition-colors ${
                      active
                        ? "bg-primary/20 border-primary/40 text-primary"
                        : "bg-surface-800 border-surface-700 text-surface-200 hover:border-surface-500"
                    }`}
                    style={{
                      left: point.x - NODE_WIDTH / 2,
                      top: point.y - NODE_HEIGHT / 2,
                      width: NODE_WIDTH,
                      minHeight: NODE_HEIGHT
                    }}
                  >
                    <div className="text-sm font-medium truncate">{character.name}</div>
                    <div className="text-[11px] text-surface-400 truncate">{character.role_type}</div>
                  </button>
                );
              })}
            </div>
          </Card>

          <Card padding="md" className="space-y-3">
            <h2 className="text-sm font-semibold text-surface-200">
              {focusedCharacter ? `${focusedCharacter.name} 的关系` : "关系明细"}
            </h2>
            {focusedLinks.length === 0 ? (
              <p className="text-xs text-surface-500">暂无关系记录</p>
            ) : (
              <div className="space-y-2 max-h-[520px] overflow-y-auto pr-1">
                {focusedLinks.map((link) => (
                  <div key={link.id} className="p-2 rounded-lg bg-surface-800 border border-surface-700">
                    <p className="text-sm text-surface-200">
                      {getCharacterName(link.sourceCharacterId)}
                      <span className="text-surface-500 mx-1">→</span>
                      {getCharacterName(link.targetCharacterId)}
                    </p>
                    <p className="text-xs text-primary mt-0.5">{link.relationshipType}</p>
                    {link.description && (
                      <p className="text-xs text-surface-400 mt-1 whitespace-pre-wrap break-words">
                        {link.description}
                      </p>
                    )}
                  </div>
                ))}
              </div>
            )}
            <div className="pt-2 border-t border-surface-700">
              <Button variant="primary" size="sm" onClick={() => setActiveRoute("characters")}>
                去角色页维护关系
              </Button>
            </div>
          </Card>
        </div>
      )}

      <Modal open={showAiReview} onClose={() => setShowAiReview(false)} title="AI 关系审阅" width="lg">
        <div className="space-y-4">
          <Textarea
            label="附加要求（可选）"
            value={aiPrompt}
            onChange={(e) => setAiPrompt(e.target.value)}
            placeholder="例如：优先检查主角关系是否缺少关键对立线"
            className="min-h-[90px]"
          />
          <Button
            variant="primary"
            loading={aiLoading}
            onClick={async () => {
              if (!projectRoot) return;
              setAiLoading(true);
              setAiError(null);
              setAiResult(null);
              try {
                const result = await runModuleAiTask({
                  projectRoot,
                  taskType: "relationship.review",
                  persistMode: "derived_review",
                  automationTier: "auto",
                  uiAction: "relationship.ai.review",
                  userInstruction: aiPrompt,
                });
                setAiResult(result || "AI 未返回内容。");
                await loadGraphData();
              } catch (err) {
                setAiError(err instanceof Error ? err.message : "AI 审阅失败");
              } finally {
                setAiLoading(false);
              }
            }}
            disabled={!projectRoot}
          >
            {aiLoading ? "审阅中..." : "生成关系审阅报告"}
          </Button>
          {aiError && (
            <div className="p-3 rounded-lg bg-error/10 border border-error/30 text-sm text-error">
              {aiError}
            </div>
          )}
          {aiResult && (
            <div className="p-4 rounded-xl bg-primary/5 border border-primary/20">
              <pre className="text-sm text-surface-200 whitespace-pre-wrap font-sans leading-relaxed max-h-80 overflow-y-auto">{aiResult}</pre>
            </div>
          )}
        </div>
      </Modal>
    </div>
  );
}
