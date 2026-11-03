import { useEffect, useRef, useState } from "react";
import { confirm as askConfirm } from "@tauri-apps/plugin-dialog";
import { deleteExperience, importExperiences, listExperiences, saveExperience } from "../api";
import { changeKind, compareExperiences, copyText, createExperience, duplicateVersion, EXPERIENCE_KINDS, experienceSubtitle, experienceText, isPresetField, matchesExperience } from "../experiences";
import type { Experience, ExperienceKind, ExperienceVersion } from "../experiences";
import { useI18n } from "../i18n";
import { useModalFocus } from "../useModalFocus";

const button = "px-3 py-1.5 text-sm border border-neutral-300 hover:bg-neutral-50 disabled:opacity-40";
const primary = "px-3 py-1.5 text-sm bg-navy text-on-navy hover:bg-navy-deep disabled:opacity-40";
const input = "w-full border border-neutral-300 bg-card px-3 py-2 text-sm";
const select = "border border-neutral-300 bg-card px-2 py-1.5 text-sm max-w-48";
const selectionKey = "offerflow.experience-versions";

function CopyButton({ text, label, prominent = false, compact = false }: { text: string; label: string; prominent?: boolean; compact?: boolean }) {
  const { t } = useI18n();
  const [status, setStatus] = useState("");
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(() => { setStatus(""); }, [text]);
  useEffect(() => () => { if (timer.current) clearTimeout(timer.current); }, []);
  const copy = async () => {
    try { await copyText(text); setStatus(t("已复制")); }
    catch { setStatus(t("复制失败，请选中文字后按 Ctrl+C")); }
    if (timer.current) clearTimeout(timer.current);
    timer.current = setTimeout(() => setStatus(""), 2500);
  };
  return <span className="inline-flex flex-wrap items-center gap-1">
    <button className={(prominent ? primary : button) + " whitespace-nowrap"} disabled={!text.trim()} onClick={copy} aria-label={label}>{compact ? t("复制") : label}</button>
    {status && <span role="status" className="text-xs text-ochre-dark">{status}</span>}
  </span>;
}

function Description({ version, heading }: { version: ExperienceVersion; heading?: boolean }) {
  const { t } = useI18n();
  return <div className="min-w-0 space-y-2">
    {heading && <div className="flex flex-wrap gap-2 items-center justify-between"><h4 className="text-sm font-bold">{version.name}</h4><CopyButton text={version.description} label={t("复制描述")} compact /></div>}
    <div className="whitespace-pre-wrap break-words text-sm leading-7 select-text border-l-2 border-ochre pl-4">{version.description || t("尚未填写描述")}</div>
    {version.source && <p className="text-xs text-neutral-400 break-words">{t("来源")}：{version.source}</p>}
  </div>;
}

function ExperienceRow({ item, selected, onSelect, onEdit, onDelete }: {
  item: Experience; selected?: string; onSelect: (id: string) => void; onEdit: () => void; onDelete: () => void;
}) {
  const { t } = useI18n();
  const [open, setOpen] = useState(false);
  const [compare, setCompare] = useState("");
  const version = item.versions.find(v => v.id === selected) ?? item.versions[0];
  const other = item.versions.find(v => v.id === compare && v.id !== version.id);
  const fields = item.fields.filter(f => f.value.trim());
  const multi = item.versions.length > 1;
  const subtitle = experienceSubtitle(item);
  return <article className="bg-card surface rounded-xl">
    <div className="flex flex-wrap items-center gap-3 px-4 py-3">
      <button className="flex-1 min-w-48 text-left flex items-start gap-3" aria-expanded={open} onClick={() => setOpen(!open)}>
        <span aria-hidden="true" className="text-neutral-400 w-3 pt-1">{open ? "▾" : "▸"}</span>
        <span className="min-w-0">
          <span className="block font-serif text-lg font-bold break-words">{item.title}</span>
          {subtitle && <span className="block text-xs text-neutral-500 break-words">{subtitle}</span>}
        </span>
      </button>
      {multi && <select aria-label={t("当前版本")} className={select} value={version.id} onChange={e => onSelect(e.target.value)}>{item.versions.map(v => <option key={v.id} value={v.id}>{v.name}</option>)}</select>}
      {version.description.trim() && <CopyButton text={version.description} label={t("复制描述")} prominent />}
    </div>
    {open &&<div className="border-t border-neutral-200 px-4 py-4 space-y-5">
      {fields.length > 0 && <div className="grid grid-cols-1 lg:grid-cols-2 gap-2">
        {fields.map(f => <div key={f.label} className="border border-neutral-200 bg-paper p-3 flex items-start justify-between gap-3 min-w-0">
          <div className="min-w-0"><div className="text-xs text-neutral-500 mb-1">{t(f.label)}</div><div className="text-sm whitespace-pre-wrap break-words select-text">{f.value}</div></div>
          <CopyButton text={f.value} label={t("复制{field}", { field: t(f.label) })} compact />
        </div>)}
      </div>}
      <div className="space-y-3">
        <div className="flex flex-wrap items-center gap-3">
          <h4 className="text-sm font-bold">{t("描述")}{multi && !other && <span className="font-normal text-neutral-500"> · {version.name}</span>}</h4>
          {multi && <label className="text-sm flex items-center gap-2 ml-auto">{t("对照版本")}<select className={select} value={other?.id ?? ""} onChange={e => setCompare(e.target.value)}><option value="">{t("不对照")}</option>{item.versions.filter(v => v.id !== version.id).map(v => <option key={v.id} value={v.id}>{v.name}</option>)}</select></label>}
        </div>
        {other
          ? <div className="grid grid-cols-1 xl:grid-cols-2 gap-6"><Description version={version} heading /><Description version={other} heading /></div>
          : <Description version={version} />}
      </div>
      <div className="flex flex-wrap gap-2 items-center border-t border-neutral-200 pt-3">
        <CopyButton text={experienceText(item, version)} label={t("复制整段")} />
        <button className={button} onClick={onEdit}>{t("编辑")}</button>
        <button className="text-xs text-red-600 px-2 ml-auto" onClick={onDelete}>{t("删除")}</button>
      </div>
    </div>}
  </article>;
}

function ExperienceEditor({ initial, selected, onClose, onSaved }: {
  initial: Experience; selected?: string; onClose: () => void; onSaved: (item: Experience, selected: string) => void;
}) {
  const { t } = useI18n();
  const [draft, setDraft] = useState<Experience>(() => structuredClone(initial));
  const [active, setActive] = useState(selected ?? initial.versions[0].id);
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);
  const savingRef = useRef(false);
  const close = async () => {
    if (savingRef.current) return;
    if (JSON.stringify(draft) !== JSON.stringify(initial) && !await askConfirm(t("放弃尚未保存的经历修改？"), { title: t("关闭编辑"), kind: "warning" })) return;
    onClose();
  };
  const focus = useModalFocus(() => { void close(); });
  const v = draft.versions.find(v => v.id === active) ?? draft.versions[0];
  const patch = (changes: Partial<ExperienceVersion>) => setDraft(d => ({ ...d, versions: d.versions.map(x => x.id === v.id ? { ...x, ...changes } : x) }));
  const patchField = (i: number, changes: Partial<{ label: string; value: string }>) => setDraft(d => ({ ...d, fields: d.fields.map((x, j) => j === i ? { ...x, ...changes } : x) }));
  const clone = () => {
    let n = 2; let name = t("{name} 副本 {n}", { name: v.name, n });
    while (draft.versions.some(x => x.name.toLowerCase() === name.toLowerCase())) name = t("{name} 副本 {n}", { name: v.name, n: ++n });
    const copy = duplicateVersion(v, name);
    setDraft(d => ({ ...d, versions: [...d.versions, copy] })); setActive(copy.id);
  };
  const removeVersion = async () => {
    if (!await askConfirm(t("删除当前版本？其他版本会保留，保存后生效。"), { title: t("删除确认"), kind: "warning" })) return;
    const remaining = draft.versions.filter(x => x.id !== v.id);
    setDraft(d => ({ ...d, versions: remaining })); setActive(remaining[0].id);
  };
  const submit = async () => {
    if (savingRef.current) return;
    savingRef.current = true; setSaving(true); setError("");
    try { const saved = await saveExperience(draft); onSaved(saved, v.id); }
    catch (e) { setError(String(e)); }
    finally { savingRef.current = false; setSaving(false); }
  };
  return <div className="fixed inset-0 z-50 bg-scrim flex items-center justify-center p-4" onMouseDown={e => { if (e.target === e.currentTarget) void close(); }}>
    <div {...focus} role="dialog" aria-modal="true" aria-labelledby="experience-editor-title" className="bg-card border-2 border-rule shadow-xl w-full max-w-4xl max-h-[92vh] overflow-y-auto p-6">
      <div className="flex items-center justify-between mb-5"><h3 id="experience-editor-title" className="text-xl font-bold">{t(initial.revision ? "编辑经历" : "新增经历")}</h3><button className={button} onClick={() => void close()} disabled={saving}>{t("关闭")}</button></div>
      <form onSubmit={e => { e.preventDefault(); void submit(); }}>
        <fieldset disabled={saving} className="space-y-6">
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <label className="text-sm space-y-1"><span>{t("经历名称")}</span><input required maxLength={200} className={input} value={draft.title} onChange={e => setDraft(d => ({ ...d, title: e.target.value }))} /></label>
            <label className="text-sm space-y-1"><span>{t("经历类型")}</span><select className={input} value={draft.kind} onChange={e => {
              const kind = e.target.value as ExperienceKind;
              setDraft(d => ({ ...d, kind, fields: changeKind(d.fields, d.kind, kind) }));
            }}>{Object.entries(EXPERIENCE_KINDS).map(([k, name]) => <option key={k} value={k}>{t(name)}</option>)}</select></label>
          </div>
          <section className="space-y-2">
            <h4 className="font-bold">{t("基本信息")}</h4>
            <p className="text-xs text-neutral-500">{t("所有版本共用这些信息，每项都能单独复制；日期填 YYYY-MM，结束时间可填“至今”。")}</p>
            {draft.fields.map((f, i) => <div key={i} className="grid grid-cols-[minmax(100px,1fr)_minmax(0,3fr)_auto] gap-2 items-start">
              {isPresetField(draft.kind, f.label)
                ? <span className="text-sm py-2 text-neutral-600">{t(f.label)}</span>
                : <input required maxLength={100} aria-label={t("字段名称 {n}", { n: i + 1 })} placeholder={t("字段名称")} className={input} value={f.label} onChange={e => patchField(i, { label: e.target.value })} />}
              <textarea aria-label={t(f.label) || t("字段内容 {n}", { n: i + 1 })} rows={f.value.length > 80 ? 3 : 1} className={input} value={f.value} onChange={e => patchField(i, { value: e.target.value })} />
              <button type="button" className={button} aria-label={t("移除{field}", { field: t(f.label) })} onClick={() => setDraft(d => ({ ...d, fields: d.fields.filter((_, j) => j !== i) }))}>×</button>
            </div>)}
            <button type="button" className={button} disabled={draft.fields.length >= 30} onClick={() => setDraft(d => ({ ...d, fields: [...d.fields, { label: "", value: "" }] }))}>{t("+ 添加字段")}</button>
          </section>
          <section className="space-y-3">
            <div>
              <h4 className="font-bold">{t("描述版本")}</h4>
              <p className="text-xs text-neutral-500">{t("同一段经历可以按岗位写几版描述，例如通用版、游戏运营版。")}</p>
            </div>
            <div className="flex flex-wrap gap-2" role="group" aria-label={t("描述版本")}>
              {draft.versions.map(x => <button type="button" key={x.id} aria-pressed={x.id === v.id} className={x.id === v.id ? primary : button} onClick={() => setActive(x.id)}>{x.name || t("未命名")}</button>)}
              <button type="button" className={button} disabled={draft.versions.length >= 50} onClick={clone}>{t("复制为新版本")}</button>
            </div>
            <div className="bg-paper p-3 border border-neutral-200 space-y-3">
              <div className="flex flex-wrap gap-2 items-end">
                <label className="flex-1 min-w-48 text-sm">{t("版本名称")}<input required maxLength={100} className={input} value={v.name} onChange={e => patch({ name: e.target.value })} placeholder={t("例如：通用版、游戏运营版")}/></label>
                {draft.versions.length > 1 && <button type="button" className={button} onClick={() => void removeVersion()}>{t("删除此版本")}</button>}
              </div>
              <label className="block text-sm">{t("描述")}<textarea rows={10} maxLength={50000} className={input + " mt-1 leading-7"} value={v.description} onChange={e => patch({ description: e.target.value })} placeholder={t("填写可直接粘贴到招聘表单的文字，保留分段。")}/></label>
              <label className="block text-sm">{t("来源（可选）")}<input maxLength={1000} className={input} value={v.source} onChange={e => patch({ source: e.target.value })}/></label>
            </div>
          </section>
          {error && <div role="alert" className="text-sm text-red-600">{error}</div>}
          <div className="flex justify-end gap-2"><button type="button" className={button} onClick={() => void close()}>{t("取消")}</button><button type="submit" className={primary}>{t(saving ? "保存中…" : "保存")}</button></div>
        </fieldset>
      </form>
    </div>
  </div>;
}

export default function ExperienceView() {
  const { t } = useI18n();
  const [items, setItems] = useState<Experience[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [query, setQuery] = useState("");
  const [kind, setKind] = useState<ExperienceKind | "">("");
  const [editing, setEditing] = useState<Experience | null>(null);
  const [importing, setImporting] = useState(false);
  const importLock = useRef(false);
  const [selected, setSelected] = useState<Record<string, string>>(() => {
    try { const value: unknown = JSON.parse(localStorage.getItem(selectionKey) ?? "{}"); return value && typeof value === "object" && !Array.isArray(value) ? value as Record<string, string> : {}; } catch { return {}; }
  });
  const select = (id: string, version: string) => setSelected(prev => {
    const next = { ...prev, [id]: version }; try { localStorage.setItem(selectionKey, JSON.stringify(next)); } catch { /* Selection persistence is optional. */ } return next;
  });
  const refresh = async () => {
    setLoading(true); setError("");
    try { setItems(await listExperiences()); } catch (e) { setError(String(e)); } finally { setLoading(false); }
  };
  useEffect(() => { void refresh(); }, []);
  const remove = async (item: Experience) => {
    if (!await askConfirm(t("删除这段经历及其所有版本？此操作无法撤销。"), { title: t("删除确认"), kind: "warning" })) return;
    try { await deleteExperience(item.id, item.revision); setItems(prev => prev.filter(x => x.id !== item.id)); setError(""); }
    catch (e) { setError(String(e)); }
  };
  const importFile = async (file?: File) => {
    if (!file || importLock.current) return;
    importLock.current = true; setImporting(true); setError(""); setNotice("");
    try {
      if (file.size > 10 * 1024 * 1024) throw new Error(t("文件不能超过 10 MB"));
      const data: unknown = JSON.parse(await file.text());
      if (!Array.isArray(data)) throw new Error(t("请选择经历 JSON 数据文件"));
      const count = await importExperiences(data);
      setNotice(t("已导入 {n} 段经历，重复条目已跳过。", { n: count }));
      await refresh();
    } catch (e) { setError(String(e)); }
    finally { importLock.current = false; setImporting(false); }
  };
  const visible = items.filter(x => (!kind || kind === x.kind) && matchesExperience(x, query));
  const groups = (Object.keys(EXPERIENCE_KINDS) as ExperienceKind[])
    .map(k => ({ kind: k, items: visible.filter(x => x.kind === k).sort(compareExperiences) }))
    .filter(g => g.items.length);
  return <div className="space-y-5 max-w-7xl mx-auto">
    <div className="flex flex-wrap justify-end gap-2">
      <label className={button + " cursor-pointer"}>{t(importing ? "导入中…" : "导入经历")}<input type="file" accept=".json,application/json" className="sr-only" disabled={importing} onChange={e => { void importFile(e.target.files?.[0]); e.target.value = ""; }} /></label>
      <button className={primary} onClick={() => setEditing(createExperience(kind || "work", t("通用版")))}>{t("+ 新增经历")}</button>
    </div>
    <div className="flex flex-wrap gap-3 items-center"><input className={input + " sm:max-w-sm"} type="search" aria-label={t("搜索经历")} placeholder={t("搜索公司、学校、项目或版本内容…")} value={query} onChange={e => setQuery(e.target.value)} /><button className={button} onClick={() => void refresh()} disabled={loading}>{t("刷新")}</button><span className="text-xs text-neutral-500">{t("{n} 段经历", { n: visible.length })}</span></div>
    <div className="flex flex-wrap gap-2" aria-label={t("经历类型")}><button aria-pressed={!kind} className={!kind ? primary : button} onClick={() => setKind("")}>{t("全部")}</button>{Object.entries(EXPERIENCE_KINDS).map(([k, name]) => <button key={k} aria-pressed={kind === k} className={kind === k ? primary : button} onClick={() => setKind(k as ExperienceKind)}>{t(name)}</button>)}</div>
    {error && <div role="alert" className="text-sm text-red-600">{error}</div>}
    {notice && <div role="status" className="text-sm text-navy">{notice}</div>}
    {loading ? <div className="py-10 text-neutral-500 text-center">{t("加载中…")}</div> : <>
      {!visible.length && <div className="surface bg-card p-10 text-center space-y-2"><p className="font-bold">{t(items.length ? "没有匹配的经历" : "把常用经历放在这里")}</p><p className="text-sm text-neutral-500">{t(items.length ? "试试其他关键词或分类。" : "新增或导入经历后，可以分别复制公司、职位、时间和描述，也可以保留不同版本。")}</p></div>}
      {groups.map(g => <section key={g.kind} className="space-y-2">
        <h3 className="text-xs font-bold text-ochre-dark tracking-wider">{t(EXPERIENCE_KINDS[g.kind])} · {g.items.length}</h3>
        {g.items.map(item => <ExperienceRow key={item.id} item={item} selected={selected[item.id]} onSelect={v => select(item.id, v)} onEdit={() => setEditing(item)} onDelete={() => void remove(item)} />)}
      </section>)}
    </>}
    {editing && <ExperienceEditor initial={editing} selected={selected[editing.id]} onClose={() => setEditing(null)} onSaved={(saved, version) => {
      setItems(prev => prev.some(x => x.id === saved.id) ? prev.map(x => x.id === saved.id ? saved : x) : [...prev, saved]);
      select(saved.id, version); setEditing(null); setError("");
    }} />}
  </div>;
}
