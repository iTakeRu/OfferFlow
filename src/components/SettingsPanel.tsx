import { useEffect, useState } from "react";
import { getSettings, saveSettings, listOllamaModels } from "../api";
import type { ExtractSettings } from "../types";
import { useI18n } from "../i18n";

interface Props {
  onClose: () => void;
}

const field = "w-full rounded-md border border-neutral-300 px-2.5 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-neutral-400 bg-white";
const label = "block text-xs font-medium text-neutral-500 mb-1";
const section = "text-sm font-semibold text-neutral-700 mb-2";

const DEFAULT_SETTINGS: ExtractSettings = {
  fetchMode: "http",
  cookie: "",
  browserPath: "",
  provider: "none",
  ollamaUrl: "http://localhost:11434",
  ollamaModel: "qwen2.5:7b",
  openaiUrl: "https://api.openai.com/v1",
  openaiKey: "",
  openaiModel: "gpt-4o-mini",
  proxy: "",
  mirror: "",
};

export default function SettingsPanel({ onClose }: Props) {
  const { t } = useI18n();
  const [s, setS] = useState<ExtractSettings>(DEFAULT_SETTINGS);
  const [msg, setMsg] = useState("");
  const [saving, setSaving] = useState(false);
  const [loadingModels, setLoadingModels] = useState(false);

  useEffect(() => {
    getSettings().then(setS).catch((e) => setMsg(String(e)));
  }, []);

  const set = <K extends keyof ExtractSettings>(key: K, value: ExtractSettings[K]) =>
    setS((prev) => ({ ...prev, [key]: value }));

  const handleSave = async () => {
    setSaving(true);
    setMsg("");
    try {
      await saveSettings(s);
      setMsg(t("已保存"));
      setTimeout(onClose, 400);
    } catch (e) {
      setMsg(String(e));
    } finally {
      setSaving(false);
    }
  };

  const handleListModels = async () => {
    setLoadingModels(true);
    setMsg("");
    try {
      const models = await listOllamaModels();
      if (models.length === 0) {
        setMsg(t("Ollama 未返回模型，请检查服务是否启动"));
      } else {
        set("ollamaModel", models[0]);
        setMsg(t("已获取 {n} 个模型，已选择首个", { n: models.length }));
      }
    } catch (e) {
      setMsg(String(e));
    } finally {
      setLoadingModels(false);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/40 flex items-center justify-center z-30 p-4">
      <div className="bg-white rounded-xl shadow-xl w-full max-w-xl max-h-[88vh] flex flex-col">
        <div className="flex items-center justify-between px-6 pt-5 pb-3 border-b border-neutral-200">
          <h2 className="text-base font-semibold">{t("设置 · 抓取与信息提取")}</h2>
          <button onClick={onClose} className="text-neutral-400 hover:text-neutral-600 text-xl leading-none">&times;</button>
        </div>

        <div className="flex-1 overflow-y-auto px-6 py-4 space-y-5">
          {msg && <div className="text-sm text-neutral-700 bg-neutral-100 rounded-md px-3 py-2">{msg}</div>}

          {/* 抓取 */}
          <div>
            <div className={section}>{t("岗位 URL 抓取")}</div>
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
              </div>

              <div>
                <div className={label}>{t("代理（可选，如 http://127.0.0.1:7890）")}</div>
                <input className={field} value={s.proxy} onChange={(e) => set("proxy", e.target.value)} placeholder="http://127.0.0.1:7890" />
              </div>
            </div>
          </div>

          {/* 模型 */}
          <div>
            <div className={section}>{t("信息提取模型")}</div>
            <div className="space-y-2">
              <div>
                <div className={label}>{t("模型提供商")}</div>
                <select className={field} value={s.provider} onChange={(e) => set("provider", e.target.value)}>
                  <option value="none">{t("不启用（纯手动录入）")}</option>
                  <option value="ollama">{t("Ollama（本地离线 / 服务）")}</option>
                  <option value="openai">{t("OpenAI 兼容接口")}</option>
                </select>
              </div>

              {s.provider === "ollama" && (
                <>
                  <div>
                    <div className={label}>{t("Ollama 地址")}</div>
                    <input className={field} value={s.ollamaUrl} onChange={(e) => set("ollamaUrl", e.target.value)} />
                  </div>
                  <div>
                    <div className={label}>{t("模型")}</div>
                    <div className="flex gap-2">
                      <input className={field} value={s.ollamaModel} onChange={(e) => set("ollamaModel", e.target.value)} />
                      <button onClick={handleListModels} disabled={loadingModels} className="px-3 py-1.5 text-sm rounded-md border border-neutral-300 hover:bg-neutral-50 disabled:opacity-50 whitespace-nowrap">
                        {loadingModels ? t("获取中…") : t("获取模型")}
                      </button>
                    </div>
                  </div>
                  <div>
                    <div className={label}>{t("模型下载镜像源（可选，供模型下载加速）")}</div>
                    <input className={field} value={s.mirror} onChange={(e) => set("mirror", e.target.value)} placeholder="https://ollama.com/library" />
                  </div>
                </>
              )}

              {s.provider === "openai" && (
                <>
                  <div>
                    <div className={label}>{t("接口地址")}</div>
                    <input className={field} value={s.openaiUrl} onChange={(e) => set("openaiUrl", e.target.value)} />
                  </div>
                  <div>
                    <div className={label}>API Key</div>
                    <input className={field} type="password" value={s.openaiKey} onChange={(e) => set("openaiKey", e.target.value)} />
                  </div>
                  <div>
                    <div className={label}>{t("模型名")}</div>
                    <input className={field} value={s.openaiModel} onChange={(e) => set("openaiModel", e.target.value)} />
                  </div>
                </>
              )}
            </div>
          </div>
        </div>

        <div className="flex justify-end gap-2 px-6 py-4 border-t border-neutral-200">
          <button onClick={onClose} className="px-3 py-1.5 text-sm rounded-md border border-neutral-300 hover:bg-neutral-50">{t("取消")}</button>
          <button onClick={handleSave} disabled={saving} className="px-3 py-1.5 text-sm rounded-md bg-neutral-900 text-white hover:bg-neutral-800 disabled:opacity-50">
            {saving ? t("保存中…") : t("保存")}
          </button>
        </div>
      </div>
    </div>
  );
}