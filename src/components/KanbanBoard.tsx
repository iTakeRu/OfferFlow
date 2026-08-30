import { DndContext, useDraggable, useDroppable } from "@dnd-kit/core";
import type { DragEndEvent } from "@dnd-kit/core";
import { CSS } from "@dnd-kit/utilities";
import type { ReactNode } from "react";
import type { Stage, Application, Job, Company } from "../types";
import { useI18n } from "../i18n";

const STATUS_COLOR: Record<string, string> = {
  推进中: "bg-neutral-200 text-neutral-700",
  已获Offer: "bg-green-100 text-green-700",
  未通过筛选: "bg-red-100 text-red-700",
  无回应超时: "bg-amber-100 text-amber-700",
};

function ApplicationCard({ app, job, company, onOpen }: {
  app: Application; job?: Job; company?: Company; onOpen: (a: Application) => void;
}) {
  const { t } = useI18n();
  const { attributes, listeners, setNodeRef, transform, isDragging } = useDraggable({ id: app.id });
  const style = { transform: CSS.Translate.toString(transform) };
  return (
    <div
      ref={setNodeRef}
      style={style}
      {...listeners}
      {...attributes}
      onClick={() => onOpen(app)}
      className={"w-64 shrink-0 bg-white rounded-lg border border-neutral-200 p-3 cursor-grab shadow-sm hover:shadow " + (isDragging ? "opacity-60" : "")}
    >
      <div className="font-medium truncate text-sm">{job?.title ?? t("未知岗位")}</div>
      <div className="text-xs text-neutral-500 mt-0.5 truncate">{company?.name ?? t("未知公司")}</div>
      {job?.category && <div className="text-[11px] text-neutral-400 mt-0.5 truncate">{t(job.category)}</div>}
      <div className="mt-2 flex items-center gap-1.5 flex-wrap">
        <span className={"text-xs px-2 py-0.5 rounded-full " + (STATUS_COLOR[app.status] ?? "bg-neutral-100 text-neutral-600")}>
          {t(app.status)}
        </span>
        {app.channel && <span className="text-xs text-neutral-400">{app.channel}</span>}
      </div>
    </div>
  );
}

function StageSection({ stage, count, children }: { stage: Stage; count: number; children: ReactNode }) {
  const { t } = useI18n();
  const { setNodeRef, isOver } = useDroppable({ id: stage.id });
  return (
    <section
      ref={setNodeRef}
      className={"rounded-lg p-3 transition-colors " + (isOver ? "bg-neutral-100 ring-2 ring-neutral-300" : "bg-neutral-200/70")}
    >
      <div className="flex items-center justify-between px-1 pb-2">
        <span className="font-semibold text-sm">{t(stage.name)}</span>
        <span className="text-xs text-neutral-400">{count}</span>
      </div>
      <div className="flex flex-wrap gap-2 min-h-16">{children}</div>
    </section>
  );
}

interface Props {
  stages: Stage[];
  applications: Application[];
  jobMap: Map<number, Job>;
  companyMap: Map<number, Company>;
  onMove: (appId: number, stageId: number) => void;
  onOpen: (app: Application) => void;
}

export default function KanbanBoard({ stages, applications, jobMap, companyMap, onMove, onOpen }: Props) {
  const { t } = useI18n();
  const handleDragEnd = (e: DragEndEvent) => {
    const { active, over } = e;
    if (over) {
      onMove(Number(active.id), Number(over.id));
    }
  };

  return (
    <DndContext onDragEnd={handleDragEnd}>
      <div className="flex flex-col gap-4">
        {stages.map((stage) => {
          const apps = applications.filter((a) => a.stage_id === stage.id);
          return (
            <StageSection key={stage.id} stage={stage} count={apps.length}>
              {apps.map((a) => {
                const job = jobMap.get(a.job_id);
                const company = companyMap.get(job?.company_id ?? -1);
                return (
                  <ApplicationCard key={a.id} app={a} job={job} company={company} onOpen={onOpen} />
                );
              })}
              {apps.length === 0 && (
                <div className="text-xs text-neutral-400 py-4 text-center w-full">{t("暂无投递")}</div>
              )}
            </StageSection>
          );
        })}
      </div>
    </DndContext>
  );
}