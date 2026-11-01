import type { Application, Job, Company } from "../types";
import { useI18n } from "../i18n";

interface Props {
  applications: Application[];
  jobMap: Map<number, Job>;
  companyMap: Map<number, Company>;
  stageMap: Map<number, string>;
  onOpen: (app: Application) => void;
}

export default function ListView({ applications, jobMap, companyMap, stageMap, onOpen }: Props) {
  const { t } = useI18n();
  return (
    <div className="bg-card surface rounded-xl overflow-hidden">
      <div className="overflow-x-auto">
        <table className="w-full text-sm min-w-[900px]">
          <thead>
            <tr className="text-left text-[11px] tracking-[0.18em] text-ochre-dark border-b-2 border-rule">
              <th className="px-4 py-2.5 font-bold">{t("公司")}</th>
              <th className="px-4 py-2.5 font-bold">{t("岗位")}</th>
              <th className="px-4 py-2.5 font-bold">{t("分类")}</th>
              <th className="px-4 py-2.5 font-bold">{t("部门")}</th>
              <th className="px-4 py-2.5 font-bold">{t("城市")}</th>
              <th className="px-4 py-2.5 font-bold">{t("渠道")}</th>
              <th className="px-4 py-2.5 font-bold">{t("阶段")}</th>
              <th className="px-4 py-2.5 font-bold">{t("状态")}</th>
              <th className="px-4 py-2.5 font-bold">{t("投递日期")}</th>
            </tr>
          </thead>
          <tbody>
            {applications.map((a) => {
              const job = jobMap.get(a.job_id);
              const company = companyMap.get(job?.company_id ?? -1);
              return (
                <tr
                  key={a.id}
                  onClick={() => onOpen(a)}
                  className="border-t border-neutral-100 hover:bg-neutral-50 cursor-pointer"
                >
                  <td className="px-4 py-2.5 font-serif font-bold">{company?.name ?? "—"}</td>
                  <td className="px-4 py-2.5">{job?.title ?? "—"}</td>
                  <td className="px-4 py-2.5">{job?.category ? t(job.category) : "—"}</td>
                  <td className="px-4 py-2.5">{job?.department ?? "—"}</td>
                  <td className="px-4 py-2.5">{job?.location ?? "—"}</td>
                  <td className="px-4 py-2.5">{a.channel ?? "—"}</td>
                  <td className="px-4 py-2.5">{stageMap.get(a.stage_id) ? t(stageMap.get(a.stage_id)!) : "—"}</td>
                  <td className="px-4 py-2.5">{t(a.status)}</td>
                  <td className="px-4 py-2.5 text-neutral-400 tabular-nums">{a.applied_at ?? "—"}</td>
                </tr>
              );
            })}
            {applications.length === 0 && (
              <tr>
                <td colSpan={9} className="px-4 py-8 text-center text-neutral-400">{t("暂无投递记录")}</td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}