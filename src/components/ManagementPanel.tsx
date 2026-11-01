import { useRef, useState } from "react";
import {
  createCompany, updateCompany, deleteCompany, createJob, updateJob, deleteJob,
  createStage, updateStage, deleteStage, reorderStages,
  saveJobSnapshot, discardSnapshotDraft,
} from "../api";
import { categoryOptions } from "../types";
import CategoryInput from "./CategoryInput";
import type { Stage, Company, Job, JobFields, SnapshotDraft } from "../types";
import { useI18n } from "../i18n";
import { confirm as askConfirm } from "@tauri-apps/plugin-dialog";
import { useModalFocus } from "../useModalFocus";
import SnapshotCapture from "./SnapshotCapture";
import SnapshotViewer from "./SnapshotViewer";

interface Props {
  stages: Stage[];
  companies: Company[];
  jobs: Job[];
  onClose: () => void;
  onChanged: () => void;
}

const field = "w-full rounded-md border border-neutral-300 px-2.5 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-navy bg-white";
const btn = "px-3 py-1.5 text-sm rounded-md";
const label = "block text-xs font-medium text-neutral-500 mb-1";
const danger = "text-red-500 hover:text-red-600 text-sm";

type Tab = "stage" | "company" | "job";

export default function ManagementPanel({ stages, companies, jobs, onClose, onChanged }: Props) {
  const { t } = useI18n();
  // 关窗时丢掉还没存的快照草稿（token 在下方 state 里，用函数取最新值）
  const draftTokenRef = useRef<string | null>(null);
  const close = () => {
    if (draftTokenRef.current) discardSnapshotDraft(draftTokenRef.current).catch(() => {});
    onClose();
  };
  const modal = useModalFocus(close);
  const [tab, setTab] = useState<Tab>("stage");
  const [error, setError] = useState("");

  // stage state
  const [newStage, setNewStage] = useState("");
  const [editingStageId, setEditingStageId] = useState<number | null>(null);
  const [editingStageName, setEditingStageName] = useState("");

  // company state
  const [cName, setCName] = useState("");
  const [cIndustry, setCIndustry] = useState("");
  const [cCity, setCCity] = useState("");
  const [editingCompanyId, setEditingCompanyId] = useState<number | null>(null);
  const [companyDraft, setCompanyDraft] = useState({ name: "", industry: "", city: "" });

  // 编辑岗位：解析出来的公司 / 岗位以前只能删（还会连带删掉投递），改不了
  const [editingJobId, setEditingJobId] = useState<number | null>(null);
  const [jobDraft, setJobDraft] = useState({ title: "", department: "", location: "", category: "" });

  // job state
  const [jobCompanyId, setJobCompanyId] = useState<number | "">("");
  const [jTitle, setJTitle] = useState("");
  const [jDepartment, setJDepartment] = useState("");
  const [jLocation, setJLocation] = useState("");
  const [jDraft, setJDraft] = useState<SnapshotDraft | null>(null);
  const [captureKey, setCaptureKey] = useState(0);
  const [viewingJob, setViewingJob] = useState<Job | null>(null);
  const [jSalaryMin, setJSalaryMin] = useState("");
  const [jSalaryMax, setJSalaryMax] = useState("");
  const [jCategory, setJCategory] = useState("");
  const [jCompanyHint, setJCompanyHint] = useState("");

  const run = async (fn: () => Promise<unknown>) => {
    setError("");
    try {
      await fn();
      await onChanged();
    } catch (e) {
      setError(String(e));
    }
  };

  const confirmDelete = async (message: string, fn: () => Promise<unknown>) => {
    const ok = await askConfirm(message, { title: t("删除确认"), kind: "warning" });
    if (ok) await run(fn);
  };

  const moveStage = (idx: number, dir: -1 | 1) => {
    const target = idx + dir;
    if (target < 0 || target >= stages.length) return;
    const next = [...stages];
    [next[idx], next[target]] = [next[target], next[idx]];
    run(() => reorderStages(next.map((s) => s.id)));
  };

  const addStage = () => {
    if (!newStage.trim()) return;
    run(async () => { await createStage(newStage.trim()); setNewStage(""); });
  };

  const saveStage = (id: number) => {
    if (!editingStageName.trim()) return;
    run(async () => { await updateStage(id, editingStageName.trim()); setEditingStageId(null); });
  };

  const addCompany = () => {
    if (!cName.trim()) return;
    run(async () => {
      await createCompany({ name: cName.trim(), industry: cIndustry || null, city: cCity || null });
      setCName(""); setCIndustry(""); setCCity("");
    });
  };

  const startEditCompany = (c: Company) => {
    setEditingCompanyId(c.id);
    setCompanyDraft({ name: c.name, industry: c.industry ?? "", city: c.city ?? "" });
  };

  const saveCompany = (c: Company) => {
    if (!companyDraft.name.trim()) return;
    run(async () => {
      await updateCompany(c.id, {
        name: companyDraft.name.trim(),
        industry: companyDraft.industry || null,
        city: companyDraft.city || null,
        website: c.website,
        notes: c.notes,
      });
      setEditingCompanyId(null);
    });
  };

  const startEditJob = (j: Job) => {
    setEditingJobId(j.id);
    setJobDraft({
      title: j.title,
      department: j.department ?? "",
      location: j.location ?? "",
      category: j.category ?? "",
    });
  };

  const saveJob = (j: Job) => {
    if (!jobDraft.title.trim()) return;
    run(async () => {
      await updateJob(j.id, {
        companyId: j.company_id,
        title: jobDraft.title.trim(),
        department: jobDraft.department || null,
        location: jobDraft.location || null,
        category: jobDraft.category || null,
        jdUrl: j.jd_url,
        jdText: j.jd_text,
        salaryMin: j.salary_min,
        salaryMax: j.salary_max,
      });
      setEditingJobId(null);
    });
  };

  const setDraft = (next: SnapshotDraft | null) => {
    draftTokenRef.current = next?.token ?? null;
    setJDraft(next);
  };

  /**
   * 只填识别到、且用户还空着的字段，不覆盖已经手动填好的内容。
   * 用函数式更新读最新值：AI 提取要等好几秒，期间用户可能还在填表。
   */
  const applyFields = (f: JobFields) => {
    const fill = (next: string) => (current: string) => (!current.trim() && next ? next : current);
    setJTitle(fill(f.title));
    setJDepartment(fill(f.department));
    setJLocation(fill(f.location));
    setJCategory(fill(f.category));
    setJSalaryMin(fill(f.salaryMin));
    setJSalaryMax(fill(f.salaryMax));
    if (!f.companyName) return;
    const known = companies.find(
      (c) => c.name.trim().toLocaleLowerCase() === f.companyName.trim().toLocaleLowerCase(),
    );
    setJobCompanyId((current) => (current === "" && known ? known.id : current));
    setJCompanyHint(known ? "" : f.companyName);
  };

  const handleCaptured = (draft: SnapshotDraft, f: JobFields) => {
    if (jDraft?.token && jDraft.token !== draft.token) discardSnapshotDraft(jDraft.token).catch(() => {});
    setDraft(draft);
    applyFields(f);
  };

  const addJob = () => {
    if (jobCompanyId === "" || !jTitle.trim()) return;
    run(async () => {
      const job = await createJob({
        companyId: Number(jobCompanyId),
        title: jTitle.trim(),
        department: jDepartment || null,
        location: jLocation || null,
        category: jCategory || null,
        jdUrl: jDraft?.sourceUrl ?? null,
        jdText: null,
        salaryMin: jSalaryMin ? Number(jSalaryMin) : null,
        salaryMax: jSalaryMax ? Number(jSalaryMax) : null,
      });
      if (jDraft) await saveJobSnapshot(job.id, jDraft);
      setDraft(null);
      setCaptureKey((k) => k + 1);
      setJTitle(""); setJDepartment(""); setJLocation(""); setJCategory("");
      setJSalaryMin(""); setJSalaryMax(""); setJCompanyHint("");
    });
  };

  const tabs: { key: Tab; label: string }[] = [
    { key: "stage", label: t("阶段") },
    { key: "company", label: t("公司") },
    { key: "job", label: t("岗位") },
  ];

  return (
    <>
    <div className="fixed inset-0 bg-scrim flex items-center justify-center z-20 p-4">
      <div ref={modal.ref} onKeyDown={modal.onKeyDown} role="dialog" aria-modal="true" aria-label={t("公司、岗位与阶段管理")} tabIndex={-1} className="bg-white rounded-2xl shadow-xl border-2 border-rule w-full max-w-2xl max-h-[85vh] flex flex-col">
        <div className="flex items-center justify-between px-6 pt-5 pb-3 border-b border-neutral-200">
          <div className="flex gap-2">
            {tabs.map((tb) => (
              <button
                key={tb.key}
                onClick={() => setTab(tb.key)}
                className={`px-3 py-1.5 text-sm rounded-md ${
                  tab === tb.key ? "bg-navy text-on-navy" : "bg-neutral-100 hover:bg-neutral-200"
                }`}
              >
                {tb.label}
              </button>
            ))}
          </div>
          <button onClick={close} aria-label={t("关闭")} className="text-neutral-400 hover:text-neutral-600 text-xl leading-none">&times;</button>
        </div>

        <div className="flex-1 overflow-y-auto px-6 py-4">
          {error && <div className="mb-3 text-sm text-red-600 bg-red-50 rounded-md px-3 py-2">{error}</div>}

          {tab === "stage" && (
            <div className="space-y-3">
              <div className="flex gap-2">
                <input className={field} value={newStage} onChange={(e) => setNewStage(e.target.value)} placeholder={t("新增阶段名称")} onKeyDown={(e) => e.key === "Enter" && addStage()} />
                <button onClick={addStage} className={`${btn} bg-navy text-on-navy hover:bg-navy-deep`}>{t("添加")}</button>
              </div>
              <ul className="divide-y divide-neutral-100">
                {stages.map((s, idx) => (
                  <li key={s.id} className="flex items-center justify-between py-2">
                    {editingStageId === s.id ? (
                      <div className="flex-1 flex gap-2">
                        <input className={field} value={editingStageName} onChange={(e) => setEditingStageName(e.target.value)} />
                        <button onClick={() => saveStage(s.id)} className={`${btn} bg-navy text-on-navy`}>{t("保存")}</button>
                      </div>
                    ) : (
                      <span className="text-sm">{t(s.name)}</span>
                    )}
                    <div className="flex items-center gap-3">
                      <button className="text-neutral-400 hover:text-neutral-600 text-sm disabled:opacity-30" disabled={idx === 0} onClick={() => moveStage(idx, -1)}>↑</button>
                      <button className="text-neutral-400 hover:text-neutral-600 text-sm disabled:opacity-30" disabled={idx === stages.length - 1} onClick={() => moveStage(idx, 1)}>↓</button>
                      <button
                        className="text-neutral-400 hover:text-neutral-600 text-sm"
                        onClick={() => { setEditingStageId(s.id); setEditingStageName(s.name); }}
                      >
                        {t("重命名")}
                      </button>
                      <button className={danger} onClick={() => confirmDelete(t("删除该阶段？仍被投递使用的阶段无法删除。"), () => deleteStage(s.id))}>{t("删除")}</button>
                    </div>
                  </li>
                ))}
              </ul>
            </div>
          )}

          {tab === "company" && (
            <div className="space-y-3">
              <div className="grid grid-cols-3 gap-2">
                <input className={field} value={cName} onChange={(e) => setCName(e.target.value)} placeholder={t("公司名称 *")} />
                <input className={field} value={cIndustry} onChange={(e) => setCIndustry(e.target.value)} placeholder={t("行业（可选）")} />
                <input className={field} value={cCity} onChange={(e) => setCCity(e.target.value)} placeholder={t("城市（可选）")} />
              </div>
              <div className="flex justify-end">
                <button onClick={addCompany} className={`${btn} bg-navy text-on-navy hover:bg-navy-deep`}>{t("添加公司")}</button>
              </div>
              <ul className="divide-y divide-neutral-100">
                {companies.map((c) => (
                  <li key={c.id} className="flex items-center justify-between gap-2 py-2">
                    {editingCompanyId === c.id ? (
                      <>
                        <div className="flex-1 grid grid-cols-3 gap-2">
                          <input className={field} value={companyDraft.name} onChange={(e) => setCompanyDraft((d) => ({ ...d, name: e.target.value }))} placeholder={t("公司名称 *")} />
                          <input className={field} value={companyDraft.industry} onChange={(e) => setCompanyDraft((d) => ({ ...d, industry: e.target.value }))} placeholder={t("行业（可选）")} />
                          <input className={field} value={companyDraft.city} onChange={(e) => setCompanyDraft((d) => ({ ...d, city: e.target.value }))} placeholder={t("城市（可选）")} />
                        </div>
                        <div className="flex items-center gap-2 shrink-0">
                          <button className={`${btn} bg-navy text-on-navy`} onClick={() => saveCompany(c)}>{t("保存")}</button>
                          <button className={`${btn} border border-neutral-300`} onClick={() => setEditingCompanyId(null)}>{t("取消")}</button>
                        </div>
                      </>
                    ) : (
                      <>
                        <div>
                          <span className="text-sm font-medium">{c.name}</span>
                          {c.city && <span className="text-xs text-neutral-400 ml-2">{c.city}</span>}
                          {c.industry && <span className="text-xs text-neutral-400 ml-2">{c.industry}</span>}
                        </div>
                        <div className="flex items-center gap-3 shrink-0">
                          <button className="text-neutral-400 hover:text-neutral-600 text-sm" onClick={() => startEditCompany(c)}>{t("编辑")}</button>
                          <button className={danger} onClick={() => confirmDelete(t("删除公司会同时删除其岗位、投递、面试、附件和 JD 快照，确认继续？"), () => deleteCompany(c.id))}>{t("删除")}</button>
                        </div>
                      </>
                    )}
                  </li>
                ))}
                {companies.length === 0 && <li className="text-sm text-neutral-400 py-3">{t("暂无公司")}</li>}
              </ul>
            </div>
          )}

          {tab === "job" && (
            <div className="space-y-4">
              <div>
                <label className={label}>{t("JD 快照（可选）")}</label>
                <SnapshotCapture key={captureKey} onCaptured={handleCaptured} onExtracted={applyFields} />
                {jDraft && (
                  <p className="mt-1 text-xs text-neutral-400">
                    {t("快照共 {n} 字，添加岗位时会一并存档", { n: jDraft.text.length })}
                  </p>
                )}
              </div>

              <div>
                <label className={label}>{t("所属公司")}</label>
                <select className={field} value={jobCompanyId} onChange={(e) => setJobCompanyId(e.target.value === "" ? "" : Number(e.target.value))}>
                  <option value="">{t("选择公司")}</option>
                  {companies.map((c) => <option key={c.id} value={c.id}>{c.name}</option>)}
                </select>
                {jCompanyHint && (
                  <div className="text-xs text-neutral-700 mt-1">{t("识别公司：")}{jCompanyHint}{t("（请在上方选择或到「公司」页新增）")}</div>
                )}
              </div>

              <div className="grid grid-cols-3 gap-2">
                <input className={field} value={jTitle} onChange={(e) => setJTitle(e.target.value)} placeholder={t("岗位名称 *")} />
                <input className={field} value={jDepartment} onChange={(e) => setJDepartment(e.target.value)} placeholder={t("部门（可选）")} />
                <input className={field} value={jLocation} onChange={(e) => setJLocation(e.target.value)} placeholder={t("城市（可选）")} />
              </div>

              <div>
                <label className={label}>{t("岗位分类（可选）")}</label>
                <CategoryInput className={field} value={jCategory} onChange={setJCategory} options={categoryOptions(jobs)} />
              </div>

              <div className="grid grid-cols-2 gap-2">
                <div>
                  <label className={label}>{t("薪资下限（k/月，可选）")}</label>
                  <input className={field} type="number" value={jSalaryMin} onChange={(e) => setJSalaryMin(e.target.value)} placeholder={t("如 12")} />
                </div>
                <div>
                  <label className={label}>{t("薪资上限（k/月，可选）")}</label>
                  <input className={field} type="number" value={jSalaryMax} onChange={(e) => setJSalaryMax(e.target.value)} placeholder={t("如 20")} />
                </div>
              </div>

              <div className="flex justify-end">
                <button onClick={addJob} className={`${btn} bg-navy text-on-navy hover:bg-navy-deep`}>{t("添加岗位")}</button>
              </div>

              <div>
                <div className="text-xs font-medium text-neutral-500 mb-1">{t("岗位列表")}</div>
                <ul className="divide-y divide-neutral-100">
                  {jobs.filter((j) => jobCompanyId !== "" && j.company_id === Number(jobCompanyId)).map((j) => (
                    <li key={j.id} className="flex items-center justify-between gap-2 py-2">
                      {editingJobId === j.id ? (
                        <>
                          <div className="flex-1 grid grid-cols-2 gap-2">
                            <input className={field} value={jobDraft.title} onChange={(e) => setJobDraft((d) => ({ ...d, title: e.target.value }))} placeholder={t("岗位名称 *")} />
                            <input className={field} value={jobDraft.department} onChange={(e) => setJobDraft((d) => ({ ...d, department: e.target.value }))} placeholder={t("部门（可选）")} />
                            <input className={field} value={jobDraft.location} onChange={(e) => setJobDraft((d) => ({ ...d, location: e.target.value }))} placeholder={t("城市（可选）")} />
                            <CategoryInput className={field} value={jobDraft.category} onChange={(v) => setJobDraft((d) => ({ ...d, category: v }))} options={categoryOptions(jobs)} />
                          </div>
                          <div className="flex items-center gap-2 shrink-0">
                            <button className={`${btn} bg-navy text-on-navy`} onClick={() => saveJob(j)}>{t("保存")}</button>
                            <button className={`${btn} border border-neutral-300`} onClick={() => setEditingJobId(null)}>{t("取消")}</button>
                          </div>
                        </>
                      ) : (
                        <>
                          <div>
                            <span className="text-sm">{j.title}</span>
                            {j.department && <span className="text-xs text-neutral-400 ml-2">{j.department}</span>}
                            {j.location && <span className="text-xs text-neutral-400 ml-2">{j.location}</span>}
                            {j.category && <span className="text-xs text-neutral-400 ml-2">{t(j.category)}</span>}
                            {j.salary_min != null && (
                              <span className="text-xs text-neutral-400 ml-2">
                                {j.salary_min}{j.salary_max != null ? `-${j.salary_max}` : ""}k
                              </span>
                            )}
                          </div>
                          <div className="flex items-center gap-3 shrink-0">
                            <button className="text-neutral-400 hover:text-neutral-600 text-sm" onClick={() => setViewingJob(j)}>{t("快照")}</button>
                            <button className="text-neutral-400 hover:text-neutral-600 text-sm" onClick={() => startEditJob(j)}>{t("编辑")}</button>
                            <button className={danger} onClick={() => confirmDelete(t("删除岗位会同时删除相关投递、面试、附件和 JD 快照，确认继续？"), () => deleteJob(j.id))}>{t("删除")}</button>
                          </div>
                        </>
                      )}
                    </li>
                  ))}
                  {jobCompanyId === "" && <li className="text-sm text-neutral-400 py-3">{t("请先选择公司")}</li>}
                </ul>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
    {viewingJob && (
      <SnapshotViewer
        jobId={viewingJob.id}
        jobTitle={viewingJob.title}
        companyName={companies.find((c) => c.id === viewingJob.company_id)?.name ?? ""}
        onClose={() => setViewingJob(null)}
      />
    )}
    </>
  );
}
