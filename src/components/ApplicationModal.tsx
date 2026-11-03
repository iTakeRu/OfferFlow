import { useMemo, useRef, useState } from "react";
import {
  createApplication, updateApplication, deleteApplication, createCompany, createJob, updateJob,
  saveJobSnapshot, discardSnapshotDraft,
} from "../api";
import { APPLICATION_STATUS, EMPTY_JOB_FIELDS, categoryOptions } from "../types";
import type { Stage, Company, Job, Application, JobFields, SnapshotDraft } from "../types";
import { useI18n } from "../i18n";
import ActivityTimeline from "./ActivityTimeline";
import { CategoryInput, CustomSelect } from "./CustomSelect";
import SnapshotCapture from "./SnapshotCapture";
import SnapshotViewer from "./SnapshotViewer";
import { confirm as askConfirm } from "@tauri-apps/plugin-dialog";
import { useModalFocus } from "../useModalFocus";
import { channelFromSource, channelOptions, createApplicationSaver } from "../applicationSave";
import type { ApplicationInput } from "../api";

interface Props {
  state: { mode: "create" } | { mode: "edit"; app: Application };
  stages: Stage[];
  companies: Company[];
  jobs: Job[];
  applications: Application[];
  onClose: () => void;
  onSaved: () => void | Promise<void>;
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

export default function ApplicationModal({ state, stages, companies, jobs, applications, onClose, onSaved, onOpenInterview }: Props) {
  const { t } = useI18n();
  const app = state.mode === "edit" ? state.app : null;
  const currentJob = app ? jobs.find((j) => j.id === app.job_id) : undefined;

  const [companyId, setCompanyId] = useState<number | "">(currentJob?.company_id ?? "");
  const [manualChannel, setChannel] = useState<string | null>(app?.channel ?? null);
  const [resumeVersion, setResumeVersion] = useState(app?.resume_version ?? "");
  const [stageId, setStageId] = useState<number>(app?.stage_id ?? stages[0]?.id ?? 0);
  const [status, setStatus] = useState<string>(app?.status ?? APPLICATION_STATUS[0]);
  const [appliedAt, setAppliedAt] = useState(app?.applied_at ? app.applied_at.slice(0, 10) : "");
  const [notes, setNotes] = useState(app?.notes ?? "");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");

  // JD 快照与新岗位信息：都只在保存投递时才落库
  const [draft, setDraft] = useState<SnapshotDraft | null>(null);
  const [fields, setFields] = useState<JobFields>(() => currentJob ? {
    title: currentJob.title, companyName: companies.find((c) => c.id === currentJob.company_id)?.name ?? "",
    department: currentJob.department ?? "", location: currentJob.location ?? "", category: currentJob.category ?? "",
    salaryMin: currentJob.salary_min?.toString() ?? "", salaryMax: currentJob.salary_max?.toString() ?? "",
  } : EMPTY_JOB_FIELDS);
  const channel = manualChannel ?? channelFromSource(draft?.sourceUrl);
  const channels = useMemo(() => channelOptions(applications), [applications]);
  const selectedChannel = channels.find((c) => norm(c) === norm(channel)) ?? channel;
  // Keep successful writes on a retry even if refreshing the parent lists failed.
  const savedJob = useRef(currentJob);
  const createdCompany = useRef<Company | null>(null);
  const [showSnapshots, setShowSnapshots] = useState(false);
  // 快照先于投递保存；投递若保存失败后重试，不能把同一份快照再存一遍
  const snapshotSavedFor = useRef<number | null>(null);
  // 创建成功后刷新可能失败；重试应更新这条投递，不能再次创建。
  const [saveApplication] = useState(() => createApplicationSaver<ApplicationInput & { status: string }>(
    app?.id ?? null, createApplication, updateApplication,
  ));
  const submitting = useRef(false);

  const close = () => {
    if (draft?.token && snapshotSavedFor.current === null) {
      discardSnapshotDraft(draft.token).catch(() => {});
    }
    onClose();
  };
  const modal = useModalFocus(close);

  const selectedCompany = companies.find((c) => c.id === companyId);
  const categories = useMemo(() => categoryOptions(jobs), [jobs]);

  const setField = (key: keyof JobFields, value: string) =>
    setFields((prev) => ({ ...prev, [key]: value }));

  const matchCompany = (name: string) => {
    if (name.trim()) setCompanyId(companies.find((c) => norm(c.name) === norm(name))?.id ?? "");
  };

  const handleCaptured = (next: SnapshotDraft, detected: JobFields) => {
    if (draft?.token && draft.token !== next.token) discardSnapshotDraft(draft.token).catch(() => {});
    setDraft(next);
    setFields(detected);
    snapshotSavedFor.current = null;
    matchCompany(detected.companyName);
  };

  // AI 提取要等好几秒，期间用户可能还在填表。回调里必须读最新的值，
  // 不能用发起请求那一刻闭包里的旧值，否则会把刚填的内容覆盖掉。
  const fieldsRef = useRef(fields);
  fieldsRef.current = fields;
  const companyIdRef = useRef(companyId);
  companyIdRef.current = companyId;

  /** AI 结果只填用户还空着的字段，不覆盖已经手动填好的内容。 */
  const handleExtracted = (detected: JobFields) => {
    const merged = { ...fieldsRef.current };
    for (const key of Object.keys(merged) as (keyof JobFields)[]) {
      if (!merged[key].trim() && detected[key]) merged[key] = detected[key];
    }
    setFields(merged);
    if (companyIdRef.current === "") matchCompany(merged.companyName);
  };

  const toSalary = (value: string, name: string): number | null => {
    const v = value.trim();
    if (!v) return null;
    const n = Number(v);
    if (!Number.isInteger(n) || n < 0) throw new Error(t("{name}必须是非负整数", { name }));
    return n;
  };

  /** Each new application gets the posting captured in this form, even if titles coincide. */
  const resolveJobId = async (): Promise<number> => {
    const title = fields.title.trim();
    if (!title) throw new Error(t("请填写岗位名称"));
    // 先校验完所有输入再写库，免得公司建好了、岗位因为薪资格式不对建失败，留下孤儿公司
    const salaryMin = toSalary(fields.salaryMin, t("薪资下限"));
    const salaryMax = toSalary(fields.salaryMax, t("薪资上限"));
    if (salaryMin != null && salaryMax != null && salaryMin > salaryMax) {
      throw new Error(t("薪资下限不能高于上限"));
    }
    let company: Company | undefined = selectedCompany
      ?? companies.find((c) => norm(c.name) === norm(fields.companyName))
      ?? (createdCompany.current && norm(createdCompany.current.name) === norm(fields.companyName) ? createdCompany.current : undefined);
    if (!company) {
      if (!fields.companyName.trim()) throw new Error(t("请选择公司，或填写公司名称"));
      company = await createCompany({ name: fields.companyName.trim() });
      createdCompany.current = company;
    }
    const before = savedJob.current;
    const payload = {
      companyId: company.id,
      title,
      department: fields.department.trim() || null,
      location: fields.location.trim() || null,
      category: fields.category.trim() || null,
      jdUrl: app ? currentJob?.jd_url ?? null : draft?.sourceUrl ?? null,
      jdText: currentJob?.jd_text ?? null,
      salaryMin,
      salaryMax,
    };
    if (!before) {
      savedJob.current = await createJob({ ...payload, allowSameTitle: true });
    } else if (before.company_id !== company.id || before.title !== title ||
      before.department !== payload.department || before.location !== payload.location ||
      before.category !== payload.category || before.salary_min !== salaryMin ||
      before.salary_max !== salaryMax || before.jd_url !== payload.jdUrl) {
      savedJob.current = await updateJob(before.id, payload);
    }
    return savedJob.current!.id;
  };

  const handleSubmit = async () => {
    if (submitting.current) return;
    if (stages.length === 0) {
      setError(t("请先在「公司与岗位」中添加至少一个阶段"));
      return;
    }
    submitting.current = true;
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
        channel: channel.trim() || null,
        resumeVersion: resumeVersion || null,
        stageId,
        status,
        appliedAt: appliedAt || null,
        notes: notes || null,
      };
      await saveApplication(payload, onSaved);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      submitting.current = false;
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

  return (
    <>
      <div className="fixed inset-0 bg-scrim flex items-center justify-center z-20 p-4">
        <div ref={modal.ref} onKeyDown={modal.onKeyDown} role="dialog" aria-modal="true" aria-labelledby="application-modal-title" tabIndex={-1} className="bg-white rounded-2xl shadow-xl border-2 border-rule w-full max-w-xl max-h-[90vh] overflow-y-auto p-6">
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
              <label className={label} htmlFor="application-company">{t("公司")}</label>
              <select
                id="application-company"
                className={field}
                value={companyId}
                onChange={(e) => {
                  setCompanyId(e.target.value === "" ? "" : Number(e.target.value));
                  setField("companyName", "");
                }}
              >
                <option value="">{t("新公司（在下方填写）")}</option>
                {companies.map((c) => (
                  <option key={c.id} value={c.id}>{c.name}</option>
                ))}
              </select>
              {!selectedCompany && (
                <input className={`${field} mt-2`} value={fields.companyName} onChange={(e) => setField("companyName", e.target.value)} aria-label={t("公司名称 *")} placeholder={t("公司名称 *")} />
              )}
            </div>
            <div>
              <label className={label} htmlFor="application-title">{t("岗位")}</label>
              <input id="application-title" autoComplete="off" className={field} value={fields.title} onChange={(e) => setField("title", e.target.value)} placeholder={t("岗位名称 *")} />
            </div>
          </div>

          <div className="mt-3 rounded-lg border border-neutral-200 bg-neutral-50 p-3">
            {currentJob && applications.some((a) => a.id !== app?.id && a.job_id === currentJob.id) && (
              <div className="text-xs font-medium text-neutral-500 mb-2">{t("岗位信息（保存时更新，同一岗位的投递共用）")}</div>
            )}
            <div className="grid grid-cols-2 gap-2">
              <input className={field} value={fields.department} onChange={(e) => setField("department", e.target.value)} placeholder={t("部门（可选）")} />
              <input className={field} value={fields.location} onChange={(e) => setField("location", e.target.value)} placeholder={t("城市（可选）")} />
              <CategoryInput className={field} value={fields.category} onChange={(v) => setField("category", v)} options={categories} />
              <div className="flex gap-2">
                <input className={field} inputMode="numeric" value={fields.salaryMin} onChange={(e) => setField("salaryMin", e.target.value)} placeholder={t("薪资下限 k")} />
                <input className={field} inputMode="numeric" value={fields.salaryMax} onChange={(e) => setField("salaryMax", e.target.value)} placeholder={t("薪资上限 k")} />
              </div>
            </div>
          </div>

          <div className="grid grid-cols-2 gap-4 mt-4">
            <div>
              <label className={label} htmlFor="application-channel">{t("投递渠道")}</label>
              <CustomSelect id="application-channel" className={field} label="投递渠道" prompt="选择渠道"
                placeholder="输入投递渠道" value={selectedChannel} onChange={setChannel} options={channels} />
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
                className="px-4 py-2 text-sm rounded-md bg-navy text-on-navy hover:bg-navy-deep disabled:opacity-50"
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
