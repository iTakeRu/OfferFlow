import {
  DndContext, KeyboardSensor, PointerSensor, useDraggable, useDroppable, useSensor, useSensors,
} from "@dnd-kit/core";
import type { DragEndEvent } from "@dnd-kit/core";
import { CSS } from "@dnd-kit/utilities";
import type { ReactNode } from "react";
import type { Stage, Application, Job, Company } from "../types";
import { useI18n } from "../i18n";

const STATUS_COLOR: Record<string, string> = {
  推进中: "text-navy",
  已获Offer: "text-ink",
  未通过筛选: "text-neutral-400",
  无回应超时: "text-ochre-dark",
};

function ApplicationCard({ app, job, company, onOpen }: {
  app: Application; job?: Job; company?: Company; onOpen: (a: Application) => void;
}) {
  const { t } = useI18n();
  const { attributes, listeners, setNodeRef, transform, isDragging } = useDraggable({ id: app.id });
  const style = { transform: CSS.Translate.toString(transform) };
  const offer = app.status === "已获Offer";
  const rejected = app.status === "未通过筛选";
  return (
    <div
      ref={setNodeRef}
      style={style}
      {...listeners}
      {...attributes}
      role="button"
      tabIndex={0}
      aria-label={`${company?.name ?? t("未知公司")} · ${job?.title ?? t("未知岗位")}`}
      onClick={() => onOpen(app)}
      onKeyDown={(e) => {
        if (e.key === "Enter") {
          e.preventDefault();
          onOpen(app);
        }
      }}
      className={
        "block rounded-lg border p-3.5 cursor-grab focus:outline-none focus:ring-2 focus:ring-navy transition-colors " +
        (offer ? "bg-ochre border-ochre hover:border-ink " : "bg-card border-neutral-200 hover:border-ink ") +
        (rejected ? "opacity-60 " : "") +
        (isDragging ? "opacity-60 border-2 border-ink" : "")
      }
    >
      <div className="flex items-center gap-2">
        <span
          aria-hidden="true"
          className={"w-6 h-6 shrink-0 flex items-center justify-center font-serif font-black text-xs text-paper " + (offer ? "bg-ink" : "bg-navy")}
        >
          {(company?.name ?? "?").slice(0, 1)}
        </span>
        <span className={"text-xs truncate flex-1 " + (offer ? "text-ink" : "text-neutral-500")}>
          {company?.name ?? t("未知公司")}
        </span>
        {job?.category && <span className={"text-[11px] shrink-0 " + (offer ? "text-ink" : "text-neutral-400")}>{t(job.category)}</span>}
      </div>
      <div className={"mt-2 font-serif font-bold text-[15px] leading-snug " + (rejected ? "line-through" : "")}>
        {job?.title ?? t("未知岗位")}
      </div>
      <div className={"mt-2.5 pt-2 flex items-center gap-2 flex-wrap border-t " + (offer ? "border-ink" : "border-neutral-200")}>
        <span className={"inline-flex items-center gap-1.5 text-xs font-bold " + (STATUS_COLOR[app.status] ?? "text-neutral-500")}>
          <span className="w-1.5 h-1.5 rounded-full bg-current" />
          {t(app.status)}
        </span>
        {app.channel && <span className={"text-xs " + (offer ? "text-ink" : "text-neutral-400")}>{app.channel}</span>}
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
      aria-label={t(stage.name)}
      className={"w-64 shrink-0 flex flex-col p-2 -m-2 rounded-xl transition-colors " + (isOver ? "bg-navy-tint" : "")}
    >
      <div className="flex items-baseline gap-2 px-0.5 pb-2 mb-3 border-b-2 border-ink">
        <span className="font-serif text-base font-black tracking-wide truncate">{t(stage.name)}</span>
        <span className="font-display text-lg leading-none text-ochre-dark tabular-nums">{count}</span>
      </div>
      <div className="flex flex-col gap-2.5 min-h-24">{children}</div>
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
  // 必须给一个激活距离：默认的 PointerSensor 在 pointerdown 就进入拖拽态，
  // 并在 document 捕获阶段吞掉 click，导致卡片根本点不开。
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
    useSensor(KeyboardSensor),
  );

  const handleDragEnd = (e: DragEndEvent) => {
    const { active, over } = e;
    if (!over) return;
    const appId = Number(active.id);
    const stageId = Number(over.id);
    // 拖回原阶段没必要打一次库、也没必要写一条时间线。
    if (applications.find((a) => a.id === appId)?.stage_id === stageId) return;
    onMove(appId, stageId);
  };

  return (
    <DndContext sensors={sensors} onDragEnd={handleDragEnd}>
      <div className="flex gap-7 items-start overflow-x-auto p-2 pb-4">
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
                <div className="text-xs text-neutral-400 py-6 text-center border-[1.5px] border-dashed border-neutral-300 rounded-lg">{t("暂无投递")}</div>
              )}
            </StageSection>
          );
        })}
      </div>
    </DndContext>
  );
}