import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useI18n } from "../i18n";

interface UpdateCheck {
  status: "available" | "current" | "unavailable" | "rateLimited" | "networkError" | "invalidResponse";
  latestVersion: string | null;
}

const STATUS_TEXT = {
  current: "当前已是最新正式版",
  unavailable: "暂时无法获取版本信息，仓库可能尚未公开或没有正式版。可在浏览器中查看发布页面。",
  rateLimited: "GitHub 请求暂时受限，请稍后重试，或打开发布页面。",
  networkError: "无法连接 GitHub，请检查网络后重试。",
  invalidResponse: "收到的版本信息无效，请打开发布页面查看。",
};

export default function AboutPanel() {
  const { t } = useI18n();
  const [version, setVersion] = useState<string | null>(null);
  const [checking, setChecking] = useState(false);
  const [result, setResult] = useState<UpdateCheck | null>(null);
  const [error, setError] = useState("");

  useEffect(() => { getVersion().then(setVersion).catch(() => setVersion(null)); }, []);

  const check = async () => {
    setChecking(true);
    setResult(null);
    setError("");
    try {
      setResult(await invoke<UpdateCheck>("check_for_updates"));
    } catch {
      setError("检查更新失败，请稍后重试。");
    } finally {
      setChecking(false);
    }
  };

  const openReleases = async () => {
    setError("");
    try { await openUrl("https://github.com/iTakeRu/OfferFlow/releases/latest"); }
    catch { setError("无法打开浏览器，请手动访问 GitHub 发布页面。"); }
  };

  return (
    <section aria-labelledby="about-offerflow-title">
      <h3 id="about-offerflow-title" className="font-serif text-base font-black tracking-wide mb-3 pb-1.5 border-b-2 border-rule">{t("关于 OfferFlow")}</h3>
      <p className="text-sm font-medium">OfferFlow · {version ? `v${version}` : t("未知")}</p>
      <p className="text-xs text-neutral-500 mt-1 mb-3">{t("手动检查 GitHub 最新正式版，下载后自行安装。")}</p>
      <div className="flex flex-wrap gap-2">
        <button type="button" onClick={check} disabled={checking} className="px-3 py-1.5 text-sm rounded-md border border-neutral-300 hover:bg-neutral-50 disabled:opacity-50">
          {checking ? t("检查中…") : t("检查更新")}
        </button>
        <button type="button" onClick={openReleases} className="px-3 py-1.5 text-sm rounded-md border border-neutral-300 hover:bg-neutral-50">
          {result?.status === "available" ? t("下载新版本") : t("查看发布页面")}
        </button>
      </div>
      <div role="status" aria-live="polite" className="text-xs mt-2 text-neutral-600">
        {result && (result.status === "available"
          ? t("发现新版本 v{version}", { version: result.latestVersion ?? "" })
          : t(STATUS_TEXT[result.status]))}
        {error && <p className="text-red-700">{t(error)}</p>}
      </div>
    </section>
  );
}
