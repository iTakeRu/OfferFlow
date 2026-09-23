import { useState } from "react";
import { open, confirm } from "@tauri-apps/plugin-dialog";
import {
  captureSnapshotFromUrl, captureSnapshotFromFile, extractJdInfo, getSettings,
} from "../api";
import { CATEGORIES, EMPTY_JOB_FIELDS } from "../types";
import type { ExtractSettings, ExtractedJob, JobDetail, JobFields, SnapshotDraft } from "../types";
import { hostOf, isLoopback } from "../net";
import { useI18n } from "../i18n";

interface Props {
  onCaptured: (draft: SnapshotDraft, fields: JobFields) => void;
  /** 用户手动触发 AI 提取后回传识别结果；由调用方决定如何与已填内容合并。 */
  onExtracted?: (fields: JobFields) => void;
  /** 给已有岗位追加快照时不需要识别岗位字段。 */
  skipExtraction?: boolean;
}

/** AI 提取的目标服务；remote 表示正文会离开本机。 */
type AiTarget = { name: string; host: string; remote: boolean };
type AiOffer = { draft: SnapshotDraft; target: AiTarget };

function aiTarget(s: ExtractSettings): AiTarget | null {
  if (s.provider === "ollama") {
    const host = hostOf(s.ollamaUrl);
    return { name: "Ollama", host, remote: !isLoopback(host) };
  }
  if (s.provider === "openai") {
    const host = hostOf(s.openaiUrl);
    return { name: host, host, remote: !isLoopback(host) };
  }
  return null;
}

type Mode = "url" | "file" | "paste";
type Status = { tone: "ok" | "warn"; text: string } | null;

const field =
  "w-full rounded-md border border-neutral-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-neutral-400 bg-white";

const text = (v: string | null | undefined) => (v ?? "").trim();
const num = (v: number | null | undefined) => (v == null ? "" : String(v));
const category = (v: string | null | undefined) =>
  CATEGORIES.includes(text(v)) ? text(v) : "";

function fromDetail(d: JobDetail | null): JobFields {
  if (!d) return EMPTY_JOB_FIELDS;
  return {
    companyName: text(d.company),
    title: text(d.title),
    department: text(d.department),
    location: text(d.location),
    category: category(d.category),
    salaryMin: num(d.salaryMin),
    salaryMax: num(d.salaryMax),
  };
}

/** AI 结果只补空缺，结构化数据里已有的字段优先。 */
function fillFrom(base: JobFields, r: ExtractedJob): JobFields {
  const pick = (current: string, next: string) => current || next;
  return {
    companyName: pick(base.companyName, text(r.company)),
    title: pick(base.title, text(r.title)),
    department: pick(base.department, text(r.department)),
    location: pick(base.location, text(r.location)),
    category: pick(base.category, category(r.category)),
    salaryMin: pick(base.salaryMin, num(r.salaryMin)),
    salaryMax: pick(base.salaryMax, num(r.salaryMax)),
  };
}

export default function SnapshotCapture({ onCaptured, onExtracted, skipExtraction = false }: Props) {
  const { t } = useI18n();
  const [mode, setMode] = useState<Mode>("url");
  const [url, setUrl] = useState("");
  const [pasteText, setPasteText] = useState("");
  const [pasteUrl, setPasteUrl] = useState("");
  const [busy, setBusy] = useState<"" | "capture" | "ai">("");
  const [status, setStatus] = useState<Status>(null);
  const [error, setError] = useState("");
  const [aiOffer, setAiOffer] = useState<AiOffer | null>(null);

  // 默认只保存快照：先看页面自带的结构化数据，识别不到就交给用户手动填写。
  // AI 提取不会自动调用，只作为一个按钮提供，由用户决定要不要把正文交给模型。
  const finish = async (draft: SnapshotDraft) => {
    const fields = skipExtraction ? EMPTY_JOB_FIELDS : fromDetail(draft.detail);
    onCaptured(draft, fields);
    if (skipExtraction) {
      setStatus({ tone: "ok", text: t("快照已获取") });
      return;
    }
    if (fields.title) {
      setStatus({ tone: "ok", text: t("已从页面结构化数据识别岗位信息，请核对") });
      return;
    }
    const settings = await getSettings().catch(() => null);
    const target = settings ? aiTarget(settings) : null;
    if (target && onExtracted) {
      setAiOffer({ draft, target });
      setStatus({ tone: "warn", text: t("快照已获取，页面里没有结构化岗位信息。可以手动填写，或使用 AI 提取。") });
    } else {
      setStatus({ tone: "warn", text: t("快照已获取。未启用 AI 提取，请手动填写岗位信息") });
    }
  };

  const runAi = async () => {
    if (!aiOffer || !onExtracted) return;
    const { draft, target } = aiOffer;
    if (target.remote) {
      const ok = await confirm(
        t("将把快照正文（约 {n} 字）发送到 {host} 进行识别。该服务可能保存所收到的内容，也可能按用量计费。确认发送？", {
          n: draft.text.length,
          host: target.host,
        }),
        { title: t("发送到外部服务"), kind: "warning", okLabel: t("发送并提取"), cancelLabel: t("取消") },
      );
      if (!ok) return;
    }
    setError("");
    setBusy("ai");
    try {
      const fields = fillFrom(EMPTY_JOB_FIELDS, await extractJdInfo(draft.text));
      onExtracted(fields);
      setAiOffer(null);
      setStatus({
        tone: fields.title ? "ok" : "warn",
        text: fields.title
          ? t("已由 AI 提取岗位信息，只填入了空着的字段，请核对")
          : t("AI 未识别出岗位名称，请手动填写"),
      });
    } catch (e) {
      // 失败时保留按钮，方便改完设置后重试
      setStatus({ tone: "warn", text: t("AI 提取失败，请手动填写：") + String(e) });
    } finally {
      setBusy("");
    }
  };

  const run = async (capture: () => Promise<SnapshotDraft | null>) => {
    setError("");
    setStatus(null);
    setAiOffer(null);
    setBusy("capture");
    let draft: SnapshotDraft | null = null;
    try {
      draft = await capture();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy("");
    }
    if (draft) await finish(draft);
  };

  const fromUrl = () => {
    if (!url.trim()) {
      setError(t("请先粘贴岗位链接"));
      return;
    }
    run(() => captureSnapshotFromUrl(url.trim()));
  };

  const fromFile = () =>
    run(async () => {
      const path = await open({
        multiple: false,
        title: t("选择保存的网页文件"),
        filters: [{ name: t("网页文件"), extensions: ["html", "htm", "mhtml", "mht"] }],
      });
      return typeof path === "string" ? captureSnapshotFromFile(path) : null;
    });

  const fromPaste = () => {
    const body = pasteText.trim();
    if (!body) {
      setError(t("请先粘贴 JD 文本"));
      return;
    }
    const source = pasteUrl.trim();
    if (source && !/^https?:\/\//i.test(source)) {
      setError(t("来源地址必须是 http:// 或 https:// 开头的网址"));
      return;
    }
    run(async () => ({
      token: null,
      sourceUrl: source || null,
      sourceKind: "paste",
      text: body,
      detail: null,
    }));
  };

  const tab = (m: Mode, label: string) => (
    <button
      type="button"
      onClick={() => { setMode(m); setError(""); }}
      className={
        "px-3 py-1 text-xs rounded-md " +
        (mode === m ? "bg-neutral-900 text-white" : "text-neutral-600 hover:bg-neutral-100")
      }
    >
      {label}
    </button>
  );

  const actionBtn =
    "px-4 py-2 text-sm rounded-md bg-neutral-700 text-white hover:bg-neutral-800 disabled:opacity-50 whitespace-nowrap";

  return (
    <div className="space-y-2">
      <div className="flex gap-1">
        {tab("url", t("岗位链接"))}
        {tab("file", t("网页文件"))}
        {tab("paste", t("粘贴文本"))}
      </div>

      {mode === "url" && (
        <div className="flex gap-2">
          <input
            className={field}
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            onKeyDown={(e) => { if (e.key === "Enter") fromUrl(); }}
            placeholder={t("粘贴岗位页面链接")}
          />
          <button type="button" onClick={fromUrl} disabled={busy !== ""} className={actionBtn}>
            {busy === "capture" ? t("抓取中…") : t("抓取快照")}
          </button>
        </div>
      )}

      {mode === "file" && (
        <div className="flex items-center gap-3">
          <button type="button" onClick={fromFile} disabled={busy !== ""} className={actionBtn}>
            {busy === "capture" ? t("读取中…") : t("选择 .html / .mhtml 文件")}
          </button>
          <p className="text-xs text-neutral-500">
            {t("在浏览器中打开岗位页，按 Ctrl+S 保存为「网页，单个文件」即可。需要登录才能看到的页面也适用。")}
          </p>
        </div>
      )}

      {mode === "paste" && (
        <div className="space-y-2">
          <textarea
            className={`${field} resize-y`}
            rows={5}
            value={pasteText}
            onChange={(e) => setPasteText(e.target.value)}
            placeholder={t("把岗位描述全文粘贴到这里")}
          />
          <div className="flex gap-2">
            <input
              className={field}
              value={pasteUrl}
              onChange={(e) => setPasteUrl(e.target.value)}
              placeholder={t("来源链接（可选）")}
            />
            <button type="button" onClick={fromPaste} disabled={busy !== ""} className={actionBtn}>
              {t("使用这段文本")}
            </button>
          </div>
        </div>
      )}

      {status && (
        <p className={"text-xs " + (status.tone === "ok" ? "text-neutral-700" : "text-amber-700")}>
          {status.text}
        </p>
      )}
      {aiOffer && (
        <div className="flex items-center gap-2 flex-wrap">
          <button
            type="button"
            onClick={runAi}
            disabled={busy !== ""}
            className="px-3 py-1.5 text-xs rounded-md border border-neutral-400 text-neutral-700 hover:bg-neutral-100 disabled:opacity-50"
          >
            {busy === "ai" ? t("AI 提取中…") : t("用 {name} 提取岗位信息", { name: aiOffer.target.name })}
          </button>
          <span className="text-xs text-neutral-400">
            {aiOffer.target.remote
              ? t("正文会发送到 {host}，发送前会再次确认", { host: aiOffer.target.host })
              : t("在本机运行，正文不会离开这台电脑")}
          </span>
        </div>
      )}
      {error && <p className="text-xs text-red-600 break-words">{error}</p>}
    </div>
  );
}
