import { useMemo, useRef, useState } from "react";
import {
  createApplication, updateApplication, deleteApplication, createCompany, createJob,
  saveJobSnapshot, discardSnapshotDraft,
} from "../api";
import { APPLICATION_STATUS, CATEGORIES, EMPTY_JOB_FIELDS } from "../types";
import type { Stage, Company, Job, Application, JobFields, SnapshotDraft } from "../types";
import { useI18n } from "../i18n";
import ActivityTimeline from "./ActivityTimeline";
import SnapshotCapture from "./SnapshotCapture";
import SnapshotViewer from "./SnapshotViewer";
import { confirm as askConfirm } from "@tauri-apps/plugin-dialog";
import { useModalFocus } from "../useModalFocus";

interface Props {
  state: { mode: "create" } | { mode: "edit"; app: Application };
  stages: Stage[];
  companies: Company[];
  jobs: Job[];
  onClose: () => void;
  onSaved: () => void;
  onOpenInterview?: (app: Application) => void;
}

const field = "w-full rounded-md border border-neutral-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-navy bg-white";
const label = "block text-xs font-medium text-neutral-500 mb-1";

const today = () => {
  const d = new Date();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${d.getFullYear()}-${m}-${day}`;
};

const norm = (v: string) => v.trim().toLocaleLowerCase();

export default function ApplicationModal({ state, stages, companies, jobs, onClose, onSaved, onOpenInterview }: Props) {
  const { t } = useI18n();
  const app = state.mode === "edit" ? state.app : null;
  const initialCompany = app ? jobs.find((j) => j.id === app.job_id)?.company_id ?? "" : "";

  const [companyId, setCompanyId] = useState<number | "">(initialCompany);
  const [jobId, setJobId] = useState<number | "">(app ? app.job_id : "");
  const [channel, setChannel] = useState(app?.channel ?? "");
  const [resumeVersion, setResumeVersion] = useState(app?.resume_version ?? "");
  const [stageId, setStageId] = useState<number>(app?.stage_id ?? stages[0]?.id ?? 0);
  const [status, setStatus] = useState<string>(app?.status ?? APPLICATION_STATUS[0]);
  const [appliedAt, setAppliedAt] = useState(app?.applied_at ? app.applied_at.slice(0, 10) : "");
  const [notes, setNotes] = useState(app?.notes ?? "");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");

  // JD 快照与新岗位信息：都只在保存投递时才落库
  const [draft, setDraft] = useState<SnapshotDraft | null>(null);
  const [fields, setFields] = useState<JobFields>(EMPTY_JOB_FIELDS);
  const [showSnapshots, setShowSnapshots] = useState(false);
  // 快照先于投递保存；投递若保存失败后重试，不能把同一份快照再存一遍
  const snapshotSavedFor = useRef<number | null>(null);

  const close = () => {
    if (draft?.token && snapshotSavedFor.current === null) {
      discardSnapshotDraft(draft.token).catch(() => {});
    }
    onClose();
  };
  const modal = useModalFocus(close);

  const companyJobs = useMemo(
    () => jobs.filter((j) => j.company_id === (companyId === "" ? -1 : companyId)),
    [jobs, companyId],
  );
  const selectedCompany = companies.find((c) => c.id === companyId);
  const currentJob = jobs.find((j) => j.id === jobId);

  const setField = (key: keyof JobFields, value: string) =>
    setFields((prev) => ({ ...prev, [key]: value }));

  // 识别出的公司 / 岗位已存在就直接选中，快照会追加到已有岗位上
  const matchExisting = (detected: JobFields) => {
    const knownCompany = detected.companyName
      ? companies.find((c) => norm(c.name) === norm(detected.companyName))
      : undefined;
    if (knownCompany) {
      setCompanyId(knownCompany.id);
      const knownJob = detected.title
        ? jobs.find((j) => j.company_id === knownCompany.id && norm(j.title) === norm(detected.title))
        : undefined;
      setJobId(knownJob ? knownJob.id : "");
    } else if (detected.companyName) {
      setCompanyId("");
      setJobId("");
    }
  };

  const handleCaptured = (next: SnapshotDraft, detected: JobFields) => {
    if (draft?.token && draft.token !== next.token) discardSnapshotDraft(draft.token).catch(() => {});
    setDraft(next);
    setFields(detected);
    snapshotSavedFor.current = null;
    matchExisting(detected);
    if (!channel) setChannel("官网");
  };

  // AI 提取要等好几秒，期间用户可能还在填表。回调里必须读最新的值，
  // 不能用发起请求那一刻闭包里的旧值，否则会把刚填的内容覆盖掉。
  const fieldsRef = useRef(fields);
  fieldsRef.current = fields;
  const jobIdRef = useRef(jobId);
  jobIdRef.current = jobId;

  /** AI 结果只填用户还空着的字段，不覆盖已经手动填好的内容。 */
  const handleExtracted = (detected: JobFields) => {
    const merged = { ...fieldsRef.current };
    for (const key of Object.keys(merged) as (keyof JobFields)[]) {
      if (!merged[key].trim() && detected[key]) merged[key] = detected[key];
    }
    setFields(merged);
    if (jobIdRef.current === "") matchExisting(merged);
  };

  const toSalary = (value: string, name: string): number | null => {
    const v = value.trim();
    if (!v) return null;
    const n = Number(v);
    if (!Number.isInteger(n) || n < 0) throw new Error(t("{name}必须是非负整数", { name }));
    return n;
  };

  /** 选了已有岗位就用它；否则按表单新建公司 / 岗位。 */
  const resolveJobId = async (): Promise<number> => {
    if (jobId !== "") return Number(jobId);
    if (state.mode === "edit") throw new Error(t("请选择公司与岗位"));
    const title = fields.title.trim();
    if (!title) throw new Error(t("请选择已有岗位，或填写新岗位名称"));
    // 先校验完所有输入再写库，免得公司建好了、岗位因为薪资格式不对建失败，留下孤儿公司
    const salaryMin = toSalary(fields.salaryMin, t("薪资下限"));
    const salaryMax = toSalary(fields.salaryMax, t("薪资上限"));
    if (salaryMin != null && salaryMax != null && salaryMin > salaryMax) {
      throw new Error(t("薪资下限不能高于上限"));
    }
    let company: Company | undefined = selectedCompany
      ?? companies.find((c) => norm(c.name) === norm(fields.companyName));
    if (!company) {
      if (!fields.companyName.trim()) throw new Error(t("请选择公司，或填写公司名称"));
      company = await createCompany({ name: fields.companyName.trim() });
      setCompanyId(company.id);
    }
    const existing = jobs.find((j) => j.company_id === company!.id && norm(j.title) === norm(title));
    if (existing) return existing.id;
    const job = await createJob({
      companyId: company.id,
      title,
      department: fields.department.trim() || null,
      location: fields.location.trim() || null,
      category: fields.category || null,
      jdUrl: draft?.sourceUrl ?? null,
      jdText: null,
      salaryMin,
      salaryMax,
    });
    setJobId(job.id);
    return job.id;
  };

  const handleSubmit = async () => {
    if (stages.length === 0) {
      setError(t("请先在「公司与岗位」中添加至少一个阶段"));
      return;
    }
    setSaving(true);
    setError("");
    try {
      const resolvedJobId = await resolveJobId();
      if (draft && snapshotSavedFor.current !== resolvedJobId) {
        await saveJobSnapshot(resolvedJobId, draft);
        snapshotSavedFor.current = resolvedJobId;
      }
      const payload = {
        jobId: resolvedJobId,
        channel: channel || null,
        resumeVersion: resumeVersion || null,
        stageId,
        status,
        appliedAt: appliedAt || null,
        notes: notes || null,
      };
      if (state.mode === "edit") {
        await updateApplication(app!.id, payload);
      } else {
        await createApplication(payload);
      }
      onSaved();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async () => {
    if (!app) return;
    const ok = await askConfirm(t("删除投递会同时删除面试、复盘和附件，确认继续？"), { title: t("删除确认"), kind: "warning" });
    if (!ok) return;
    setSaving(true);
    try { await deleteApplication(app.id); await onSaved(); }
    catch (e) { setError(String(e)); }
    finally { setSaving(false); }
  };

  const needsNewJob = state.mode === "create" && jobId === "";

  return (
    <>
      <div className="fixed inset-0 bg-ink/45 flex items-center justify-center z-20 p-4">
        <div ref={modal.ref} onKeyDown={modal.onKeyDown} role="dialog" aria-modal="true" aria-labelledby="application-modal-title" tabIndex={-1} className="bg-white rounded-2xl shadow-xl border-2 border-ink w-full max-w-xl max-h-[90vh] overflow-y-auto p-6">
          <h2 id="application-modal-title" className="font-serif text-xl font-black tracking-wide mb-4">{state.mode === "edit" ? t("编辑投递") : t("新建投递")}</h2>

          {state.mode === "create" && (
            <div className="mb-4">
              <label className={label}>{t("JD 快照（可选）")}</label>
              <SnapshotCapture onCaptured={handleCaptured} onExtracted={handleExtracted} />
              {draft && (
                <p className="mt-1 text-xs text-neutral-400">
                  {t("快照共 {n} 字，保存投递时会一并存档", { n: draft.text.length })}
                </p>
              )}
            </div>
          )}

          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className={label}>{t("公司")}</label>
              <select
                className={field}
                value={companyId}
                onChange={(e) => {
                  setCompanyId(e.target.value === "" ? "" : Number(e.target.value));
                  setJobId("");
                }}
              >
                <option value="">{state.mode === "create" ? t("新公司（在下方填写）") : t("选择公司")}</option>
                {companies.map((c) => (
                  <option key={c.id} value={c.id}>{c.name}</option>
                ))}
              </select>
            </div>
            <div>
              <label className={label}>{t("岗位")}</label>
              <select
                className={field}
                value={jobId}
                onChange={(e) => setJobId(e.target.value === "" ? "" : Number(e.target.value))}
              >
                <option value="">{state.mode === "create" ? t("新岗位（在下方填写）") : t("选择岗位")}</option>
                {companyJobs.map((j) => (
                  <option key={j.id} value={j.id}>{j.title}</option>
                ))}
              </select>
            </div>
          </div>

          {needsNewJob && (
            <div className="mt-3 rounded-lg border border-neutral-200 bg-neutral-50 p-3">
              <div className="text-xs font-medium text-neutral-500 mb-2">{t("新岗位信息（保存时创建）")}</div>
              <div className="grid grid-cols-2 gap-2">
                {selectedCompany ? (
                  <div className="flex items-center text-sm text-neutral-600 px-1">{selectedCompany.name}</div>
                ) : (
                  <input className={field} value={fields.companyName} onChange={(e) => setField("companyName", e.target.value)} placeholder={t("公司名称 *")} />
                )}
                <input className={field} value={fields.title} onChange={(e) => setField("title", e.target.value)} placeholder={t("岗位名称 *")} />
                <input className={field} value={fields.department} onChange={(e) => setField("department", e.target.value)} placeholder={t("部门（可选）")} />
                <input className={field} value={fields.location} onChange={(e) => setField("location", e.target.value)} placeholder={t("城市（可选）")} />
                <select className={field} value={fields.category} onChange={(e) => setField("category", e.target.value)}>
                  <option value="">{t("选择分类")}</option>
                  {CATEGORIES.map((c) => <option key={c} value={c}>{t(c)}</option>)}
                </select>
                <div className="flex gap-2">
                  <input className={field} inputMode="numeric" value={fields.salaryMin} onChange={(e) => setField("salaryMin", e.target.value)} placeholder={t("薪资下限 k")} />
                  <input className={field} inputMode="numeric" value={fields.salaryMax} onChange={(e) => setField("salaryMax", e.target.value)} placeholder={t("薪资上限 k")} />
                </div>
              </div>
            </div>
          )}
          {state.mode === "create" && jobId !== "" && draft && (
            <p className="mt-2 text-xs text-neutral-500">{t("已匹配到现有岗位，快照会追加到该岗位的快照记录里。")}</p>
          )}

          <div className="grid grid-cols-2 gap-4 mt-4">
            <div>
              <label className={label}>{t("投递渠道")}</label>
              <input className={field} value={channel} onChange={(e) => setChannel(e.target.value)} placeholder={t("如 Boss直聘 / 官网")} />
            </div>
            <div>
              <label className={label}>{t("简历版本")}</label>
              <input className={field} value={resumeVersion} onChange={(e) => setResumeVersion(e.target.value)} placeholder={t("可选")} />
            </div>
            <div>
              <label className={label}>{t("当前阶段")}</label>
              <select className={field} value={stageId} onChange={(e) => setStageId(Number(e.target.value))}>
                {stages.map((s) => (
                  <option key={s.id} value={s.id}>{t(s.name)}</option>
                ))}
              </select>
            </div>
            <div>
              <label className={label}>{t("状态")}</label>
              <select className={field} value={status} onChange={(e) => setStatus(e.target.value)}>
                {APPLICATION_STATUS.map((s) => (
                  <option key={s} value={s}>{t(s)}</option>
                ))}
              </select>
            </div>
            <div>
              <label className={label}>{t("投递日期")}</label>
              <div className="flex gap-2">
                <input type="date" className={field} value={appliedAt} onChange={(e) => setAppliedAt(e.target.value)} />
                <button type="button" onClick={() => setAppliedAt(today())} className="px-3 py-2 text-sm rounded-md border border-neutral-300 hover:bg-neutral-50 whitespace-nowrap">{t("今天")}</button>
              </div>
            </div>
          </div>

          <div className="mt-4">
            <label className={label}>{t("备注")}</label>
            <textarea className={`${field} resize-none`} rows={3} value={notes} onChange={(e) => setNotes(e.target.value)} placeholder={t("可选")} />
          </div>

          {state.mode === "edit" && <ActivityTimeline applicationId={app!.id} />}

          {error && <div className="mt-3 text-sm text-red-600 break-words">{error}</div>}

          <div className="mt-6 flex justify-between items-center gap-2">
            <div>
              {state.mode === "edit" && (
                <div className="flex gap-2">
                  {onOpenInterview && (
                    <button onClick={() => onOpenInterview(app!)} className="px-4 py-2 text-sm rounded-md bg-neutral-100 hover:bg-neutral-200">{t("面试复盘")}</button>
                  )}
                  {currentJob && (
                    <button onClick={() => setShowSnapshots(true)} className="px-4 py-2 text-sm rounded-md bg-neutral-100 hover:bg-neutral-200">{t("JD 快照")}</button>
                  )}
                  <button onClick={handleDelete} disabled={saving} className="px-3 py-2 text-sm text-red-600 hover:bg-red-50 rounded-md">{t("删除投递")}</button>
                </div>
              )}
            </div>
            <div className="flex gap-2">
              <button onClick={close} className="px-4 py-2 text-sm rounded-md border border-neutral-300 hover:bg-neutral-50">
                {t("取消")}
              </button>
              <button
                onClick={handleSubmit}
                disabled={saving}
                className="px-4 py-2 text-sm rounded-md bg-navy text-paper hover:bg-navy-deep disabled:opacity-50"
              >
                {saving ? t("保存中…") : t("保存")}
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* 放在对话框 DOM 之外：嵌套在里面的话，Tab / Esc 会被外层的焦点陷阱重复处理 */}
      {showSnapshots && currentJob && (
        <SnapshotViewer
          jobId={currentJob.id}
          jobTitle={currentJob.title}
          companyName={companies.find((c) => c.id === currentJob.company_id)?.name ?? ""}
          onClose={() => setShowSnapshots(false)}
        />
      )}
    </>
  );
}
