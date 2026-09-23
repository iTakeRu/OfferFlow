import { useEffect, useState } from "react";
import {
  getSettings, saveSettings, listOllamaModels, checkOllama, exportBackup, importBackup,
  getProtectSettings, saveProtectSettings, syncMirrorNow, getMirrorStatus, openTrashDir,
} from "../api";
import { open, message, confirm as askConfirm } from "@tauri-apps/plugin-dialog";
import type { ExtractSettings, OllamaStatus, ProtectSettings, MirrorReport } from "../types";
import { formatSize } from "../format";
import { useI18n } from "../i18n";
import { useModalFocus } from "../useModalFocus";

interface Props {
  onClose: () => void;
}

const field = "w-full rounded-md border border-neutral-300 px-2.5 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-navy bg-white";
const label = "block text-xs font-medium text-neutral-500 mb-1";
const section = "font-serif text-base font-black tracking-wide mb-3 pb-1.5 border-b-2 border-ink";

const DEFAULT_SETTINGS: ExtractSettings = {
  fetchMode: "http",
  cookie: "",
  cookieDomain: "",
  browserPath: "",
  provider: "none",
  ollamaUrl: "http://localhost:11434",
  ollamaModel: "qwen2.5:7b",
  openaiUrl: "https://api.openai.com/v1",
  openaiKey: "",
  openaiModel: "gpt-4o-mini",
  proxy: "",
};

const DEFAULT_PROTECT: ProtectSettings = {
  trashEnabled: true,
  trashDays: 30,
  mirrorEnabled: false,
  mirrorDir: "",
};

export default function SettingsPanel({ onClose }: Props) {
  const { t } = useI18n();
  const modal = useModalFocus(onClose);
  const [s, setS] = useState<ExtractSettings>(DEFAULT_SETTINGS);
  const [msg, setMsg] = useState("");
  const [saving, setSaving] = useState(false);
  const [loadingModels, setLoadingModels] = useState(false);
  const [availableModels, setAvailableModels] = useState<string[]>([]);
  const [backingUp, setBackingUp] = useState(false);
  const [ollamaStatus, setOllamaStatus] = useState<OllamaStatus | null>(null);
  const [protect, setProtect] = useState<ProtectSettings>(DEFAULT_PROTECT);
  const [mirrorStatus, setMirrorStatus] = useState<MirrorReport | null>(null);
  const [syncing, setSyncing] = useState(false);

  useEffect(() => {
    getSettings().then(setS).catch((e) => setMsg(String(e)));
    getProtectSettings().then(setProtect).catch((e) => setMsg(String(e)));
    getMirrorStatus().then(setMirrorStatus).catch(() => setMirrorStatus(null));
  }, []);

  const setP = <K extends keyof ProtectSettings>(key: K, value: ProtectSettings[K]) =>
    setProtect((prev) => ({ ...prev, [key]: value }));

  useEffect(() => {
    if (s.provider === "ollama") {
      setOllamaStatus(null);
      checkOllama().then(setOllamaStatus).catch(() => setOllamaStatus(null));
    }
  }, [s.provider]);

  const set = <K extends keyof ExtractSettings>(key: K, value: ExtractSettings[K]) =>
    setS((prev) => ({ ...prev, [key]: value }));

  const handleSave = async () => {
    setSaving(true);
    setMsg("");
    try {
      await saveSettings(s);
      await saveProtectSettings(protect);
      setMsg(t("已保存"));
      setTimeout(onClose, 400);
    } catch (e) {
      setMsg(String(e));
    } finally {
      setSaving(false);
    }
  };

  const handleCheckOllama = async () => {
    setOllamaStatus(null);
    try {
      setOllamaStatus(await checkOllama());
    } catch {
      setOllamaStatus(null);
    }
  };

  const handleListModels = async () => {
    setLoadingModels(true);
    setMsg("");
    try {
      const models = await listOllamaModels();
      if (models.length === 0) {
        setMsg(t("Ollama 未返回模型，请检查服务是否启动，或手动输入模型名"));
      } else {
        setAvailableModels(models);
        setMsg(t("已获取 {n} 个模型，可从下拉选择或手动输入", { n: models.length }));
      }
    } catch (e) {
      setMsg(String(e));
    } finally {
      setLoadingModels(false);
    }
  };

  const handleExportBackup = async () => {
    const target = await open({ directory: true, multiple: false, title: t("选择备份保存位置") });
    if (typeof target !== "string") return;
    setBackingUp(true); setMsg("");
    try {
      const path = await exportBackup(target);
      await message(t("备份已创建：") + path, { title: t("备份完成"), kind: "info" });
    } catch (e) { setMsg(String(e)); }
    finally { setBackingUp(false); }
  };

  const handlePickMirror = async () => {
    const dir = await open({ directory: true, multiple: false, title: t("选择备份位置") });
    if (typeof dir === "string") setP("mirrorDir", dir);
  };

  // 「立即同步」先保存当前的数据保护设置，保证同步用的是界面上看到的位置
  const handleSyncNow = async () => {
    setSyncing(true); setMsg("");
    try {
      await saveProtectSettings(protect);
      setMirrorStatus(await syncMirrorNow());
    } catch (e) {
      setMsg(String(e));
      getMirrorStatus().then(setMirrorStatus).catch(() => {});
    } finally { setSyncing(false); }
  };

  const handleImportBackup = async () => {
    const source = await open({ directory: true, multiple: false, title: t("选择 OfferFlow 备份目录") });
    if (typeof source !== "string") return;
    const ok = await askConfirm(t("恢复会在重启后替换当前业务数据，当前数据会保留为 .bak，确认继续？"), { title: t("恢复确认"), kind: "warning" });
    if (!ok) return;
    setBackingUp(true); setMsg("");
    try { setMsg(await importBackup(source)); }
    catch (e) { setMsg(String(e)); }
    finally { setBackingUp(false); }
  };

  return (
    <div className="fixed inset-0 bg-ink/45 flex items-center justify-center z-30 p-4">
      <div ref={modal.ref} onKeyDown={modal.onKeyDown} role="dialog" aria-modal="true" aria-labelledby="settings-modal-title" tabIndex={-1} className="bg-white rounded-2xl shadow-xl border-2 border-ink w-full max-w-xl max-h-[88vh] flex flex-col">
        <div className="flex items-center justify-between px-6 pt-5 pb-3 border-b border-neutral-200">
          <h2 id="settings-modal-title" className="font-serif text-xl font-black tracking-wide">{t("系统设置")}</h2>
          <button onClick={onClose} aria-label={t("关闭")} className="text-neutral-400 hover:text-neutral-600 text-xl leading-none">&times;</button>
        </div>

        <div className="flex-1 overflow-y-auto px-6 py-4 space-y-5">
          {msg && <div className="text-sm text-neutral-700 bg-neutral-100 rounded-md px-3 py-2">{msg}</div>}

          {/* 抓取 */}
          <div>
            <div className={section}>{t("JD 快照抓取")}</div>
            <p className="text-xs text-neutral-500 mb-2">
              {t("粘贴岗位链接时按下面的方式访问页面并存档。打不开或需要登录的页面，可以在浏览器里「另存为」网页后导入文件，或直接粘贴文本。")}
            </p>
            <div className="space-y-2">
              <div>
                <div className={label}>{t("抓取方式")}</div>
                <div className="flex gap-3 text-sm">
                  <label className="flex items-center gap-1.5"><input type="radio" checked={s.fetchMode === "http"} onChange={() => set("fetchMode", "http")} /> {t("HTTP 请求")}</label>
                  <label className="flex items-center gap-1.5"><input type="radio" checked={s.fetchMode === "browser"} onChange={() => set("fetchMode", "browser")} /> {t("无头浏览器（渲染后抓取）")}</label>
                </div>
              </div>

              {s.fetchMode === "browser" && (
                <div>
                  <div className={label}>{t("浏览器路径（留空自动检测 Edge/Chrome）")}</div>
                  <input className={field} value={s.browserPath} onChange={(e) => set("browserPath", e.target.value)} placeholder="C:\...\msedge.exe" />
                </div>
              )}

              <div>
                <div className={label}>{t("Cookie（登录态注入，可选）")}</div>
                <textarea className={field} rows={2} value={s.cookie} onChange={(e) => set("cookie", e.target.value)} placeholder="key1=value1; key2=value2" />
                <div className={`${label} mt-2`}>{t("Cookie 仅发送到此域名及其子域名")}</div>
                <input className={field} value={s.cookieDomain} onChange={(e) => set("cookieDomain", e.target.value)} placeholder="example.com" />
              </div>

              <div>
                <div className={label}>{t("代理（可选，如 http://127.0.0.1:7890）")}</div>
                <input className={field} value={s.proxy} onChange={(e) => set("proxy", e.target.value)} placeholder="http://127.0.0.1:7890" />
              </div>
            </div>
          </div>

          {/* 模型 */}
          <div>
            <div className={section}>{t("AI 信息提取（可选）")}</div>
            <p className="text-xs text-neutral-500 mb-2">
              {t("抓取快照不会自动调用 AI。页面带有结构化岗位数据时会直接识别；识别不到时，启用后会出现「用 AI 提取」按钮，由你决定是否使用。")}
            </p>
            <div className="space-y-2">
              <div>
                <div className={label}>{t("提取方式")}</div>
                <select className={field} value={s.provider} onChange={(e) => set("provider", e.target.value)}>
                  <option value="none">{t("不启用（保存快照，手动填写）")}</option>
                  <option value="ollama">{t("Ollama（本地模型，数据不出本机）")}</option>
                  <option value="openai">{t("API 调用（OpenAI 兼容接口）")}</option>
                </select>
              </div>

              {s.provider === "ollama" && (
                <>
                  <div className="rounded-md border border-neutral-200 bg-neutral-50 px-3 py-2 space-y-1">
                    <div className="flex items-center justify-between">
                      <span className="text-xs font-medium text-neutral-600">{t("Ollama 服务状态")}</span>
                      <button type="button" onClick={handleCheckOllama} className="text-xs text-neutral-500 hover:text-neutral-700 underline">{t("重新检测")}</button>
                    </div>
                    {ollamaStatus == null ? (
                      <div className="text-xs text-neutral-400">{t("检测中…")}</div>
                    ) : ollamaStatus.running ? (
                      <div className="text-xs text-green-700">{t("服务运行中，可直接获取模型")}</div>
                    ) : ollamaStatus.installed ? (
                      <div className="text-xs text-amber-700">{t("已安装但未启动，请运行 ollama serve")}</div>
                    ) : (
                      <div className="text-xs text-red-700">
                        {t("未检测到 Ollama，请先安装：")}
                        <a href="https://ollama.com/download" target="_blank" rel="noreferrer" className="underline">ollama.com/download</a>
                      </div>
                    )}
                  </div>
                  <div>
                    <div className={label}>{t("Ollama 地址")}</div>
                    <input className={field} value={s.ollamaUrl} onChange={(e) => set("ollamaUrl", e.target.value)} />
                  </div>
                  <div>
                    <div className={label}>{t("模型")}</div>
                    <div className="flex gap-2">
                      <input className={field} list="ollama-model-list" value={s.ollamaModel} onChange={(e) => set("ollamaModel", e.target.value)} />
                      <button onClick={handleListModels} disabled={loadingModels} className="px-3 py-1.5 text-sm rounded-md border border-neutral-300 hover:bg-neutral-50 disabled:opacity-50 whitespace-nowrap">
                        {loadingModels ? t("获取中…") : t("获取模型")}
                      </button>
                    </div>
                    <datalist id="ollama-model-list">
                      {availableModels.map((m) => (
                        <option key={m} value={m} />
                      ))}
                    </datalist>
                    <p className="mt-1 text-xs text-neutral-400">{t("可手动输入模型名（适用于手动下载的模型）")}</p>
                  </div>
                </>
              )}

              {s.provider === "openai" && (
                <>
                  <p className="text-xs text-amber-700">
                    {t("使用时快照正文会发送到下面的接口，每次发送前都会先征求你的确认。")}
                  </p>
                  <div>
                    <div className={label}>{t("接口地址")}</div>
                    <input className={field} value={s.openaiUrl} onChange={(e) => set("openaiUrl", e.target.value)} placeholder="https://api.openai.com/v1" />
                    <p className="mt-1 text-xs text-neutral-400">{t("任何兼容 OpenAI Chat Completions 的服务均可，如 DeepSeek、通义千问、Moonshot 等；远程地址必须使用 HTTPS")}</p>
                  </div>
                  <div>
                    <div className={label}>API Key</div>
                    <input className={field} type="password" value={s.openaiKey} onChange={(e) => set("openaiKey", e.target.value)} />
                  </div>
                  <div>
                    <div className={label}>{t("模型名")}</div>
                    <input className={field} value={s.openaiModel} onChange={(e) => set("openaiModel", e.target.value)} placeholder="gpt-4o-mini" />
                  </div>
                </>
              )}
            </div>
          </div>

          <div>
            <div className={section}>{t("数据保护")}</div>
            <div className="space-y-4">
              <div>
                <label className="flex items-center gap-2 text-sm font-medium">
                  <input type="checkbox" checked={protect.trashEnabled} onChange={(e) => setP("trashEnabled", e.target.checked)} />
                  {t("删除的文件先放进回收站")}
                </label>
                <p className="text-xs text-neutral-500 mt-1 ml-6">
                  {t("删除投递、面试、附件或简历时，录音等文件先移到回收站，到期后自动清理。期间可以打开回收站手动找回。")}
                </p>
                <div className="flex items-center gap-2 mt-2 ml-6 text-sm">
                  <input
                    type="number" min={1} max={365}
                    aria-label={t("回收站保留天数")}
                    className="w-20 border border-neutral-300 px-2 py-1 text-sm bg-white focus:outline-none focus:ring-2 focus:ring-navy disabled:opacity-50"
                    value={protect.trashDays}
                    disabled={!protect.trashEnabled}
                    onChange={(e) => setP("trashDays", Number(e.target.value))}
                  />
                  <span className="text-neutral-600">{t("天后自动清理")}</span>
                  <button type="button" onClick={() => openTrashDir().catch((e) => setMsg(String(e)))} className="ml-auto px-3 py-1.5 text-sm border border-neutral-300 hover:bg-neutral-50">{t("打开回收站")}</button>
                </div>
              </div>

              <div>
                <label className="flex items-center gap-2 text-sm font-medium">
                  <input type="checkbox" checked={protect.mirrorEnabled} onChange={(e) => setP("mirrorEnabled", e.target.checked)} />
                  {t("每次启动时自动同步到备份位置")}
                </label>
                <p className="text-xs text-neutral-500 mt-1 ml-6">
                  {t("把新增的录音、简历、JD 快照和最新数据库同步到另一块硬盘或网盘同步文件夹。只复制新增文件，本地删除不会影响备份；数据库另按天保留最近 14 份。需要时用「从备份恢复」选择其中的 OfferFlow-Mirror 文件夹即可还原。")}
                </p>
                <div className="flex gap-2 mt-2 ml-6">
                  <input className={field} readOnly value={protect.mirrorDir} placeholder={t("尚未选择备份位置")} aria-label={t("备份位置")} />
                  <button type="button" onClick={handlePickMirror} className="px-3 py-1.5 text-sm border border-neutral-300 hover:bg-neutral-50 whitespace-nowrap">{t("选择位置")}</button>
                  <button type="button" onClick={handleSyncNow} disabled={syncing || !protect.mirrorDir} className="px-3 py-1.5 text-sm border border-neutral-300 hover:bg-neutral-50 disabled:opacity-50 whitespace-nowrap">
                    {syncing ? t("同步中…") : t("立即同步")}
                  </button>
                </div>
                {mirrorStatus && (
                  <p className={"text-xs mt-1.5 ml-6 " + (mirrorStatus.error ? "text-red-700" : "text-neutral-500")}>
                    {mirrorStatus.error
                      ? t("上次同步失败（{time}）：{error}", { time: mirrorStatus.finishedAt, error: mirrorStatus.error })
                      : t("上次同步：{time}，新增 {n} 个文件（{size}），{same} 个已存在", {
                        time: mirrorStatus.finishedAt,
                        n: mirrorStatus.copiedFiles,
                        size: formatSize(mirrorStatus.copiedBytes),
                        same: mirrorStatus.unchangedFiles,
                      })}
                  </p>
                )}
              </div>
            </div>
          </div>

          <div>
            <div className={section}>{t("备份与恢复")}</div>
            <p className="text-xs text-neutral-500 mb-2">{t("备份包含业务数据、简历、附件和 JD 快照，不包含 Cookie 与 API Key。")}</p>
            <div className="flex gap-2">
              <button onClick={handleExportBackup} disabled={backingUp} className="px-3 py-1.5 text-sm rounded-md border border-neutral-300 hover:bg-neutral-50 disabled:opacity-50">{t("导出完整备份")}</button>
              <button onClick={handleImportBackup} disabled={backingUp} className="px-3 py-1.5 text-sm rounded-md border border-neutral-300 text-red-700 hover:bg-red-50 disabled:opacity-50">{t("从备份恢复")}</button>
            </div>
          </div>
        </div>

        <div className="flex justify-end gap-2 px-6 py-4 border-t border-neutral-200">
          <button onClick={onClose} className="px-3 py-1.5 text-sm rounded-md border border-neutral-300 hover:bg-neutral-50">{t("取消")}</button>
          <button onClick={handleSave} disabled={saving} className="px-3 py-1.5 text-sm rounded-md bg-navy text-paper hover:bg-navy-deep disabled:opacity-50">
            {saving ? t("保存中…") : t("保存")}
          </button>
        </div>
      </div>
    </div>
  );
}
