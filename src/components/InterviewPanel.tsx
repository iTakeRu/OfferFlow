import { useCallback, useEffect, useState } from "react";
import { open, confirm } from "@tauri-apps/plugin-dialog";
import {
  listInterviews, createInterview, updateInterview, deleteInterview,
  listAttachments, importAttachment, deleteAttachment, listAudioMarks, openManagedPath,
} from "../api";
import type { Application, Interview, Attachment, AudioMark } from "../types";
import { INTERVIEW_FORMAT, INTERVIEW_STATUS } from "../types";
import { formatSize, formatTime } from "../format";
import { useI18n } from "../i18n";
import { useModalFocus } from "../useModalFocus";
import RichTextEditor from "./RichTextEditor";
import AudioPlayer from "./AudioPlayer";

interface Props {
  app: Application;
  jobTitle: string;
  companyName: string;
  onClose: () => void;
}

interface Draft {
  format: string;
  location: string;
  interviewer: string;
  scheduled_at: string;
  rating: string;
  status: string;
  feedback: string;
}

const field =
  "w-full rounded-md border border-neutral-300 px-2.5 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-navy bg-white";
const fieldAuto =
  "rounded-md border border-neutral-300 px-2 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-navy bg-white";
const label = "block text-xs font-medium text-neutral-500 mb-1";

const KIND_LABEL: Record<string, string> = { audio: "音频", video: "视频", file: "文件" };
const KIND_BADGE: Record<string, string> = {
  audio: "bg-neutral-200 text-neutral-700",
  video: "bg-purple-100 text-purple-700",
  file: "bg-neutral-200 text-neutral-600",
};

function draftFrom(i: Interview): Draft {
  return {
    format: i.format ?? INTERVIEW_FORMAT[0],
    location: i.location ?? "",
    interviewer: i.interviewer ?? "",
    scheduled_at: i.scheduled_at ? i.scheduled_at.slice(0, 10) : "",
    rating: i.rating != null ? String(i.rating) : "",
    status: i.status ?? INTERVIEW_STATUS[0],
    feedback: i.feedback ?? "",
  };
}

function detectKind(name: string): string {
  const ext = name.split(".").pop()?.toLowerCase() ?? "";
  if (["mp3", "wav", "m4a", "ogg", "flac", "aac"].includes(ext)) return "audio";
  if (["mp4", "mov", "webm", "avi", "mkv"].includes(ext)) return "video";
  return "file";
}

export default function InterviewPanel({ app, jobTitle, companyName, onClose }: Props) {
  const { t } = useI18n();
  const modal = useModalFocus(onClose);
  const [interviews, setInterviews] = useState<Interview[]>([]);
  const [drafts, setDrafts] = useState<Record<number, Draft>>({});
  const [attachments, setAttachments] = useState<Record<number, Attachment[]>>({});
  const [marks, setMarks] = useState<Record<number, AudioMark[]>>({});
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [savingId, setSavingId] = useState<number | null>(null);

  const load = useCallback(async () => {
    const list = await listInterviews(app.id);
    setInterviews(list);
    setDrafts((prev) => {
      const d: Record<number, Draft> = {};
      for (const it of list) {
        d[it.id] = prev[it.id] ?? draftFrom(it);
      }
      return d;
    });
    const loaded = await Promise.all(
      list.map(async (it) => ({
        id: it.id,
        attachments: await listAttachments(it.id),
        marks: await listAudioMarks(it.id),
      })),
    );
    setAttachments(Object.fromEntries(loaded.map((x) => [x.id, x.attachments])));
    setMarks(Object.fromEntries(loaded.map((x) => [x.id, x.marks])));
  }, [app.id]);

  useEffect(() => {
    load()
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  }, [load]);

  const addRound = async () => {
    setError("");
    const next = interviews.reduce((m, i) => Math.max(m, i.round), 0) + 1;
    try {
      await createInterview({ applicationId: app.id, round: next });
      await load();
    } catch (e) {
      setError(String(e));
    }
  };

  const updateDraft = (id: number, patch: Partial<Draft>) => {
    setDrafts((prev) => ({ ...prev, [id]: { ...prev[id], ...patch } }));
  };

  const saveRound = async (id: number) => {
    const it = interviews.find((x) => x.id === id);
    const d = drafts[id];
    if (!it || !d) return;
    setSavingId(id);
    setError("");
    try {
      await updateInterview(id, {
        applicationId: app.id,
        round: it.round,
        format: d.format || null,
        location: d.location || null,
        interviewer: d.interviewer || null,
        scheduledAt: d.scheduled_at || null,
        rating: d.rating === "" ? null : Number(d.rating),
        status: d.status,
        feedback: d.feedback || null,
      });
    } catch (e) {
      setError(String(e));
    } finally {
      setSavingId(null);
    }
  };

  const removeRound = async (id: number) => {
    const ok = await confirm(t("删除后该轮面试及其复盘笔记、附件将一并删除，确认？"), {
      title: t("删除确认"),
      kind: "warning",
    });
    if (!ok) return;
    setError("");
    try {
      await deleteInterview(id);
      await load();
    } catch (e) {
      setError(String(e));
    }
  };

  const importFile = async (interviewId: number) => {
    setError("");
    try {
      const selected = await open({ multiple: false, filters: [{ name: t("面试附件"), extensions: ["mp3", "wav", "m4a", "aac", "ogg", "flac", "mp4", "webm", "pdf", "doc", "docx", "txt", "png", "jpg", "jpeg", "webp"] }] });
      if (typeof selected !== "string") return;
      const kind = detectKind(selected);
      await importAttachment({ interviewId, kind, sourcePath: selected });
      const list = await listAttachments(interviewId);
      setAttachments((prev) => ({ ...prev, [interviewId]: list }));
    } catch (e) {
      setError(String(e));
    }
  };

  const removeAttachment = async (interviewId: number, attId: number) => {
    setError("");
    try {
      await deleteAttachment(attId);
      const [list, markList] = await Promise.all([
        listAttachments(interviewId),
        listAudioMarks(interviewId),
      ]);
      setAttachments((prev) => ({ ...prev, [interviewId]: list }));
      setMarks((prev) => ({ ...prev, [interviewId]: markList }));
    } catch (e) {
      setError(String(e));
    }
  };

  const openAttachment = async (path: string) => {
    setError("");
    try {
      await openManagedPath(path);
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div className="fixed inset-0 bg-ink/45 flex items-center justify-center z-30 p-4">
      <div ref={modal.ref} onKeyDown={modal.onKeyDown} role="dialog" aria-modal="true" aria-label={t("面试复盘")} tabIndex={-1} className="bg-white rounded-2xl shadow-xl border-2 border-ink w-full max-w-4xl max-h-[90vh] flex flex-col overflow-hidden">
        <div className="flex items-center justify-between px-6 py-4 border-b border-neutral-200">
          <div>
            <h2 className="font-serif text-xl font-black tracking-wide">{t("面试复盘")}</h2>
            <p className="text-sm text-neutral-500">
              {companyName || t("未知公司")} · {jobTitle || t("未知岗位")}
            </p>
          </div>
          <div className="flex gap-2">
            <button
              onClick={addRound}
              className="px-3 py-1.5 text-sm rounded-md bg-navy text-paper hover:bg-navy-deep"
            >
              {t("+ 新增一轮")}
            </button>
            <button
              onClick={onClose}
              className="px-3 py-1.5 text-sm rounded-md border border-neutral-300 hover:bg-neutral-50"
            >
              {t("关闭")}
            </button>
          </div>
        </div>

        <div className="flex-1 overflow-y-auto p-6 space-y-6">
          {loading ? (
            <div className="text-neutral-500 text-center py-12">{t("加载中…")}</div>
          ) : interviews.length === 0 ? (
            <div className="text-neutral-500 text-center py-12">
              {t("暂无面试记录，点击右上角「新增一轮」开始复盘。")}
            </div>
          ) : (
            interviews.map((it) => {
              const d = drafts[it.id] ?? draftFrom(it);
              const atts = attachments[it.id] ?? [];
              return (
                <div key={it.id} className="border border-neutral-200 rounded-lg">
                  <div className="flex items-center gap-3 px-4 py-3 border-b border-neutral-200 bg-neutral-50 rounded-t-lg">
                    <span className="font-semibold text-sm whitespace-nowrap">{t("第 {n} 轮", { n: it.round })}</span>
                    <select
                      className={fieldAuto}
                      value={d.format}
                      onChange={(e) => updateDraft(it.id, { format: e.target.value })}
                    >
                      {INTERVIEW_FORMAT.map((f) => (
                        <option key={f} value={f}>
                          {t(f)}
                        </option>
                      ))}
                    </select>
                    <select
                      className={fieldAuto}
                      value={d.status}
                      onChange={(e) => updateDraft(it.id, { status: e.target.value })}
                    >
                      {INTERVIEW_STATUS.map((s) => (
                        <option key={s} value={s}>
                          {t(s)}
                        </option>
                      ))}
                    </select>
                    <div className="flex-1" />
                    <button
                      onClick={() => removeRound(it.id)}
                      className="text-sm text-red-500 hover:text-red-600 whitespace-nowrap"
                    >
                      {t("删除本轮")}
                    </button>
                  </div>

                  <div className="p-4 space-y-4">
                    <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
                      <div>
                        <label className={label}>{t("地点")}</label>
                        <input
                          className={field}
                          value={d.location}
                          onChange={(e) => updateDraft(it.id, { location: e.target.value })}
                          placeholder={t("如 线上/北京")}
                        />
                      </div>
                      <div>
                        <label className={label}>{t("面试官")}</label>
                        <input
                          className={field}
                          value={d.interviewer}
                          onChange={(e) => updateDraft(it.id, { interviewer: e.target.value })}
                          placeholder={t("姓名/职位")}
                        />
                      </div>
                      <div>
                        <label className={label}>{t("面试日期")}</label>
                        <input
                          type="date"
                          className={field}
                          value={d.scheduled_at}
                          onChange={(e) => updateDraft(it.id, { scheduled_at: e.target.value })}
                        />
                      </div>
                      <div>
                        <label className={label}>{t("评分 (1-5)")}</label>
                        <input
                          type="number"
                          min={1}
                          max={5}
                          className={field}
                          value={d.rating}
                          onChange={(e) => updateDraft(it.id, { rating: e.target.value })}
                        />
                      </div>
                    </div>

                    <div>
                      <label className={label}>{t("复盘笔记")}</label>
                      <RichTextEditor
                        key={it.id}
                        value={d.feedback}
                        onChange={(html) => updateDraft(it.id, { feedback: html })}
                      />
                    </div>

                    <div>
                      <div className="flex items-center justify-between mb-2">
                        <label className="text-xs font-medium text-neutral-500">{t("附件（{n}）", { n: atts.length })}</label>
                        <button
                          onClick={() => importFile(it.id)}
                          className="px-2 py-1 text-xs rounded-md border border-neutral-300 hover:bg-neutral-50"
                        >
                          {t("+ 导入附件")}
                        </button>
                      </div>
                      {atts.length === 0 ? (
                        <div className="text-xs text-neutral-400">{t("无附件")}</div>
                      ) : (
                        <ul className="space-y-1.5">
                          {atts.map((a) => (
                            <li
                              key={a.id}
                              className="flex items-center gap-2 text-sm bg-neutral-50 rounded px-2.5 py-1.5"
                            >
                              <span className={`text-xs px-1.5 py-0.5 rounded ${KIND_BADGE[a.kind] ?? "bg-neutral-200 text-neutral-600"}`}>
                                {t(KIND_LABEL[a.kind] ?? a.kind)}
                              </span>
                              <span className="flex-1 truncate">{a.filename}</span>
                              <span className="text-xs text-neutral-400 whitespace-nowrap">
                                {formatSize(a.size)}
                                {a.duration != null ? ` · ${formatTime(a.duration)}` : ""}
                              </span>
                              <button
                                onClick={() => openAttachment(a.filepath)}
                                className="text-xs text-neutral-700 hover:underline whitespace-nowrap"
                              >
                                {t("打开")}
                              </button>
                              <button
                                onClick={() => removeAttachment(it.id, a.id)}
                                className="text-xs text-red-500 hover:text-red-600 whitespace-nowrap"
                              >
                                {t("删除")}
                              </button>
                            </li>
                          ))}
                        </ul>
                      )}
                      {atts.filter((a) => a.kind === "audio").length > 0 && (
                        <div className="mt-3 space-y-3">
                          {atts.filter((a) => a.kind === "audio").map((a) => (
                            <AudioPlayer
                              key={a.id}
                              attachment={a}
                              interviewId={it.id}
                              marks={(marks[it.id] ?? []).filter((m) => m.attachment_id === a.id)}
                              onMarksChanged={(next) =>
                                setMarks((prev) => ({
                                  ...prev,
                                  [it.id]: [
                                    ...(prev[it.id] ?? []).filter((m) => m.attachment_id !== a.id),
                                    ...next,
                                  ],
                                }))
                              }
                            />
                          ))}
                        </div>
                      )}
                    </div>

                    <div className="flex justify-end">
                      <button
                        onClick={() => saveRound(it.id)}
                        disabled={savingId === it.id}
                        className="px-4 py-2 text-sm rounded-md bg-navy text-paper hover:bg-navy-deep disabled:opacity-50"
                      >
                        {savingId === it.id ? t("保存中…") : t("保存本轮")}
                      </button>
                    </div>
                  </div>
                </div>
              );
            })
          )}
        </div>

        {error && <div className="px-6 pb-3 text-sm text-red-600">{error}</div>}
      </div>
    </div>
  );
}
