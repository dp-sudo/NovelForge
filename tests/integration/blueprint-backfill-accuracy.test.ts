import assert from "node:assert/strict";
import test from "node:test";

import { BLUEPRINT_DEFAULTS, parseBlueprintContent } from "../../src/domain/types.js";

type StepCase = {
  stepKey: string;
  payload: unknown;
  expected: Record<string, string>;
};

const STEP_CASES: StepCase[] = [
  {
    stepKey: "step-01-anchor",
    payload: {
      data: {
        核心灵感: "废墟文明中的求生火种",
        主题命题: "自由要付出秩序的代价",
        emotion: "压抑中带倔强",
        目标读者: "18-35 岁幻想读者",
        卖点: "修真与反乌托邦混搭",
        预期: "高冲突和高反转",
      },
    },
    expected: {
      coreInspiration: "废墟文明中的求生火种",
      coreProposition: "自由要付出秩序的代价",
      coreEmotion: "压抑中带倔强",
      targetReader: "18-35 岁幻想读者",
      sellingPoint: "修真与反乌托邦混搭",
      readerExpectation: "高冲突和高反转",
    },
  },
  {
    stepKey: "step-02-genre",
    payload: {
      content: {
        主类型: "仙侠",
        子题材: "复仇成长",
        叙事视角: "第三人称限知",
        风格关键词: "冷峻,克制,刀锋感",
        节奏: "前稳后爆",
        禁用风格: "搞笑跳脱",
      },
    },
    expected: {
      mainGenre: "仙侠",
      subGenre: "复仇成长",
      narrativePov: "第三人称限知",
      styleKeywords: "冷峻,克制,刀锋感",
      rhythmType: "前稳后爆",
      bannedStyle: "搞笑跳脱",
    },
  },
  {
    stepKey: "step-03-premise",
    payload: {
      fields: {
        一句话梗概: "孤剑客为灭门真相逆天改命。",
        三段式梗概: "开端受辱，中段失控，终局重生。",
        开端: "主角在雪夜重返故地。",
        中段: "追查线索却反被利用。",
        高潮: "在宗门祭典公开审判仇首。",
        结局: "复仇后重建秩序并学会生活。",
      },
    },
    expected: {
      oneLineLogline: "孤剑客为灭门真相逆天改命。",
      threeParagraphSummary: "开端受辱，中段失控，终局重生。",
      beginning: "主角在雪夜重返故地。",
      middle: "追查线索却反被利用。",
      climax: "在宗门祭典公开审判仇首。",
      ending: "复仇后重建秩序并学会生活。",
    },
  },
  {
    stepKey: "step-04-characters",
    payload: {
      payload: {
        主角: "沈惊寒",
        反派: "玄霄宗主",
        配角: "师弟遗孤,旧友医者",
        角色关系: "主角与遗孤是守护关系，与医者是互相救赎。",
        成长弧线: "从只会杀人到承担重建责任。",
      },
    },
    expected: {
      protagonist: "沈惊寒",
      antagonist: "玄霄宗主",
      supportingCharacters: "师弟遗孤,旧友医者",
      relationshipSummary: "主角与遗孤是守护关系，与医者是互相救赎。",
      growthArc: "从只会杀人到承担重建责任。",
    },
  },
  {
    stepKey: "step-05-world",
    payload: {
      result: {
        世界背景: "灵气衰退后的宗门割据时代",
        规则体系: "修行需付出寿元代价",
        地点: "寒川,赤霄城,葬剑谷",
        势力: "玄霄宗,寒川盟,黑市商会",
        铁律: "不得逆转生死",
      },
    },
    expected: {
      worldBackground: "灵气衰退后的宗门割据时代",
      rules: "修行需付出寿元代价",
      locations: "寒川,赤霄城,葬剑谷",
      organizations: "玄霄宗,寒川盟,黑市商会",
      inviolableRules: "不得逆转生死",
    },
  },
  {
    stepKey: "step-06-glossary",
    payload: {
      data: {
        人名: "沈惊寒,苏晚棠,岳沉舟",
        地名: "寒川,青霄台,赤霄城",
        组织名: "玄霄宗,寒川盟",
        术语: "剑脉,逆命印,祭火阵",
        别名: "沈惊寒=寒剑",
        禁词: "现代网络词",
      },
    },
    expected: {
      personNames: "沈惊寒,苏晚棠,岳沉舟",
      placeNames: "寒川,青霄台,赤霄城",
      organizationNames: "玄霄宗,寒川盟",
      terms: "剑脉,逆命印,祭火阵",
      aliases: "沈惊寒=寒剑",
      bannedTerms: "现代网络词",
    },
  },
  {
    stepKey: "step-07-plot",
    payload: {
      content: {
        主线目标: "揭露灭门真相并重建秩序",
        阶段节点: "追查线索;潜入敌宗;公开审判",
        关键冲突: "复仇冲动与守护责任冲突",
        反转: "师门惨案幕后另有操盘者",
        高潮: "祭典决战",
        结局: "主角放下执念并承担新秩序",
      },
    },
    expected: {
      mainGoal: "揭露灭门真相并重建秩序",
      stages: "追查线索;潜入敌宗;公开审判",
      keyConflicts: "复仇冲动与守护责任冲突",
      twists: "师门惨案幕后另有操盘者",
      climax: "祭典决战",
      ending: "主角放下执念并承担新秩序",
    },
  },
  {
    stepKey: "step-08-chapters",
    payload: {
      data: {
        卷结构: "三卷九十章",
        章节列表: "卷一起势;卷二破局;卷三终战",
        章节目标: "每章推进主线或角色弧线",
        出场人物: "沈惊寒,苏晚棠,岳沉舟",
        关联主线节点: "寒川追凶->赤霄潜入->祭典审判",
      },
    },
    expected: {
      volumeStructure: "三卷九十章",
      chapterList: "卷一起势;卷二破局;卷三终战",
      chapterGoals: "每章推进主线或角色弧线",
      characters: "沈惊寒,苏晚棠,岳沉舟",
      plotNodes: "寒川追凶->赤霄潜入->祭典审判",
    },
  },
];

test("问题5回填准确率：蓝图 8 步字段可 100% 映射到表单", () => {
  for (const step of STEP_CASES) {
    const parsed = parseBlueprintContent(step.stepKey, JSON.stringify(step.payload));
    assert.deepEqual(parsed, step.expected, `step ${step.stepKey} mapped value mismatch`);

    const defaults = BLUEPRINT_DEFAULTS[step.stepKey] as Record<string, string>;
    const total = Object.keys(defaults).length;
    const filled = Object.values(parsed).filter((value) => value.trim().length > 0).length;
    assert.equal(filled, total, `step ${step.stepKey} should fill all ${total} fields`);
  }
});

