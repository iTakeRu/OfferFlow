import { lazy, Suspense, useCallback, useEffect, useMemo, useState } from "react";
import {
  listStages, listCompanies, listJobs, listApplications, moveApplication, exportExcel,
} from "./api";
import { save, message } from "@tauri-apps/plugin-dialog";
import type { Stage, Company, Job, Application } from "./types";
import { CATEGORIES } from "./types";
import { useI18n } from "./i18n";
import KanbanBoard from "./components/KanbanBoard";
import ListView from "./components/ListView";
import ApplicationModal from "./components/ApplicationModal";
import appIcon from "../src-tauri/icons/128x128@2x.png";

const DashboardView = lazy(() => import("./components/DashboardView"));
const ReviewBankView = lazy(() => import("./components/ReviewBankView"));
const ResumeView = lazy(() => import("./components/ResumeView"));
const ManagementPanel = lazy(() => import("./components/ManagementPanel"));
const InterviewPanel = lazy(() => import("./components/InterviewPanel"));
const SettingsPanel = lazy(() => import("./components/SettingsPanel"));

type AppModalState = { mode: "create" } | { mode: "edit"; app: Application } | null;
type NavKey = "board" | "list" | "dashboard" | "review" | "resume";

type AppsView = "board" | "list";

// 每个栏目像杂志的一个版块：编号 + 大标题。
// 看板和列表只是「投递」的两种排法，共用一个栏目。
const VIEW_META: Record<NavKey, { no: string; title: string }> = {
  board: { no: "01", title: "投递" },
  list: { no: "01", title: "投递" },
  dashboard: { no: "02", title: "投递统计" },
  review: { no: "03", title: "面试复盘" },
  resume: { no: "04", title: "简历版本" },
};

const issueDate = () => {
  const d = new Date();
  return `${d.getFullYear()}.${String(d.getMonth() + 1).padStart(2, "0")}.${String(d.getDate()).padStart(2, "0")}`;
};

// 鼠标点击不留焦点框；键盘 Tab 时显示一条内嵌的赭石色焦点线，和直角风格一致
const navBtn = (active: boolean) =>
  "w-full text-left px-3 py-2 text-sm transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ochre " +
  (active ? "bg-paper text-navy font-bold" : "text-paper hover:bg-navy-soft");

const sectionLabel = "px-3 pt-5 pb-1.5 text-[11px] font-bold tracking-[0.22em] text-navy-muted";

export default function App() {
  const { t, lang, setLang } = useI18n();
  const [stages, setStages] = useState<Stage[]>([]);
  const [companies, setCompanies] = useState<Company[]>([]);
  const [jobs, setJobs] = useState<Job[]>([]);
  const [applications, setApplications] = useState<Application[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState("");
  const [nav, setNav] = useState<NavKey>("board");
  // 从别的栏目回到「投递」时，沿用上次选的看板 / 列表
  const [appsView, setAppsView] = useState<AppsView>("board");
  const showApps = (view: AppsView) => {
    setAppsView(view);
    setNav(view);
  };
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
      await message(t("导出完成：") + path, { title: t("导出 Excel"), kind: "info" });
    } catch (e) {
      await message(t("导出失败：") + String(e), { title: t("导出 Excel"), kind: "error" });
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
    refresh().catch((e) => setLoadError(String(e))).finally(() => setLoading(false));
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
  if (loadError) {
    return <div className="min-h-screen flex flex-col gap-4 items-center justify-center p-8 text-center">
      <div className="text-red-700 font-medium">{t("数据加载失败")}</div>
      <div className="text-sm text-neutral-600 max-w-xl break-words">{loadError}</div>
      <button className="px-4 py-2 bg-navy text-paper font-bold" onClick={() => { setLoading(true); setLoadError(""); refresh().catch((e) => setLoadError(String(e))).finally(() => setLoading(false)); }}>{t("重试")}</button>
    </div>;
  }

  const activeJob = interviewApp ? jobMap.get(interviewApp.job_id) : undefined;
  const activeCompany = activeJob ? companyMap.get(activeJob.company_id) : undefined;

  const meta = VIEW_META[nav];
  const inApps = nav === "board" || nav === "list";
  // 只有投递页有实际数字可说；其他栏目不放标语式的导语
  const deck = inApps
    ? t("{total} 份投递，{active} 份正在推进。", {
      total: filtered.length,
      active: filtered.filter((a) => a.status === "推进中").length,
    })
    : null;

  return (
    <div className="min-h-screen bg-paper text-ink flex">
      <aside className="w-60 shrink-0 bg-navy text-paper flex flex-col h-screen sticky top-0">
        <div className="px-5 pt-6 pb-4 flex items-center gap-2.5">
          <img src={appIcon} alt="" className="w-7 h-7" />
          <h1 className="font-serif text-xl font-black tracking-wide">OfferFlow</h1>
          <button
            onClick={() => setLang(lang === "zh" ? "en" : "zh")}
            className="ml-auto text-xs font-semibold px-2 py-1 border border-navy-muted text-paper hover:bg-navy-soft"
            title={lang === "zh" ? "Switch to English" : "切换为中文"}
          >
            {lang === "zh" ? "EN" : "中"}
          </button>
        </div>
        <div className="px-4 pb-1">
          <button
            onClick={() => setAppModal({ mode: "create" })}
            className="w-full py-2.5 text-sm font-bold tracking-wider bg-ochre text-ink hover:bg-ochre-light"
          >
            {t("+ 新建投递")}
          </button>
        </div>
        <nav className="flex-1 overflow-y-auto px-3 space-y-0.5">
          <div className={sectionLabel}>{t("求职")}</div>
          <button className={navBtn(inApps)} onClick={() => showApps(appsView)}>{t("投递")}</button>
          <button className={navBtn(nav === "dashboard")} onClick={() => setNav("dashboard")}>{t("投递统计")}</button>
          <div className={sectionLabel}>{t("积累")}</div>
          <button className={navBtn(nav === "review")} onClick={() => setNav("review")}>{t("面试复盘")}</button>
          <button className={navBtn(nav === "resume")} onClick={() => setNav("resume")}>{t("简历版本")}</button>
        </nav>
        <div className="p-3 border-t border-navy-soft space-y-0.5">
          <button className={navBtn(false)} onClick={() => setShowSettings(true)}>{t("系统设置")}</button>
          <button className={navBtn(false)} onClick={() => setShowManage(true)}>{t("公司与岗位")}</button>
          <button className={navBtn(false)} onClick={handleExport} disabled={exporting}>
            {exporting ? t("导出中…") : t("导出 Excel")}
          </button>
        </div>
      </aside>

      <div className="flex-1 flex flex-col min-w-0">
        <header className="bg-paper px-8 pt-7 pb-5 border-b-2 border-ink flex flex-wrap items-end gap-x-6 gap-y-4 sticky top-0 z-10">
          <div className="min-w-0">
            <div className="flex items-center gap-3 text-xs font-bold tracking-[0.22em] text-ochre-dark">
              <span>NO.{meta.no}</span>
              <span className="w-12 h-0.5 bg-ink" />
              <span className="text-ink tabular-nums">{issueDate()}</span>
            </div>
            <h2 className="mt-2 font-serif text-4xl font-black tracking-wide leading-tight">{t(meta.title)}</h2>
            {deck && <p className="mt-1.5 text-sm font-medium text-navy">{deck}</p>}
          </div>
          <div className="ml-auto flex flex-wrap items-center gap-2">
            {/* 搜索、分类筛选和看板 / 列表切换只作用于投递，别的栏目不显示 */}
            {inApps && (
              <>
                <input
                  className="w-72 border-[1.5px] border-ink bg-card px-3 py-2 text-sm placeholder:text-neutral-400 focus:outline-none focus:ring-2 focus:ring-navy"
                  value={query}
                  onChange={(e) => setQuery(e.target.value)}
                  placeholder={t("搜索公司 / 岗位 / 部门 / 城市 / 分类 / 渠道")}
                />
                <select
                  className="border-[1.5px] border-ink bg-card px-2 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-navy"
                  value={catFilter}
                  onChange={(e) => setCatFilter(e.target.value)}
                >
                  <option value="">{t("全部分类")}</option>
                  {CATEGORIES.map((c) => (
                    <option key={c} value={c}>{t(c)}</option>
                  ))}
                </select>
              </>
            )}
            <button
              onClick={() => setAppModal({ mode: "create" })}
              className="px-3 py-2 text-sm font-bold border-[1.5px] border-ink hover:bg-ink hover:text-paper whitespace-nowrap"
            >
              {t("快速粘贴链接")}
            </button>
            {inApps && (
              <div className="flex border-[1.5px] border-ink p-0.5" role="group" aria-label={t("排列方式")}>
                <button
                  onClick={() => showApps("board")}
                  aria-pressed={nav === "board"}
                  className={"px-3 py-1.5 text-sm " + (nav === "board" ? "bg-navy text-paper font-bold" : "hover:bg-neutral-100")}
                >
                  {t("看板")}
                </button>
                <button
                  onClick={() => showApps("list")}
                  aria-pressed={nav === "list"}
                  className={"px-3 py-1.5 text-sm " + (nav === "list" ? "bg-navy text-paper font-bold" : "hover:bg-neutral-100")}
                >
                  {t("列表")}
                </button>
              </div>
            )}
          </div>
        </header>

        <main className="flex-1 overflow-auto px-8 py-6">
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
          <Suspense fallback={<div className="text-neutral-400">{t("加载中…")}</div>}>
          {nav === "dashboard" && (
            <DashboardView applications={applications} jobMap={jobMap} stageMap={stageMap} />
          )}
          {nav === "review" && <ReviewBankView />}
          {nav === "resume" && <ResumeView onChanged={refresh} />}
          </Suspense>
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

      <Suspense fallback={null}>
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
      </Suspense>
    </div>
  );
}
