import { useCallback, useEffect, useMemo, useState } from "react";
import {
  listStages, listCompanies, listJobs, listApplications, moveApplication, exportExcel,
} from "./api";
import { save } from "@tauri-apps/plugin-dialog";
import { openPath } from "@tauri-apps/plugin-opener";
import type { Stage, Company, Job, Application } from "./types";
import { CATEGORIES } from "./types";
import { useI18n } from "./i18n";
import KanbanBoard from "./components/KanbanBoard";
import ListView from "./components/ListView";
import DashboardView from "./components/DashboardView";
import ReviewBankView from "./components/ReviewBankView";
import ResumeView from "./components/ResumeView";
import ApplicationModal from "./components/ApplicationModal";
import ManagementPanel from "./components/ManagementPanel";
import InterviewPanel from "./components/InterviewPanel";
import SettingsPanel from "./components/SettingsPanel";

type AppModalState = { mode: "create" } | { mode: "edit"; app: Application } | null;
type NavKey = "board" | "list" | "dashboard" | "review" | "resume";

const navBtn = (active: boolean) =>
  "w-full text-left px-3 py-2 text-sm rounded-md " +
  (active ? "bg-neutral-100 text-neutral-900 font-medium" : "text-neutral-600 hover:bg-neutral-50");

export default function App() {
  const { t, lang, setLang } = useI18n();
  const [stages, setStages] = useState<Stage[]>([]);
  const [companies, setCompanies] = useState<Company[]>([]);
  const [jobs, setJobs] = useState<Job[]>([]);
  const [applications, setApplications] = useState<Application[]>([]);
  const [loading, setLoading] = useState(true);
  const [nav, setNav] = useState<NavKey>("board");
  const [query, setQuery] = useState("");
  const [catFilter, setCatFilter] = useState("");
  const [appModal, setAppModal] = useState<AppModalState>(null);
  const [showManage, setShowManage] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [interviewApp, setInterviewApp] = useState<Application | null>(null);
  const [exporting, setExporting] = useState(false);

  const handleExport = async () => {
    try {
      const p = await save({
        title: t("导出为 Excel"),
        defaultPath: "OfferFlow-Applications.xlsx",
        filters: [{ name: "Excel", extensions: ["xlsx"] }],
      });
      if (!p) return;
      const path = p.toLowerCase().endsWith(".xlsx") ? p : p + ".xlsx";
      setExporting(true);
      await exportExcel(path);
      await openPath(path);
    } catch (e) {
      alert(t("导出失败：") + String(e));
    } finally {
      setExporting(false);
    }
  };

  const refresh = useCallback(async () => {
    const [s, c, j, a] = await Promise.all([
      listStages(), listCompanies(), listJobs(null), listApplications(),
    ]);
    setStages(s);
    setCompanies(c);
    setJobs(j);
    setApplications(a);
  }, []);

  useEffect(() => {
    refresh().finally(() => setLoading(false));
  }, [refresh]);

  const jobMap = useMemo(
    () => new Map<number, Job>(jobs.map((j): [number, Job] => [j.id, j])),
    [jobs],
  );
  const companyMap = useMemo(
    () => new Map<number, Company>(companies.map((c): [number, Company] => [c.id, c])),
    [companies],
  );
  const stageMap = useMemo(
    () => new Map<number, string>(stages.map((s): [number, string] => [s.id, s.name])),
    [stages],
  );

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    return applications.filter((a) => {
      const job = jobMap.get(a.job_id);
      const company = companyMap.get(job?.company_id ?? -1);
      if (catFilter && job?.category !== catFilter) return false;
      if (!q) return true;
      const hay = [
        company?.name, job?.title, job?.department, job?.location,
        job?.category, a.channel, a.status,
      ]
        .filter(Boolean)
        .join(" ")
        .toLowerCase();
      return hay.includes(q);
    });
  }, [applications, jobMap, companyMap, query, catFilter]);

  const handleMove = useCallback(async (appId: number, stageId: number) => {
    const updated = await moveApplication(appId, stageId);
    setApplications((prev) => prev.map((a) => (a.id === updated.id ? updated : a)));
  }, []);

  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center text-neutral-500">
        {t("加载中…")}
      </div>
    );
  }

  const activeJob = interviewApp ? jobMap.get(interviewApp.job_id) : undefined;
  const activeCompany = activeJob ? companyMap.get(activeJob.company_id) : undefined;

  return (
    <div className="min-h-screen bg-neutral-100 text-neutral-800 flex">
      <aside className="w-56 shrink-0 bg-white border-r border-neutral-200 flex flex-col h-screen sticky top-0">
        <div className="px-5 py-4 border-b border-neutral-200 flex items-center justify-between">
          <h1 className="text-lg font-bold tracking-tight">OfferFlow</h1>
          <button
            onClick={() => setLang(lang === "zh" ? "en" : "zh")}
            className="text-xs px-2 py-1 rounded border border-neutral-300 text-neutral-600 hover:bg-neutral-50"
            title={lang === "zh" ? "Switch to English" : "切换为中文"}
          >
            {lang === "zh" ? "EN" : "中"}
          </button>
        </div>
        <div className="p-3">
          <button
            onClick={() => setAppModal({ mode: "create" })}
            className="w-full px-3 py-2 text-sm rounded-md bg-neutral-900 text-white hover:bg-neutral-800"
          >
            {t("+ 新建投递")}
          </button>
        </div>
        <nav className="flex-1 overflow-y-auto px-3 space-y-1">
          <div className="px-3 pt-2 pb-1 text-xs text-neutral-400">{t("视图")}</div>
          <button className={navBtn(nav === "board")} onClick={() => setNav("board")}>{t("全部看板")}</button>
          <button className={navBtn(nav === "list")} onClick={() => setNav("list")}>{t("列表明细")}</button>
          <button className={navBtn(nav === "dashboard")} onClick={() => setNav("dashboard")}>{t("数据看板")}</button>
          <div className="px-3 pt-4 pb-1 text-xs text-neutral-400">{t("沉淀")}</div>
          <button className={navBtn(nav === "review")} onClick={() => setNav("review")}>{t("复盘题库")}</button>
          <button className={navBtn(nav === "resume")} onClick={() => setNav("resume")}>{t("简历版本")}</button>
        </nav>
        <div className="p-3 border-t border-neutral-200 space-y-1">
          <button className={navBtn(false)} onClick={() => setShowSettings(true)}>{t("系统设置")}</button>
          <button className={navBtn(false)} onClick={() => setShowManage(true)}>{t("公司与岗位")}</button>
          <button className={navBtn(false)} onClick={handleExport} disabled={exporting}>
            {exporting ? t("导出中…") : t("导出 Excel")}
          </button>
        </div>
      </aside>

      <div className="flex-1 flex flex-col min-w-0">
        <header className="bg-white border-b border-neutral-200 px-6 py-3 flex items-center gap-3 sticky top-0 z-10">
          <input
            className="flex-1 max-w-md rounded-md border border-neutral-300 px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-neutral-400 bg-white"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder={t("搜索公司 / 岗位 / 部门 / 城市 / 分类 / 渠道")}
          />
          <select
            className="rounded-md border border-neutral-300 px-2 py-1.5 text-sm bg-white"
            value={catFilter}
            onChange={(e) => setCatFilter(e.target.value)}
          >
            <option value="">{t("全部分类")}</option>
            {CATEGORIES.map((c) => (
              <option key={c} value={c}>{t(c)}</option>
            ))}
          </select>
          <button
            onClick={() => setAppModal({ mode: "create" })}
            className="px-3 py-1.5 text-sm rounded-md border border-neutral-300 hover:bg-neutral-50 whitespace-nowrap"
          >
            {t("快速粘贴链接")}
          </button>
          <div className="flex rounded-md border border-neutral-300 overflow-hidden">
            <button
              onClick={() => setNav("board")}
              className={"px-3 py-1.5 text-sm " + (nav === "board" ? "bg-neutral-900 text-white" : "hover:bg-neutral-50")}
            >
              {t("看板")}
            </button>
            <button
              onClick={() => setNav("list")}
              className={"px-3 py-1.5 text-sm " + (nav === "list" ? "bg-neutral-900 text-white" : "hover:bg-neutral-50")}
            >
              {t("列表")}
            </button>
          </div>
        </header>

        <main className="flex-1 overflow-auto p-6">
          {nav === "board" && (
            <KanbanBoard
              stages={stages}
              applications={filtered}
              jobMap={jobMap}
              companyMap={companyMap}
              onMove={handleMove}
              onOpen={(app) => setAppModal({ mode: "edit", app })}
            />
          )}
          {nav === "list" && (
            <ListView
              applications={filtered}
              jobMap={jobMap}
              companyMap={companyMap}
              stageMap={stageMap}
              onOpen={(app) => setAppModal({ mode: "edit", app })}
            />
          )}
          {nav === "dashboard" && (
            <DashboardView
              applications={applications}
              jobMap={jobMap}
              companyMap={companyMap}
              stageMap={stageMap}
            />
          )}
          {nav === "review" && <ReviewBankView />}
          {nav === "resume" && <ResumeView onChanged={refresh} />}
        </main>
      </div>

      {appModal && (
        <ApplicationModal
          state={appModal}
          stages={stages}
          companies={companies}
          jobs={jobs}
          onClose={() => setAppModal(null)}
          onSaved={async () => {
            await refresh();
            setAppModal(null);
          }}
          onOpenInterview={(app) => {
            setAppModal(null);
            setInterviewApp(app);
          }}
        />
      )}

      {interviewApp && (
        <InterviewPanel
          app={interviewApp}
          jobTitle={activeJob?.title ?? ""}
          companyName={activeCompany?.name ?? ""}
          onClose={() => setInterviewApp(null)}
        />
      )}

      {showManage && (
        <ManagementPanel
          stages={stages}
          companies={companies}
          jobs={jobs}
          onClose={() => setShowManage(false)}
          onChanged={refresh}
        />
      )}

      {showSettings && <SettingsPanel onClose={() => setShowSettings(false)} />}
    </div>
  );
}