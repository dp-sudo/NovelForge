import test from "node:test";
import assert from "node:assert/strict";

import {
  SYSTEM_NAV_ROUTES,
  selectCommandCenterChapter,
} from "../../src/pages/CommandCenter/model.ts";

test("system navigation only exposes workbench routes", () => {
  assert.deepEqual(SYSTEM_NAV_ROUTES, ["command-center", "export", "settings"]);
});

test("selectCommandCenterChapter prefers explicit chapter then active drafting work", () => {
  const chapters = [
    { id: "completed-1", chapterIndex: 1, status: "completed" },
    { id: "planned-2", chapterIndex: 2, status: "planned" },
    { id: "drafting-3", chapterIndex: 3, status: "drafting" },
    { id: "revising-4", chapterIndex: 4, status: "revising" },
  ];

  assert.equal(selectCommandCenterChapter(chapters, "planned-2")?.id, "planned-2");
  assert.equal(selectCommandCenterChapter(chapters)?.id, "drafting-3");
});

test("selectCommandCenterChapter falls back to planned then non-archived chapter", () => {
  const plannedOnly = [
    { id: "planned-2", chapterIndex: 2, status: "planned" },
    { id: "archived-3", chapterIndex: 3, status: "archived" },
  ];
  const completedOnly = [
    { id: "archived-1", chapterIndex: 1, status: "archived" },
    { id: "completed-2", chapterIndex: 2, status: "completed" },
  ];

  assert.equal(selectCommandCenterChapter(plannedOnly)?.id, "planned-2");
  assert.equal(selectCommandCenterChapter(completedOnly)?.id, "completed-2");
  assert.equal(selectCommandCenterChapter([]), null);
});
