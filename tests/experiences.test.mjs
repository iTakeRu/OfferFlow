import assert from "node:assert/strict";
import test from "node:test";
import { changeKind, compareExperiences, createExperience, duplicateVersion, experienceSubtitle, experienceText, isPresetField, matchesExperience, copyText } from "../src/experiences.ts";

test("versions share fields and differ only in description; text keeps newlines", () => {
  const e = createExperience("work", "通用版");
  const original = e.versions[0];
  e.fields[0].value = "Example Company";
  original.description = "First achievement\nSecond achievement";
  const copy = duplicateVersion(original, "游戏版");
  copy.description = "Game research";
  assert.equal(original.description, "First achievement\nSecond achievement");
  assert.notEqual(copy.id, original.id);
  assert.equal(experienceText(e, original), "公司名称：Example Company\nFirst achievement\nSecond achievement");
  assert.equal(experienceText(e, copy), "公司名称：Example Company\nGame research");
  e.versions.push(copy);
  assert.equal(matchesExperience(e, "GAME research"), true);
  assert.equal(matchesExperience(e, "example company"), true);
  assert.equal(matchesExperience(e, "missing"), false);
});

test("kind templates, locked preset labels and kind switching", () => {
  const e = createExperience("education", "通用版");
  assert.ok(e.fields.some(f => f.label === "学历 / 学位"));
  assert.ok(isPresetField("education", "专业"));
  assert.ok(!isPresetField("text", "专业"));
  assert.equal(experienceText(createExperience("text", "通用版"), createExperience("text", "通用版").versions[0]), "");
  e.fields.find(f => f.label === "学校名称").value = "Example University";
  e.fields.push({ label: "GPA", value: "3.8" });
  const labels = changeKind(e.fields, "education", "work").map(f => f.label);
  assert.ok(labels.includes("公司名称") && labels.includes("学校名称") && labels.includes("GPA"));
  assert.ok(!labels.includes("专业"));
  assert.deepEqual(labels.slice(0, 6), ["公司名称", "职位名称", "部门", "开始时间", "结束时间", "工作地点"]);
  assert.equal(labels.filter(l => l === "开始时间").length, 1);
});

test("subtitle and ordering use role and start date", () => {
  const a = createExperience("work", "通用版"), b = createExperience("work", "通用版"), c = createExperience("work", "通用版");
  const set = (x, label, value) => { x.fields.find(f => f.label === label).value = value; };
  set(a, "职位名称", "Intern"); set(a, "开始时间", "2024-06"); set(a, "结束时间", "至今");
  set(b, "开始时间", "2025-01");
  assert.equal(experienceSubtitle(a), "Intern · 2024-06 – 至今");
  assert.deepEqual([c, a, b].sort(compareExperiences), [b, a, c]);
});

test("clipboard gets exact plain text; blank values are rejected", async () => {
  const writes = [];
  Object.defineProperty(globalThis.navigator, "clipboard", { configurable: true, value: { writeText: async text => { writes.push(text); } } });
  await copyText("第一行\n第二行");
  assert.deepEqual(writes, ["第一行\n第二行"]);
  await assert.rejects(copyText("  "), /没有可复制/);
});

test("clipboard fallback reports failure and restores focus", async () => {
  Object.defineProperty(globalThis.navigator, "clipboard", { configurable: true, value: { writeText: async () => { throw new Error("denied"); } } });
  let restored = false, removed = false, selected = false;
  const textarea = { style: {}, value: "", select() { selected = true; }, remove() { removed = true; } };
  globalThis.document = { activeElement: { focus() { restored = true; } }, createElement: () => textarea, body: { appendChild() {} }, execCommand: () => false };
  await assert.rejects(copyText("exact value"), /复制失败/);
  assert.equal(textarea.value, "exact value");
  assert.ok(selected && restored && removed);
  delete globalThis.document;
});
