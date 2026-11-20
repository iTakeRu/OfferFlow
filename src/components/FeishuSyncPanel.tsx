import { useEffect, useState } from "react";
import { confirm } from "@tauri-apps/plugin-dialog";
import { bindFeishuTable, createFeishuTable, getFeishuSettings, getFeishuStatus, listFeishuTables, openFeishuTable, saveFeishuSettings, syncFeishuNow } from "../api";
import type { FeishuReport, FeishuSettings, FeishuTables } from "../types";
import { useI18n } from "../i18n";
import { intervalParts, intervalSeconds } from "../feishuInterval";

const button = "px-3 py-1.5 text-sm rounded-md border border-neutral-300 hover:bg-neutral-50 disabled:opacity-50";
const primary = "px-3 py-1.5 text-sm rounded-md bg-navy text-on-navy hover:bg-navy-deep disabled:opacity-50";
const field = "w-full rounded-md border border-neutral-300 px-3 py-2 text-sm bg-white focus:outline-none focus:ring-2 focus:ring-navy disabled:opacity-50";

export default function FeishuSyncPanel() {
  const { t, lang } = useI18n();
  const [settings, setSettings] = useState<FeishuSettings | null>(null);
  const [report, setReport] = useState<FeishuReport | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [cliPath, setCliPath] = useState("");
  const [profile, setProfile] = useState("");
  const [changing, setChanging] = useState(false);
  const [mode, setMode] = useState<"create" | "bind">("create");
  const [name, setName] = useState("OfferFlow 投递表");
  const [url, setUrl] = useState("");
  const [tables, setTables] = useState<FeishuTables | null>(null);
  const [tableId, setTableId] = useState("");
  const [createFailed, setCreateFailed] = useState(false);
  const [intervalValue, setIntervalValue] = useState("15");
  const [intervalUnit, setIntervalUnit] = useState(1);

  const accept = (value: FeishuSettings) => {
    setSettings(value); setCliPath(value.cliPath); setProfile(value.profile);
    const interval = intervalParts(value.syncIntervalSeconds);
    setIntervalValue(interval.value); setIntervalUnit(interval.unit);
  };
  useEffect(() => {
    let active = true;
    let timer: ReturnType<typeof setTimeout>;
    getFeishuSettings().then((value) => { if (active) accept(value); }).catch((e) => { if (active) setError(String(e)); });
    const refresh = async () => {
      try { const value = await getFeishuStatus(); if (active) setReport(value); }
      catch (e) { if (active) setError(String(e)); }
      if (active) timer = setTimeout(refresh, 2000);
    };
    void refresh();
    return () => { active = false; clearTimeout(timer); };
  }, []);

  const connected = Boolean(settings?.baseToken);
  const running = busy || Boolean(report?.running);
  const formOpen = !connected || changing;
  const seconds = intervalSeconds(intervalValue, intervalUnit);
  const savedInterval = intervalParts(settings?.syncIntervalSeconds ?? 15);
  const unitLabel = (unit: number) => unit === 3600 ? t("小时") : unit === 60 ? t("分钟") : t("秒");
  const act = async (work: () => Promise<void>) => {
    setBusy(true); setError("");
    try { await work(); } catch (e) { setError(String(e)); }
    finally { setBusy(false); }
  };
  const finish = (value: FeishuSettings) => {
    accept(value); setReport(null); setChanging(false); setTables(null); setCreateFailed(false);
  };
  const create = () => act(async () => {
    if (createFailed && !await confirm(t("请先查看飞书云盘。已有投递表可直接绑定，确定再创建一张？"), { title: t("重新创建投递表") })) return;
    try { finish(await createFeishuTable(name.trim(), cliPath.trim(), profile.trim(), createFailed)); }
    catch (e) { setCreateFailed(true); throw e; }
  });
  const lookup = () => act(async () => {
    setTables(null); setTableId("");
    const result = await listFeishuTables(url.trim(), cliPath.trim(), profile.trim());
    setTables(result); setTableId(result.selectedTableId ?? "");
  });
  const bind = () => act(async () => finish(await bindFeishuTable(url.trim(), tableId, cliPath.trim(), profile.trim())));
  const sync = () => act(async () => {
    try { setReport(await syncFeishuNow()); }
    finally { setReport(await getFeishuStatus()); }
  });
  const toggle = (autoSync: boolean) => act(async () => {
    if (!settings) return;
    const next = { ...settings, autoSync };
    await saveFeishuSettings(next); setSettings(next);
  });

  return <section aria-labelledby="feishu-sync-title">
    <h3 id="feishu-sync-title" className="font-serif text-base font-semibold tracking-wide mb-3 pb-1.5 border-b-2 border-rule">{t("飞书同步")}</h3>
    <p className="text-sm text-neutral-600 mb-3">{t("将投递记录更新到飞书多维表格。飞书中的修改会被本地值覆盖，本地删除不会删除飞书行。")}</p>
    {connected && <>
      <div className="flex flex-wrap gap-2">
        <button type="button" className={primary} disabled={running || changing} onClick={sync}>{running ? t("处理中…") : t("立即同步到飞书")}</button>
        <button type="button" className={button} onClick={() => openFeishuTable().catch((e) => setError(String(e)))}>{t("打开飞书表格")}</button>
        <button type="button" className={button} disabled={running} onClick={() => { setChanging(!changing); setError(""); }}>{changing ? t("取消更换") : t("更换表格")}</button>
      </div>
      <label className="flex items-center gap-2 text-sm font-medium mt-4">
        <input type="checkbox" role="switch" checked={settings?.autoSync ?? false} disabled={running || changing} onChange={(e) => toggle(e.target.checked)} />
        {t("自动同步到飞书")}
      </label>
      <fieldset disabled={running || changing} className="flex flex-wrap items-center gap-2 mt-3 text-sm">
        <label htmlFor="feishu-sync-interval">{t("同步间隔")}</label>
        <input id="feishu-sync-interval" type="number" min={1} max={86400 / intervalUnit} step={1}
          className={field.replace("w-full", "w-24")} value={intervalValue} aria-invalid={seconds === null}
          onChange={(e) => setIntervalValue(e.target.value)} />
        <select aria-label={t("同步间隔单位")} className={field.replace("w-full", "w-auto")} value={intervalUnit}
          onChange={(e) => setIntervalUnit(Number(e.target.value))}>
          {[1, 60, 3600].map((unit) => <option key={unit} value={unit}>{unitLabel(unit)}</option>)}
        </select>
        <button type="button" className={button} disabled={seconds === null || seconds === settings?.syncIntervalSeconds}
          onClick={() => act(async () => {
            if (!settings || seconds === null) return;
            const next = { ...settings, syncIntervalSeconds: seconds };
            await saveFeishuSettings(next); setSettings(next);
          })}>{t("保存间隔")}</button>
      </fieldset>
      {seconds === null && <p className="text-xs text-red-700 mt-1">{t("请输入整数，间隔范围为 1 秒至 24 小时。")}</p>}
      <p className="text-xs text-neutral-500 mt-1">{t("已保存：每 {value} {unit}检测变化。开关即时保存，失败后至少 5 分钟再试。", { value: savedInterval.value, unit: unitLabel(savedInterval.unit) })}</p>
    </>}
    {formOpen && <fieldset disabled={running || !settings} className="mt-3 space-y-3">
      <legend className="sr-only">{t("连接飞书表格")}</legend>
      <div className="flex gap-4 text-sm">
        <label className="flex items-center gap-2"><input type="radio" name="feishu-connect-mode" checked={mode === "create"} onChange={() => setMode("create")} />{t("新建投递表")}</label>
        <label className="flex items-center gap-2"><input type="radio" name="feishu-connect-mode" checked={mode === "bind"} onChange={() => setMode("bind")} />{t("绑定已有表格")}</label>
      </div>
      {mode === "create" ? <>
        <label className="block text-sm">{t("表格名称")}<input className={`${field} mt-1`} value={name} maxLength={100} onChange={(e) => setName(e.target.value)} /></label>
        <button type="button" className={primary} disabled={!name.trim()} onClick={create}>{busy ? t("处理中…") : createFailed ? t("重新创建投递表") : t("创建并连接")}</button>
      </> : <>
        <label className="block text-sm">{t("飞书表格链接")}<input type="url" className={`${field} mt-1`} value={url} placeholder="https://…feishu.cn/base/…" onChange={(e) => { setUrl(e.target.value); setTables(null); setTableId(""); }} /></label>
        <button type="button" className={button} disabled={!url.trim()} onClick={lookup}>{busy ? t("处理中…") : t("读取数据表")}</button>
        {tables && <>
          <label className="block text-sm">{t("数据表")}<select className={`${field} mt-1`} value={tableId} onChange={(e) => setTableId(e.target.value)}>
            <option value="" disabled>{t("选择数据表")}</option>
            {tables.tables.map((table) => <option key={table.id} value={table.id}>{table.name}</option>)}
          </select></label>
          <p className="text-xs text-neutral-500">{t("绑定时补齐投递字段，保留表内现有内容。")}</p>
          <button type="button" className={primary} disabled={!tableId} onClick={bind}>{busy ? t("处理中…") : t("绑定表格")}</button>
        </>}
      </>}
    </fieldset>}
    <div role="status" aria-live="polite" className="text-sm mt-3 text-neutral-600">
      {report?.running ? t("正在同步投递记录…") : report?.finishedAt && !report.error ? t("{time} · 新增 {created}，更新 {updated}，未变化 {unchanged}", {
        time: new Date(report.finishedAt).toLocaleString(lang === "en" ? "en-US" : "zh-CN"),
        created: report.created, updated: report.updated, unchanged: report.unchanged,
      }) : connected && !report?.error ? t("已连接投递表") : null}
      {(error || report?.error) && <p className="text-red-700 break-words">{error || report?.error}</p>}
    </div>
    <details className="mt-3 text-sm">
      <summary className="cursor-pointer text-neutral-600">{t("CLI 设置")}</summary>
      <fieldset disabled={running} className="mt-3 space-y-3">
        <label className="block">{t("飞书 CLI 路径（留空自动查找）")}
          <input className={`${field} mt-1`} value={cliPath} onChange={(e) => { setCliPath(e.target.value); setTables(null); setTableId(""); }} />
        </label>
        <label className="block">{t("Bridge 配置名称（留空使用 CLI 默认配置）")}
          <input className={`${field} mt-1`} value={profile} onChange={(e) => { setProfile(e.target.value); setTables(null); setTableId(""); }} />
        </label>
        {connected && !changing && <button type="button" className={button} onClick={() => act(async () => {
          if (!settings) return;
          const next = { ...settings, cliPath: cliPath.trim(), profile: profile.trim() };
          await saveFeishuSettings(next); accept(next);
        })}>{t("保存连接设置")}</button>}
      </fieldset>
    </details>
  </section>;
}
