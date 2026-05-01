import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import test from "node:test";

const REPO_ROOT = process.cwd();

async function readRepoFile(relativePath: string): Promise<string> {
  return fs.readFile(path.join(REPO_ROOT, relativePath), "utf-8");
}

test("任务路由契约：仅允许已配置供应商进入路由可选集", async () => {
  const page = await readRepoFile("src/pages/Settings/SettingsPage.tsx");
  const routingPanel = await readRepoFile("src/pages/Settings/components/ModelRoutingPanel.tsx");
  assert.match(page, /const providerIdsForRouting = VENDOR_PRESETS/);
  assert.equal(
    /: VENDOR_PRESETS\.map\(\(preset\) => preset\.id\);/.test(page),
    false,
    "仍存在“未配置时回退全部预设供应商”逻辑",
  );
  assert.match(routingPanel, /请先在“模型设置”中保存至少一个供应商/);
});

test("Task5 契约：Settings 页面接入路由与数据运维拆分组件", async () => {
  const page = await readRepoFile("src/pages/Settings/SettingsPage.tsx");
  assert.match(page, /ModelRoutingPanel/);
  assert.match(page, /DataOpsPanel/);
});
