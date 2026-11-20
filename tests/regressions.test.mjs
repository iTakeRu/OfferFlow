import assert from "node:assert/strict";
import test from "node:test";
import { channelFromSource, channelOptions, createApplicationSaver } from "../src/applicationSave.ts";
import { interviewDateTimeValue } from "../src/format.ts";
import { CATEGORIES, categoryOptions } from "../src/types.ts";
import { intervalParts, intervalSeconds } from "../src/feishuInterval.ts";

test("Feishu intervals round-trip units and reject invalid input", () => {
  for (const seconds of [1, 15, 90, 300, 3600, 86400]) {
    const { value, unit } = intervalParts(seconds);
    assert.equal(intervalSeconds(value, unit), seconds);
  }
  assert.deepEqual(intervalParts(300), { value: "5", unit: 60 });
  assert.equal(intervalSeconds("2", 3600), 7200);
  for (const value of ["", " ", "0", "-1", "1.5", "NaN", "Infinity", "86401"]) {
    assert.equal(intervalSeconds(value, 1), null);
  }
  assert.equal(intervalSeconds("25", 3600), null);
  assert.equal(intervalSeconds("1", 2), null);
});

test("refresh failure is reported; retry updates the created application", async () => {
  let created = 0;
  const updates = [];
  const save = createApplicationSaver(null,
    async () => ({ id: ++created }),
    async (id, payload) => { updates.push({ id, payload }); },
  );
  await assert.rejects(save("original", async () => { throw new Error("refresh failed"); }), /refresh failed/);
  await save("corrected", async () => {});
  assert.equal(created, 1);
  assert.deepEqual(updates, [{ id: 1, payload: "corrected" }]);
});

test("save stays pending until refresh finishes and ignores duplicate submissions", async () => {
  let finishRefresh;
  let creates = 0;
  let updates = 0;
  let completed = false;
  const refreshing = new Promise((resolve) => { finishRefresh = resolve; });
  const save = createApplicationSaver(null,
    async () => ({ id: ++creates }),
    async () => { updates++; },
  );
  const first = save({}, () => refreshing).then(() => { completed = true; });
  await Promise.resolve();
  await save({}, () => {});
  assert.equal(completed, false);
  assert.equal(creates, 1);
  assert.equal(updates, 0);
  finishRefresh();
  await first;
  assert.equal(completed, true);
});

test("a failed create can be retried and an existing application is only updated", async () => {
  let calls = 0;
  const save = createApplicationSaver(null, async () => {
    if (++calls === 1) throw new Error("write failed");
    return { id: 7 };
  }, async () => {});
  await assert.rejects(save({}, () => {}), /write failed/);
  await save({}, () => {});
  assert.equal(calls, 2);
  const updates = [];
  await createApplicationSaver(42, async () => { assert.fail("must not create"); },
    async (id) => { updates.push(id); })({}, () => {});
  assert.deepEqual(updates, [42]);
});

test("interview input displays minute precision in supported stored formats", () => {
  assert.equal(interviewDateTimeValue("2026-10-15T14:35:27"), "2026-10-15T14:35");
  assert.equal(interviewDateTimeValue("2026-10-15 14:35:27"), "2026-10-15T14:35");
  assert.equal(interviewDateTimeValue("2026-10-15T14:35"), "2026-10-15T14:35");
  assert.equal(interviewDateTimeValue("2026-10-15"), "2026-10-15T00:00");
  assert.equal(interviewDateTimeValue(""), "");
});

test("category options keep defaults first and add used custom categories once", () => {
  const jobs = [{ category: "营销类" }, { category: null }, { category: " 技术类 " }, { category: "营销类" }, { category: "  " }, { category: "游戏策划" }];
  assert.deepEqual(categoryOptions(jobs), [...CATEGORIES, "营销类", "游戏策划"]);
  assert.deepEqual(categoryOptions([]), CATEGORIES);
});
test("channel suggestions keep custom history and infer only known source hosts", () => {
  assert.deepEqual(channelOptions([{ channel: " 微信群 " }, { channel: "Boss直聘" }, { channel: null }]),
    ["BOSS直聘", "官网", "内推", "微信群"]);
  assert.equal(channelFromSource("https://www.zhipin.com/job_detail/123.html"), "BOSS直聘");
  assert.equal(channelFromSource("https://app.mokahr.com/jobs/123"), "官网");
  assert.equal(channelFromSource("https://zhipin.com.example.org/"), "");
  assert.equal(channelFromSource(null), "");
});
