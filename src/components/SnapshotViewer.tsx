import { useCallback, useEffect, useState } from "react";
import { confirm as askConfirm } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  listJobSnapshots, deleteJobSnapshot, saveJobSnapshot, discardSnapshotDraft, openManagedPath,
} from "../api";
import type { JobSnapshot, SnapshotDraft } from "../types";
import { useI18n } from "../i18n";
import { useModalFocus } from "../useModalFocus";
import SnapshotCapture from "./SnapshotCapture";

interface Props {
  jobId: number;
  jobTitle: string;
  companyName: string;
  onClose: () => void;
}

const KIND_LABEL: Record<string, string> = {
  url: "链接抓取",
  file: "网页文件",
  paste: "粘贴文本",
  legacy: "旧版 JD 文本",
};

const smallBtn = "text-xs text-neutral-600 hover:text-neutral-900 hover:underline whitespace-nowrap";

export default function SnapshotViewer({ jobId, jobTitle, companyName, onClose }: Props) {
  const { t } = useI18n();
  const [snapshots, setSnapshots] = useState<JobSnapshot[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [adding, setAdding] = useState(false);
  const [draft, setDraft] = useState<SnapshotDraft | null>(null);
  const [saving, setSaving] = useState(false);
  const [expanded, setExpanded] = useState<number | null>(null);
  const [copied, setCopied] = useState<number | null>(null);

  const close = () => {
    if (draft?.token) discardSnapshotDraft(draft.token).catch(() => {});
    onClose();
  };
  const modal = useModalFocus(close);

  const load = useCallback(async () => {
    const list = await listJobSnapshots(jobId);
    setSnapshots(list);
    setExpanded((prev) => prev ?? list[0]?.id ?? null);
  }, [jobId]);

  useEffect(() => {
    load().catch((e) => setError(String(e))).finally(() => setLoading(false));
  }, [load]);

  const act = async (fn: () => Promise<unknown>) => {
    setError("");
    try {
      await fn();
    } catch (e) {
      setError(String(e));
    }
  };

  const handleCaptured = (next: SnapshotDraft) => {
    if (draft?.token && draft.token !== next.token) discardSnapshotDraft(draft.token).catch(() => {});
    setDraft(next);
  };

  const saveDraft = async () => {
    if (!draft) return;
    setSaving(true);
    await act(async () => {
      const saved = await saveJobSnapshot(jobId, draft);
      setDraft(null);
      setAdding(false);
      await load();
      setExpanded(saved.id);
    });
    setSaving(false);
  };

  const remove = (s: JobSnapshot) =>
    act(async () => {
      const ok = await askConfirm(t("删除这份快照？删除后无法恢复。"), { title: t("删除确认"), kind: "warning" });
      if (!ok) return;
      await deleteJobSnapshot(s.id);
      await load();
    });

  const copy = (s: JobSnapshot) =>
    act(async () => {
      await navigator.clipboard.writeText(s.text);
      setCopied(s.id);
      setTimeout(() => setCopied((c) => (c === s.id ? null : c)), 1500);
    });

  return (
    <div className="fixed inset-0 bg-ink/45 flex items-center justify-center z-40 p-4">
      <div
        ref={modal.ref}
        onKeyDown={modal.onKeyDown}
        role="dialog"
        aria-modal="true"
        aria-labelledby="snapshot-viewer-title"
        tabIndex={-1}
        className="bg-white rounded-2xl shadow-xl border-2 border-ink w-full max-w-3xl max-h-[88vh] flex flex-col"
      >
        <div className="flex items-center justify-between px-6 py-4 border-b border-neutral-200">
          <div className="min-w-0">
            <h2 id="snapshot-viewer-title" className="font-serif text-xl font-black tracking-wide">{t("JD 快照")}</h2>
            <p className="text-sm text-neutral-500 truncate">{companyName || t("未知公司")} · {jobTitle || t("未知岗位")}</p>
          </div>
          <div className="flex gap-2 shrink-0">
            {!adding && (
              <button
                onClick={() => setAdding(true)}
                className="px-3 py-1.5 text-sm rounded-md bg-navy text-paper hover:bg-navy-deep"
              >
                {t("+ 新增快照")}
              </button>
            )}
            <button onClick={close} className="px-3 py-1.5 text-sm rounded-md border border-neutral-300 hover:bg-neutral-50">
              {t("关闭")}
            </button>
          </div>
        </div>

        <div className="flex-1 overflow-y-auto px-6 py-4 space-y-3">
          {adding && (
            <div className="rounded-lg border border-neutral-200 p-3 space-y-3 bg-neutral-50">
              <SnapshotCapture skipExtraction onCaptured={handleCaptured} />
              {draft && (
                <div className="space-y-2">
                  <pre className="max-h-40 overflow-y-auto whitespace-pre-wrap text-xs text-neutral-700 bg-white border border-neutral-200 rounded p-2">
                    {draft.text}
                  </pre>
                  <div className="flex justify-end gap-2">
                    <button
                      onClick={() => {
                        if (draft.token) discardSnapshotDraft(draft.token).catch(() => {});
                        setDraft(null);
                        setAdding(false);
                      }}
                      className="px-3 py-1.5 text-sm rounded-md border border-neutral-300 hover:bg-white"
                    >
                      {t("取消")}
                    </button>
                    <button
                      onClick={saveDraft}
                      disabled={saving}
                      className="px-3 py-1.5 text-sm rounded-md bg-navy text-paper hover:bg-navy-deep disabled:opacity-50"
                    >
                      {saving ? t("保存中…") : t("保存快照")}
                    </button>
                  </div>
                </div>
              )}
            </div>
          )}

          {loading ? (
            <div className="text-neutral-400 text-center py-8">{t("加载中…")}</div>
          ) : snapshots.length === 0 ? (
            <div className="text-neutral-400 text-center py-8">
              {t("这个岗位还没有 JD 快照。岗位下架后链接会失效，建议投递时顺手存一份。")}
            </div>
          ) : (
            snapshots.map((s) => {
              const open = expanded === s.id;
              return (
                <div key={s.id} className="border border-neutral-200 rounded-lg">
                  <div className="flex items-center gap-3 px-3 py-2 bg-neutral-50 rounded-t-lg">
                    <button
                      onClick={() => setExpanded(open ? null : s.id)}
                      className="text-sm font-medium text-left flex-1 min-w-0 truncate"
                      aria-expanded={open}
                    >
                      {open ? "▾ " : "▸ "}
                      {s.captured_at}
                      <span className="ml-2 text-xs font-normal text-neutral-500">{t(KIND_LABEL[s.source_kind] ?? s.source_kind)}</span>
                    </button>
                    {s.source_url && (
                      <button className={smallBtn} onClick={() => act(() => openUrl(s.source_url!))} title={s.source_url}>
                        {t("原链接")}
                      </button>
                    )}
                    {s.html_path && (
                      <button className={smallBtn} onClick={() => act(() => openManagedPath(s.html_path!))}>
                        {t("打开网页原件")}
                      </button>
                    )}
                    <button className={smallBtn} onClick={() => copy(s)}>
                      {copied === s.id ? t("已复制") : t("复制文本")}
                    </button>
                    <button className="text-xs text-red-500 hover:text-red-600 whitespace-nowrap" onClick={() => remove(s)}>
                      {t("删除")}
                    </button>
                  </div>
                  {open && (
                    <pre className="max-h-96 overflow-y-auto whitespace-pre-wrap break-words text-sm text-neutral-800 px-3 py-2 font-sans">
                      {s.text}
                    </pre>
                  )}
                </div>
              );
            })
          )}
        </div>

        {error && <div className="px-6 pb-3 text-sm text-red-600 break-words">{error}</div>}
      </div>
    </div>
  );
}
