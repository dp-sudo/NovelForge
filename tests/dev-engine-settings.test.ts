import assert from "node:assert/strict";
import test from "node:test";
import { createTempWorkspace, removeTempWorkspace } from "./helpers/temp-workspace.js";
import { NovelForgeMvp } from "./dev-engine-fixtures/services/novelforge-mvp.js";

test("Dev Engine: 蓝图步骤保存和状态切换正确", async () => {
  const workspace = await createTempWorkspace();
  const mvp = new NovelForgeMvp();
  try {
    const project = await mvp.project.createProject({
      name: "dev-test", genre: "测试", saveDirectory: workspace
    });

    await mvp.blueprint.saveStep(project.projectRoot, "step-01-anchor", "灵感内容");
    const steps = await mvp.blueprint.listSteps(project.projectRoot);
    const step = steps.find((s: any) => s.stepKey === "step-01-anchor");
    assert.ok(step);
    assert.equal(step.content, "灵感内容");

    await mvp.blueprint.markCompleted(project.projectRoot, "step-01-anchor");
    const steps2 = await mvp.blueprint.listSteps(project.projectRoot);
    const step2 = steps2.find((s: any) => s.stepKey === "step-01-anchor");
    assert.ok(step2);
    assert.equal(step2.status, "completed");

    await mvp.blueprint.resetStep(project.projectRoot, "step-01-anchor");
    const steps3 = await mvp.blueprint.listSteps(project.projectRoot);
    const step3 = steps3.find((s: any) => s.stepKey === "step-01-anchor");
    assert.ok(step3);
    assert.equal(step3.status, "not_started");
  } finally {
    await removeTempWorkspace(workspace);
  }
});

test("Dev Engine: 角色创建和修改正确", async () => {
  const workspace = await createTempWorkspace();
  const mvp = new NovelForgeMvp();
  try {
    const project = await mvp.project.createProject({
      name: "char-test", genre: "都市", saveDirectory: workspace
    });

    const id = await mvp.character.create(project.projectRoot, {
      name: "林云", roleType: "主角"
    });
    const chars = await mvp.character.list(project.projectRoot);
    assert.equal(chars.length, 1);
    assert.equal(chars[0].name, "林云");

    await mvp.character.update(project.projectRoot, id, { name: "林云改" });
    const chars2 = await mvp.character.list(project.projectRoot);
    assert.equal(chars2[0].name, "林云改");
  } finally {
    await removeTempWorkspace(workspace);
  }
});

test("Dev Engine: 世界规则和名词库创建正确", async () => {
  const workspace = await createTempWorkspace();
  const mvp = new NovelForgeMvp();
  try {
    const project = await mvp.project.createProject({
      name: "assets-test", genre: "奇幻", saveDirectory: workspace
    });

    await mvp.world.create(project.projectRoot, {
      title: "不得使用禁忌魔法",
      category: "世界规则",
      description: "使用禁忌魔法会唤醒远古生物。",
      constraintLevel: "absolute"
    });
    const rules = await mvp.world.list(project.projectRoot);
    assert.equal(rules.length, 1);
    assert.equal(rules[0].constraint_level, "absolute");

    await mvp.glossary.create(project.projectRoot, {
      term: "深渊之眼", termType: "术语", locked: true
    });
    const terms = await mvp.glossary.list(project.projectRoot);
    assert.equal(terms.length, 1);
    assert.ok(terms[0].locked);
  } finally {
    await removeTempWorkspace(workspace);
  }
});

test("Dev Engine: 剧情节点创建和排序正确", async () => {
  const workspace = await createTempWorkspace();
  const mvp = new NovelForgeMvp();
  try {
    const project = await mvp.project.createProject({
      name: "plot-test", genre: "悬疑", saveDirectory: workspace
    });

    const id1 = await mvp.plot.create(project.projectRoot, {
      title: "开端", nodeType: "开端", sortOrder: 1
    });
    const id2 = await mvp.plot.create(project.projectRoot, {
      title: "高潮", nodeType: "高潮", sortOrder: 2
    });

    const nodes = await mvp.plot.list(project.projectRoot);
    assert.equal(nodes.length, 2);

    await mvp.plot.reorder(project.projectRoot, [id2, id1]);
    const reordered = await mvp.plot.list(project.projectRoot);
    assert.equal(reordered[0].title, "高潮");
  } finally {
    await removeTempWorkspace(workspace);
  }
});
