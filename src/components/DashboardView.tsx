import { useMemo } from "react";
import { APPLICATION_STATUS } from "../types";
import type { Application, Job, Company } from "../types";
import { useI18n } from "../i18n";

interface Props {
  applications: Application[];
  jobMap: Map<number, Job>;
  companyMap: Map<number, Company>;
  stageMap: Map<number, string>;
}

export default function DashboardView({ applications, jobMap, companyMap, stageMap }: Props) {
  const { t } = useI18n();
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
  }, [applications, jobMap, companyMap, stageMap]);

  const { total, offer, stageCount, catCount, statusCount, companyCount } = stats;

  const kpi = [
    { label: t("投递总数"), value: total },
    { label: t("已获 Offer"), value: offer },
    { label: t("推进中"), value: statusCount.get("推进中") ?? 0 },
    { label: t("覆盖公司"), value: companyCount },
  ];

  const maxStage = Math.max(1, ...Array.from(stageCount.values()));

  return (
    <div className="space-y-6">
      <div className="grid grid-cols-4 gap-4">
        {kpi.map((k) => (
          <div key={k.label} className="bg-white rounded-xl border border-neutral-200 p-4">
            <div className="text-xs text-neutral-500">{k.label}</div>
            <div className="text-2xl font-semibold mt-1">{k.value}</div>
          </div>
        ))}
      </div>

      <div className="grid grid-cols-2 gap-4">
        <div className="bg-white rounded-xl border border-neutral-200 p-4">
          <div className="text-sm font-semibold mb-3">{t("投递阶段分布")}</div>
          <div className="space-y-2">
            {Array.from(stageCount.entries()).map(([name, count]) => (
              <div key={name}>
                <div className="flex justify-between text-xs text-neutral-500 mb-1">
                  <span>{t(name)}</span>
                  <span>{count}</span>
                </div>
                <div className="h-2 bg-neutral-100 rounded-full overflow-hidden">
                  <div className="h-full bg-neutral-800 rounded-full" style={{ width: (count / maxStage) * 100 + "%" }} />
                </div>
              </div>
            ))}
            {stageCount.size === 0 && <div className="text-sm text-neutral-400 py-3">{t("暂无数据")}</div>}
          </div>
        </div>

        <div className="bg-white rounded-xl border border-neutral-200 p-4">
          <div className="text-sm font-semibold mb-3">{t("岗位分类分布")}</div>
          <div className="space-y-2">
            {Array.from(catCount.entries()).map(([cat, count]) => (
              <div key={cat}>
                <div className="flex justify-between text-xs text-neutral-500 mb-1">
                  <span>{t(cat)}</span>
                  <span>{count}</span>
                </div>
                <div className="h-2 bg-neutral-100 rounded-full overflow-hidden">
                  <div className="h-full bg-neutral-800 rounded-full" style={{ width: total ? (count / total) * 100 + "%" : "0%" }} />
                </div>
              </div>
            ))}
            {catCount.size === 0 && <div className="text-sm text-neutral-400 py-3">{t("暂无数据")}</div>}
          </div>
        </div>
      </div>

      <div className="bg-white rounded-xl border border-neutral-200 p-4">
        <div className="text-sm font-semibold mb-3">{t("状态分布")}</div>
        <div className="flex flex-wrap gap-2">
          {APPLICATION_STATUS.map((status) => (
            <div key={status} className="px-3 py-2 rounded-md bg-neutral-50 border border-neutral-200">
              <span className="text-sm">{t(status)}</span>
              <span className="text-sm font-semibold ml-2">{statusCount.get(status) ?? 0}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}