import { useEffect, useState } from "react";
import { listReviewNotes } from "../api";
import type { ReviewNote } from "../types";
import { htmlToPlainText } from "../format";
import { useI18n } from "../i18n";

export default function ReviewBankView() {
  const { t } = useI18n();
  const [notes, setNotes] = useState<ReviewNote[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  useEffect(() => {
    listReviewNotes()
      .then(setNotes)
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  }, []);

  if (loading) {
    return <div className="text-neutral-400">{t("加载中…")}</div>;
  }

  return (
    <div className="space-y-4">
      {error && <div className="text-sm text-red-600">{error}</div>}
      <div className="text-[11px] font-bold tracking-[0.22em] text-ochre-dark">{t("已沉淀 {n} 条复盘记录", { n: notes.length })}</div>
      <div className="grid grid-cols-2 gap-4">
        {notes.map((n) => (
          <div key={n.id} className="bg-white rounded-xl border border-neutral-200 p-4">
            <div className="flex items-center justify-between">
              <div className="font-serif text-base font-bold">{n.job_title}</div>
              <span className="text-xs text-neutral-400">{n.company_name}</span>
            </div>
            <div className="text-xs text-neutral-500 mt-1">
              {t("第 {n} 轮", { n: n.round })} · {t(n.status)}
              {n.category ? " · " + t(n.category) : ""}
              {n.scheduled_at ? " · " + n.scheduled_at : ""}
            </div>
            <div className="mt-3 text-sm text-neutral-700 whitespace-pre-wrap">{htmlToPlainText(n.feedback)}</div>
          </div>
        ))}
      </div>
      {notes.length === 0 && (
        <div className="text-neutral-400 py-8 text-center">{t("暂无复盘记录，在面试中填写复盘笔记后会自动沉淀到这里")}</div>
      )}
    </div>
  );
}
