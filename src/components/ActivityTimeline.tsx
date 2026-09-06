import { useEffect, useState } from "react";
import { listActivities, addActivityNote } from "../api";
import type { ActivityLog } from "../types";
import { useI18n } from "../i18n";

interface Props {
  applicationId: number;
}

const KIND_COLOR: Record<string, string> = {
  created: "bg-neutral-400",
  stage: "bg-neutral-600",
  status: "bg-amber-400",
  note: "bg-neutral-300",
};

export default function ActivityTimeline({ applicationId }: Props) {
  const { t } = useI18n();
  const [logs, setLogs] = useState<ActivityLog[]>([]);
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  const reload = () => {
    listActivities(applicationId).then(setLogs).catch((e) => setError(String(e)));
  };

  useEffect(() => {
    reload();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [applicationId]);

  const submit = async () => {
    const d = draft.trim();
    if (!d) return;
    setBusy(true);
    setError("");
    try {
      await addActivityNote(applicationId, d);
      setDraft("");
      reload();
    } catch (e) { setError(String(e)); } finally {
      setBusy(false);
    }
  };

  return (
    <div className="mt-5">
      <div className="text-xs font-medium text-neutral-500 mb-2">{t("投递时间线")}</div>
      {error && <div className="text-xs text-red-600 mb-2">{error}</div>}
      <div className="flex gap-2 mb-3">
        <input
          className="flex-1 rounded-md border border-neutral-300 px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-neutral-400 bg-white"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter") submit(); }}
          placeholder={t("添加一条备注记录…")}
        />
        <button
          type="button"
          onClick={submit}
          disabled={busy || !draft.trim()}
          className="px-3 py-1.5 text-sm rounded-md bg-neutral-100 hover:bg-neutral-200 disabled:opacity-50 whitespace-nowrap"
        >
          {t("添加")}
        </button>
      </div>
      {logs.length === 0 ? (
        <div className="text-xs text-neutral-400 py-2">{t("暂无记录")}</div>
      ) : (
        <ol className="space-y-2.5 max-h-56 overflow-y-auto pr-1">
          {logs.map((l) => (
            <li key={l.id} className="flex gap-2.5 text-sm">
              <span className={`mt-1.5 h-2 w-2 rounded-full shrink-0 ${KIND_COLOR[l.kind] ?? "bg-neutral-300"}`} />
              <div className="flex-1 min-w-0">
                <div className="text-neutral-800 break-words">{l.detail}</div>
                <div className="text-xs text-neutral-400 mt-0.5">{l.created_at}</div>
              </div>
            </li>
          ))}
        </ol>
      )}
    </div>
  );
}
