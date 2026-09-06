import { useEffect, useState } from "react";
import { listResumes, importResume, deleteResume, updateResume, openManagedPath } from "../api";
import { open, confirm as askConfirm } from "@tauri-apps/plugin-dialog";
import type { Resume } from "../types";
import { formatSize } from "../format";
import { useI18n } from "../i18n";

interface Props {
  onChanged: () => void;
}

export default function ResumeView({ onChanged }: Props) {
  const { t } = useI18n();
  const [resumes, setResumes] = useState<Resume[]>([]);
  const [importing, setImporting] = useState(false);
  const [error, setError] = useState("");
  const [editingId, setEditingId] = useState<number | null>(null);
  const [editName, setEditName] = useState("");
  const [editTags, setEditTags] = useState("");

  const refresh = () => {
    return listResumes().then(setResumes).catch((e) => setError(String(e)));
  };

  useEffect(() => {
    refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleImport = async () => {
    try {
      const source = await open({ title: t("选择简历文件"), multiple: false, filters: [{ name: t("简历文档"), extensions: ["pdf", "doc", "docx", "txt", "rtf"] }] });
      if (!source || Array.isArray(source)) return;
      setImporting(true);
      setError("");
      await importResume({ name: "", sourcePath: source });
      await refresh();
      onChanged();
    } catch (e) {
      setError(String(e));
    } finally {
      setImporting(false);
    }
  };

  const handleDelete = async (id: number) => {
    const ok = await askConfirm(t("删除后简历文件将从磁盘移除，确认继续？"), { title: t("删除确认"), kind: "warning" });
    if (!ok) return;
    try {
      setError("");
      await deleteResume(id);
      await refresh();
      onChanged();
    } catch (e) { setError(String(e)); }
  };

  const startEdit = (r: Resume) => {
    setEditingId(r.id);
    setEditName(r.name);
    setEditTags((r.tags ?? "").split(",").filter(Boolean).join(", "));
  };

  const saveEdit = async () => {
    if (editingId == null) return;
    const name = editName.trim() || t("未命名");
    const tags = editTags
      .split(/[,，]/)
      .map((s) => s.trim())
      .filter(Boolean)
      .join(",");
    try {
      setError("");
      await updateResume(editingId, name, tags || null);
      setEditingId(null);
      await refresh();
      onChanged();
    } catch (e) {
      setError(String(e));
    }
  };

  const handlePreview = async (r: Resume) => {
    try {
      setError("");
      await openManagedPath(r.filepath);
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div className="text-sm font-semibold text-neutral-700">{t("简历版本（{n}）", { n: resumes.length })}</div>
        <button
          onClick={handleImport}
          disabled={importing}
          className="px-3 py-1.5 text-sm rounded-md bg-neutral-900 text-white hover:bg-neutral-800 disabled:opacity-50"
        >
          {importing ? t("导入中…") : t("+ 导入简历")}
        </button>
      </div>
      {error && <div className="text-sm text-red-600">{error}</div>}
      <div className="grid grid-cols-1 gap-3">
        {resumes.map((r) => {
          const tags = (r.tags ?? "").split(",").filter(Boolean);
          const isEditing = editingId === r.id;
          return (
            <div key={r.id} className="bg-white rounded-xl border border-neutral-200 p-4">
              <div className="flex items-center justify-between">
                {isEditing ? (
                  <input
                    className="flex-1 rounded-md border border-neutral-300 px-2 py-1 text-sm"
                    value={editName}
                    onChange={(e) => setEditName(e.target.value)}
                    placeholder={t("简历名称")}
                  />
                ) : (
                  <div>
                    <div className="text-sm font-medium">{r.name || t("未命名")}</div>
                    <div className="text-xs text-neutral-400 mt-0.5">{r.filename} · {formatSize(r.size)}</div>
                  </div>
                )}
                <div className="flex items-center gap-2 shrink-0 ml-3">
                  {isEditing ? (
                    <>
                      <button onClick={saveEdit} className="px-2 py-1 text-sm rounded bg-neutral-900 text-white">{t("保存")}</button>
                      <button onClick={() => setEditingId(null)} className="px-2 py-1 text-sm rounded border border-neutral-300">{t("取消")}</button>
                    </>
                  ) : (
                    <>
                      <button onClick={() => handlePreview(r)} className="px-2 py-1 text-sm rounded border border-neutral-300 hover:bg-neutral-50">{t("预览")}</button>
                      <button onClick={() => startEdit(r)} className="px-2 py-1 text-sm rounded border border-neutral-300 hover:bg-neutral-50">{t("重命名")}</button>
                      <button onClick={() => handleDelete(r.id)} className="px-2 py-1 text-sm text-red-500 hover:text-red-600">{t("删除")}</button>
                    </>
                  )}
                </div>
              </div>
              <div className="mt-2 flex items-center gap-2 flex-wrap min-h-6">
                {isEditing ? (
                  <input
                    className="flex-1 max-w-xs rounded-md border border-neutral-300 px-2 py-1 text-sm"
                    value={editTags}
                    onChange={(e) => setEditTags(e.target.value)}
                    placeholder={t("标签，用逗号分隔")}
                  />
                ) : (
                  tags.map((tg) => (
                    <span key={tg} className="text-xs px-2 py-0.5 rounded-full bg-neutral-100 text-neutral-600">{tg}</span>
                  ))
                )}
              </div>
            </div>
          );
        })}
      </div>
      {resumes.length === 0 && (
        <div className="text-neutral-400 py-8 text-center">{t("暂无简历版本，导入后可在这里预览、命名和打标签")}</div>
      )}
    </div>
  );
}
