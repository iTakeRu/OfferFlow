import { useState } from "react";
import {
  createCompany, deleteCompany, createJob, deleteJob,
  createStage, updateStage, deleteStage, reorderStages,
  fetchJdText, extractJdInfo,
} from "../api";
import { CATEGORIES } from "../types";
import type { Stage, Company, Job } from "../types";
import { useI18n } from "../i18n";

interface Props {
  stages: Stage[];
  companies: Company[];
  jobs: Job[];
  onClose: () => void;
  onChanged: () => void;
}

const field = "w-full rounded-md border border-neutral-300 px-2.5 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-neutral-400 bg-white";
const btn = "px-3 py-1.5 text-sm rounded-md";
const label = "block text-xs font-medium text-neutral-500 mb-1";
const danger = "text-red-500 hover:text-red-600 text-sm";

type Tab = "stage" | "company" | "job";

export default function ManagementPanel({ stages, companies, jobs, onClose, onChanged }: Props) {
  const { t } = useI18n();
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

  // job state
  const [jobCompanyId, setJobCompanyId] = useState<number | "">("");
  const [jTitle, setJTitle] = useState("");
  const [jDepartment, setJDepartment] = useState("");
  const [jLocation, setJLocation] = useState("");
  const [jJdUrl, setJJdUrl] = useState("");
  const [jJdText, setJJdText] = useState("");
  const [jSalaryMin, setJSalaryMin] = useState("");
  const [jSalaryMax, setJSalaryMax] = useState("");
  const [jCategory, setJCategory] = useState("");
  const [jCompanyHint, setJCompanyHint] = useState("");
  const [fetching, setFetching] = useState(false);
  const [extracting, setExtracting] = useState(false);

  const run = async (fn: () => Promise<unknown>) => {
    setError("");
    try {
      await fn();
      await onChanged();
    } catch (e) {
      setError(String(e));
    }
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

  const handleFetch = async () => {
    if (!jJdUrl.trim()) return;
    setFetching(true);
    setError("");
    try {
      const text = await fetchJdText(jJdUrl.trim());
      setJJdText(text);
    } catch (e) {
      setError(String(e));
    } finally {
      setFetching(false);
    }
  };

  const handleExtract = async () => {
    if (!jJdText.trim()) return;
    setExtracting(true);
    setError("");
    try {
      const r = await extractJdInfo(jJdText);
      if (r.title) setJTitle(r.title);
      if (r.company) setJCompanyHint(r.company);
      if (r.department) setJDepartment(r.department);
      if (r.location) setJLocation(r.location);
      if (r.salaryMin != null) setJSalaryMin(String(r.salaryMin));
      if (r.salaryMax != null) setJSalaryMax(String(r.salaryMax));
      if (r.category) setJCategory(r.category);
    } catch (e) {
      setError(String(e));
    } finally {
      setExtracting(false);
    }
  };

  const addJob = () => {
    if (jobCompanyId === "" || !jTitle.trim()) return;
    run(async () => {
      await createJob({
        companyId: Number(jobCompanyId),
        title: jTitle.trim(),
        department: jDepartment || null,
        location: jLocation || null,
        category: jCategory || null,
        jdUrl: jJdUrl || null,
        jdText: jJdText || null,
        salaryMin: jSalaryMin ? Number(jSalaryMin) : null,
        salaryMax: jSalaryMax ? Number(jSalaryMax) : null,
      });
      setJTitle(""); setJDepartment(""); setJLocation(""); setJCategory("");
      setJJdUrl(""); setJJdText(""); setJSalaryMin(""); setJSalaryMax(""); setJCompanyHint("");
    });
  };

  const tabs: { key: Tab; label: string }[] = [
    { key: "stage", label: t("阶段") },
    { key: "company", label: t("公司") },
    { key: "job", label: t("岗位") },
  ];

  return (
    <div className="fixed inset-0 bg-black/40 flex items-center justify-center z-20 p-4">
      <div className="bg-white rounded-xl shadow-xl w-full max-w-2xl max-h-[85vh] flex flex-col">
        <div className="flex items-center justify-between px-6 pt-5 pb-3 border-b border-neutral-200">
          <div className="flex gap-2">
            {tabs.map((tb) => (
              <button
                key={tb.key}
                onClick={() => setTab(tb.key)}
                className={`px-3 py-1.5 text-sm rounded-md ${
                  tab === tb.key ? "bg-neutral-900 text-white" : "bg-neutral-100 hover:bg-neutral-200"
                }`}
              >
                {tb.label}
              </button>
            ))}
          </div>
          <button onClick={onClose} className="text-neutral-400 hover:text-neutral-600 text-xl leading-none">&times;</button>
        </div>

        <div className="flex-1 overflow-y-auto px-6 py-4">
          {error && <div className="mb-3 text-sm text-red-600 bg-red-50 rounded-md px-3 py-2">{error}</div>}

          {tab === "stage" && (
            <div className="space-y-3">
              <div className="flex gap-2">
                <input className={field} value={newStage} onChange={(e) => setNewStage(e.target.value)} placeholder={t("新增阶段名称")} onKeyDown={(e) => e.key === "Enter" && addStage()} />
                <button onClick={addStage} className={`${btn} bg-neutral-900 text-white hover:bg-neutral-800`}>{t("添加")}</button>
              </div>
              <ul className="divide-y divide-neutral-100">
                {stages.map((s, idx) => (
                  <li key={s.id} className="flex items-center justify-between py-2">
                    {editingStageId === s.id ? (
                      <div className="flex-1 flex gap-2">
                        <input className={field} value={editingStageName} onChange={(e) => setEditingStageName(e.target.value)} />
                        <button onClick={() => saveStage(s.id)} className={`${btn} bg-neutral-900 text-white`}>{t("保存")}</button>
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
                      <button className={danger} onClick={() => run(() => deleteStage(s.id))}>{t("删除")}</button>
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
                <button onClick={addCompany} className={`${btn} bg-neutral-900 text-white hover:bg-neutral-800`}>{t("添加公司")}</button>
              </div>
              <ul className="divide-y divide-neutral-100">
                {companies.map((c) => (
                  <li key={c.id} className="flex items-center justify-between py-2">
                    <div>
                      <span className="text-sm font-medium">{c.name}</span>
                      {c.city && <span className="text-xs text-neutral-400 ml-2">{c.city}</span>}
                      {c.industry && <span className="text-xs text-neutral-400 ml-2">{c.industry}</span>}
                    </div>
                    <button className={danger} onClick={() => run(() => deleteCompany(c.id))}>{t("删除")}</button>
                  </li>
                ))}
                {companies.length === 0 && <li className="text-sm text-neutral-400 py-3">{t("暂无公司")}</li>}
              </ul>
            </div>
          )}

          {tab === "job" && (
            <div className="space-y-4">
              <div>
                <label className={label}>{t("URL 抓取 / 信息提取")}</label>
                <div className="flex gap-2 mb-2">
                  <input className={field} value={jJdUrl} onChange={(e) => setJJdUrl(e.target.value)} placeholder={t("粘贴职位链接，点击抓取")} />
                  <button onClick={handleFetch} disabled={fetching} className={`${btn} border border-neutral-300 hover:bg-neutral-50 disabled:opacity-50 whitespace-nowrap`}>
                    {fetching ? t("抓取中…") : t("抓取")}
                  </button>
                </div>
                <textarea
                  className={field}
                  rows={4}
                  value={jJdText}
                  onChange={(e) => setJJdText(e.target.value)}
                  placeholder={t("JD 描述文本（可手动粘贴，或从链接抓取）")}
                />
                <div className="flex justify-end mt-2">
                  <button onClick={handleExtract} disabled={extracting} className={`${btn} border border-neutral-300 hover:bg-neutral-50 disabled:opacity-50`}>
                    {extracting ? t("提取中…") : t("提取信息填充")}
                  </button>
                </div>
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
                <select className={field} value={jCategory} onChange={(e) => setJCategory(e.target.value)}>
                  <option value="">{t("选择分类")}</option>
                  {CATEGORIES.map((ct) => <option key={ct} value={ct}>{t(ct)}</option>)}
                </select>
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
                <button onClick={addJob} className={`${btn} bg-neutral-900 text-white hover:bg-neutral-800`}>{t("添加岗位")}</button>
              </div>

              <div>
                <div className="text-xs font-medium text-neutral-500 mb-1">{t("岗位列表")}</div>
                <ul className="divide-y divide-neutral-100">
                  {jobs.filter((j) => jobCompanyId !== "" && j.company_id === Number(jobCompanyId)).map((j) => (
                    <li key={j.id} className="flex items-center justify-between py-2">
                      <div>
                        <span className="text-sm">{j.title}</span>
                        {j.department && <span className="text-xs text-neutral-400 ml-2">{j.department}</span>}
                        {j.category && <span className="text-xs text-neutral-400 ml-2">{t(j.category)}</span>}
                        {j.salary_min != null && (
                          <span className="text-xs text-neutral-400 ml-2">
                            {j.salary_min}{j.salary_max != null ? `-${j.salary_max}` : ""}k
                          </span>
                        )}
                      </div>
                      <button className={danger} onClick={() => run(() => deleteJob(j.id))}>{t("删除")}</button>
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
  );
}