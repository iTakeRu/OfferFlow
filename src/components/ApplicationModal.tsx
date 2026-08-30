import { useMemo, useState } from "react";
import {
  createApplication, updateApplication, createCompany, createJob,
  fetchJobDetail, fetchJdText, extractJdInfo,
} from "../api";
import { APPLICATION_STATUS } from "../types";
import type { Stage, Company, Job, Application } from "../types";
import { useI18n } from "../i18n";
import ActivityTimeline from "./ActivityTimeline";

interface Props {
  state: { mode: "create" } | { mode: "edit"; app: Application };
  stages: Stage[];
  companies: Company[];
  jobs: Job[];
  onClose: () => void;
  onSaved: () => void;
  onOpenInterview?: (app: Application) => void;
}

const field = "w-full rounded-md border border-neutral-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-neutral-400 bg-white";
const label = "block text-xs font-medium text-neutral-500 mb-1";

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

  // URL 解析
  const [url, setUrl] = useState("");
  const [fetching, setFetching] = useState(false);

  // 本地列表副本：解析时新建的公司/岗位要即时可见
  const [companyList, setCompanyList] = useState<Company[]>(companies);
  const [jobList, setJobList] = useState<Job[]>(jobs);

  const companyJobs = useMemo(
    () => jobList.filter((j) => j.company_id === (companyId === "" ? -1 : companyId)),
    [jobList, companyId],
  );

  const handleParseUrl = async () => {
    const u = url.trim();
    if (!u) {
      setError(t("请先粘贴岗位链接"));
      return;
    }
    setFetching(true);
    setError("");
    try {
      let title = "";
      let companyName = "";
      let jdText: string | null = null;
      let department: string | null = null;
      let location: string | null = null;
      let salaryMin: number | null = null;
      let salaryMax: number | null = null;
      let category: string | null = null;

      const supported = ["mokahr.com", "lever.co", "greenhouse.io"].some((h) => u.includes(h));
      if (supported) {
        const d = await fetchJobDetail(u);
        title = d.title;
        companyName = d.company;
        jdText = d.jdText;
        department = d.department;
        location = d.location;
        salaryMin = d.salaryMin;
        salaryMax = d.salaryMax;
        category = d.category;
      } else {
        const text = await fetchJdText(u);
        const r = await extractJdInfo(text);
        title = r.title ?? "";
        companyName = r.company ?? "";
        department = r.department;
        location = r.location;
        salaryMin = r.salaryMin;
        salaryMax = r.salaryMax;
        category = r.category;
      }

      if (!title) {
        setError(t("未能解析出岗位名称"));
        return;
      }

      let company = companyList.find((c) => c.name === companyName);
      if (!company && companyName) {
        company = await createCompany({ name: companyName });
        setCompanyList((prev) => [...prev, company!]);
      }

      let job = company
        ? jobList.find((j) => j.company_id === company!.id && j.title === title)
        : undefined;
      if (!job && company) {
        job = await createJob({
          companyId: company.id,
          title,
          department,
          location,
          category,
          jdUrl: u,
          jdText,
          salaryMin,
          salaryMax,
        });
        setJobList((prev) => [...prev, job!]);
      }

      if (!company || !job) {
        setError(t("未能创建公司与岗位记录"));
        return;
      }

      setCompanyId(company.id);
      setJobId(job.id);
      if (!channel) setChannel("官网");
    } catch (e) {
      setError(String(e));
    } finally {
      setFetching(false);
    }
  };

  const handleSubmit = async () => {
    if (jobId === "") {
      setError(t("请选择公司与岗位"));
      return;
    }
    setSaving(true);
    setError("");
    try {
      const payload = {
        jobId: Number(jobId),
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
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/40 flex items-center justify-center z-20 p-4">
      <div className="bg-white rounded-xl shadow-xl w-full max-w-lg max-h-[90vh] overflow-y-auto p-6">
        <h2 className="text-lg font-semibold mb-4">{state.mode === "edit" ? t("编辑投递") : t("新建投递")}</h2>

        {state.mode === "create" && (
          <div className="mb-4">
            <label className={label}>{t("从链接解析")}</label>
            <div className="flex gap-2">
              <input
                className={field}
                value={url}
                onChange={(e) => setUrl(e.target.value)}
                placeholder={t("粘贴岗位链接，如 MokaHR / 官网 / Boss 链接")}
                onKeyDown={(e) => {
                  if (e.key === "Enter") handleParseUrl();
                }}
              />
              <button
                type="button"
                onClick={handleParseUrl}
                disabled={fetching}
                className="px-4 py-2 text-sm rounded-md bg-neutral-700 text-white hover:bg-neutral-800 disabled:opacity-50 whitespace-nowrap"
              >
                {fetching ? t("解析中…") : t("解析")}
              </button>
            </div>
            <p className="mt-1 text-xs text-neutral-400">{t("解析后自动匹配或新建公司与岗位")}</p>
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
              <option value="">{t("选择公司")}</option>
              {companyList.map((c) => (
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
              <option value="">{t("选择岗位")}</option>
              {companyJobs.map((j) => (
                <option key={j.id} value={j.id}>{j.title}</option>
              ))}
            </select>
          </div>
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
            <input type="date" className={field} value={appliedAt} onChange={(e) => setAppliedAt(e.target.value)} />
          </div>
        </div>

        <div className="mt-4">
          <label className={label}>{t("备注")}</label>
          <textarea className={`${field} resize-none`} rows={3} value={notes} onChange={(e) => setNotes(e.target.value)} placeholder={t("可选")} />
        </div>

        {state.mode === "edit" && <ActivityTimeline applicationId={app!.id} />}

        {error && <div className="mt-3 text-sm text-red-600">{error}</div>}

        <div className="mt-6 flex justify-between items-center gap-2">
          <div>
            {state.mode === "edit" && onOpenInterview && (
              <button
                onClick={() => onOpenInterview(app!)}
                className="px-4 py-2 text-sm rounded-md bg-neutral-100 hover:bg-neutral-200"
              >
                {t("面试复盘")}
              </button>
            )}
          </div>
          <div className="flex gap-2">
            <button onClick={onClose} className="px-4 py-2 text-sm rounded-md border border-neutral-300 hover:bg-neutral-50">
              {t("取消")}
            </button>
            <button
              onClick={handleSubmit}
              disabled={saving}
              className="px-4 py-2 text-sm rounded-md bg-neutral-900 text-white hover:bg-neutral-800 disabled:opacity-50"
            >
              {saving ? t("保存中…") : t("保存")}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}