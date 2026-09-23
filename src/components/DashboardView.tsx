import { useEffect, useMemo, useState } from "react";
import { listUpcomingInterviews } from "../api";
import { APPLICATION_STATUS } from "../types";
import type { Application, Job, UpcomingInterview } from "../types";
import { useI18n } from "../i18n";

const UPCOMING_DAYS = 14;

interface Props {
  applications: Application[];
  jobMap: Map<number, Job>;
  stageMap: Map<number, string>;
}

export default function DashboardView({ applications, jobMap, stageMap }: Props) {
  const { t } = useI18n();
  const [upcoming, setUpcoming] = useState<UpcomingInterview[]>([]);

  useEffect(() => {
    // 后端一直有这个查询，只是之前没有任何界面用到它。
    listUpcomingInterviews(UPCOMING_DAYS).then(setUpcoming).catch(() => setUpcoming([]));
  }, []);

  const stats = useMemo(() => {
    const total = applications.length;
    const stageCount = new Map<string, number>();
    const catCount = new Map<string, number>();
    const statusCount = new Map<string, number>();
    const companySet = new Set<number>();
    let offer = 0;
    for (const a of applications) {
      const stage = stageMap.get(a.stage_id) ?? "未知";
      stageCount.set(stage, (stageCount.get(stage) ?? 0) + 1);
      const job = jobMap.get(a.job_id);
      const cat = job?.category ?? "未分类";
      catCount.set(cat, (catCount.get(cat) ?? 0) + 1);
      statusCount.set(a.status, (statusCount.get(a.status) ?? 0) + 1);
      if (job) companySet.add(job.company_id);
      if (a.status === "已获Offer") offer += 1;
    }
    return { total, offer, stageCount, catCount, statusCount, companyCount: companySet.size };
  }, [applications, jobMap, stageMap]);

  const { total, offer, stageCount, catCount, statusCount, companyCount } = stats;

  const kpi = [
    { label: t("投递总数"), value: total, accent: false },
    { label: t("已获 Offer"), value: offer, accent: true },
    { label: t("推进中"), value: statusCount.get("推进中") ?? 0, accent: false },
    { label: t("覆盖公司"), value: companyCount, accent: false },
  ];

  const maxStage = Math.max(1, ...Array.from(stageCount.values()));

  const tile = "bg-card rounded-xl border border-neutral-200 p-5";
  const tileTitle = "font-serif text-lg font-black tracking-wide pb-2 mb-4 border-b-2 border-ink";
  const kicker = "text-[11px] font-bold tracking-[0.22em]";

  const bars = (entries: [string, number][], max: number) => (
    <div className="space-y-3">
      {entries.map(([name, count]) => (
        <div key={name}>
          <div className="flex justify-between text-xs mb-1.5">
            <span className="text-neutral-600">{t(name)}</span>
            <span className="font-bold tabular-nums">{count}</span>
          </div>
          <div className="h-2.5 bg-neutral-100">
            <div className="h-full bg-navy" style={{ width: max ? (count / max) * 100 + "%" : "0%" }} />
          </div>
        </div>
      ))}
      {entries.length === 0 && <div className="text-sm text-neutral-400 py-3">{t("暂无数据")}</div>}
    </div>
  );

  return (
    <div className="space-y-5">
      <div className="grid grid-cols-4 gap-4">
        {kpi.map((k) => (
          <div
            key={k.label}
            className={"rounded-xl p-5 flex flex-col " + (k.accent ? "bg-ochre text-ink" : "bg-card border border-neutral-200")}
          >
            <div className={kicker + " " + (k.accent ? "text-ink" : "text-ochre-dark")}>{k.label}</div>
            <div className="font-display text-6xl leading-none mt-4 tabular-nums">{k.value}</div>
          </div>
        ))}
      </div>

      <div className="bg-navy text-paper rounded-xl p-5">
        <div className="font-serif text-lg font-black tracking-wide pb-2 mb-2 border-b-2 border-paper">
          {t("近 {n} 天面试安排", { n: UPCOMING_DAYS })}
        </div>
        {upcoming.length === 0 ? (
          <div className="text-sm text-navy-muted py-2">{t("暂无待办面试")}</div>
        ) : (
          <ul className="divide-y divide-navy-soft">
            {upcoming.map((i) => (
              <li key={i.id} className="flex items-center gap-4 py-2.5 text-sm">
                <span className="w-12 shrink-0 flex flex-col items-center">
                  <span className="text-[11px] font-bold tracking-widest text-ochre-light tabular-nums">{i.scheduled_at.slice(5, 7)}</span>
                  <span className="font-display text-3xl leading-none tabular-nums">{i.scheduled_at.slice(8, 10)}</span>
                </span>
                <span className="font-bold truncate">{i.company_name}</span>
                <span className="text-navy-muted truncate">{i.job_title}</span>
                <span className="flex-1" />
                <span className="text-xs text-navy-muted whitespace-nowrap tabular-nums">
                  {i.scheduled_at.slice(11, 16)} · {t("第 {n} 轮", { n: i.round })}
                  {i.format ? ` · ${t(i.format)}` : ""} · {t(i.status)}
                </span>
              </li>
            ))}
          </ul>
        )}
      </div>

      <div className="grid grid-cols-2 gap-4">
        <div className={tile}>
          <div className={tileTitle}>{t("投递阶段分布")}</div>
          {bars(Array.from(stageCount.entries()), maxStage)}
        </div>
        <div className={tile}>
          <div className={tileTitle}>{t("岗位分类分布")}</div>
          {bars(Array.from(catCount.entries()), total)}
        </div>
      </div>

      <div className={tile}>
        <div className={tileTitle}>{t("状态分布")}</div>
        <div className="grid grid-cols-4 gap-3">
          {APPLICATION_STATUS.map((status) => (
            <div key={status} className="border-[1.5px] border-ink px-4 py-3 flex items-baseline justify-between">
              <span className="text-sm">{t(status)}</span>
              <span className="font-display text-3xl leading-none tabular-nums">{statusCount.get(status) ?? 0}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}