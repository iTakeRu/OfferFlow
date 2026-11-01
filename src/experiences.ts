export const EXPERIENCE_KINDS = {
  work: "实习 / 工作", education: "教育经历", project: "项目经历", research: "研究经历",
  portfolio: "作品集", text: "常用文本", other: "其他经历",
} as const;
export type ExperienceKind = keyof typeof EXPERIENCE_KINDS;
export interface ExperienceField { label: string; value: string }
/** 同一段经历的不同写法：只有描述不同，公司、时间等字段由经历共用。 */
export interface ExperienceVersion {
  id: string;
  name: string;
  description: string;
  source: string;
}
export interface Experience {
  id: string;
  title: string;
  kind: ExperienceKind;
  fields: ExperienceField[];
  versions: ExperienceVersion[];
  revision: number;
}

const FIELDS: Record<ExperienceKind, string[]> = {
  work: ["公司名称", "职位名称", "部门", "开始时间", "结束时间", "工作地点"],
  education: ["学校名称", "专业", "学历 / 学位", "学院", "学习形式", "开始时间", "结束时间", "主修课程", "荣誉奖项"],
  project: ["项目名称", "担任角色", "所属组织", "开始时间", "结束时间"],
  research: ["研究题目", "担任角色", "研究状态", "开始时间", "结束时间"],
  portfolio: ["作品名称", "作品链接", "担任角色"],
  text: [],
  other: ["所属组织", "担任角色", "开始时间", "结束时间"],
};

/** 预设字段名固定（便于翻译和日期校验），自定义字段才能改名。 */
export const isPresetField = (kind: ExperienceKind, label: string) => FIELDS[kind].includes(label);

export function createExperience(kind: ExperienceKind, versionName: string): Experience {
  return {
    id: crypto.randomUUID(), title: "", kind, revision: 0,
    fields: FIELDS[kind].map(label => ({ label, value: "" })),
    versions: [{ id: crypto.randomUUID(), name: versionName, description: "", source: "" }],
  };
}

/** 切换类型：补上新类型的预设字段，丢掉空着的旧预设字段，已填内容和自定义字段保留。 */
export function changeKind(fields: ExperienceField[], from: ExperienceKind, to: ExperienceKind): ExperienceField[] {
  const kept = fields.filter(f => f.value.trim() || !isPresetField(from, f.label) || FIELDS[to].includes(f.label));
  return [
    ...FIELDS[to].map(label => kept.find(f => f.label === label) ?? { label, value: "" }),
    ...kept.filter(f => !FIELDS[to].includes(f.label)),
  ];
}

export function duplicateVersion(version: ExperienceVersion, name: string): ExperienceVersion {
  return { ...version, id: crypto.randomUUID(), name };
}

const fieldValue = (item: Experience, label: string) => item.fields.find(f => f.label === label)?.value.trim() ?? "";

/** 列表行的副标题：角色 / 专业 + 起止时间。 */
export function experienceSubtitle(item: Experience): string {
  const role = ["职位名称", "专业", "担任角色", "研究状态"].map(l => fieldValue(item, l)).find(Boolean) ?? "";
  const start = fieldValue(item, "开始时间"), end = fieldValue(item, "结束时间");
  return [role, start || end ? `${start} – ${end}` : ""].filter(Boolean).join(" · ");
}

/** 按开始时间倒序；未填时间的放最后。 */
export function compareExperiences(a: Experience, b: Experience): number {
  const sa = fieldValue(a, "开始时间"), sb = fieldValue(b, "开始时间");
  if (sa !== sb) return !sa ? 1 : !sb ? -1 : sb.localeCompare(sa);
  return a.title.localeCompare(b.title);
}

/** Copy values only: headings/version names must never leak into a form field. */
export function experienceText(item: Experience, version: ExperienceVersion): string {
  return [...item.fields.filter(f => f.value.trim()).map(f => `${f.label}：${f.value}`), version.description].filter(Boolean).join("\n");
}

export function matchesExperience(item: Experience, query: string): boolean {
  const q = query.trim().toLocaleLowerCase();
  return !q || [item.title, EXPERIENCE_KINDS[item.kind], ...item.fields.map(f => f.value), ...item.versions.flatMap(v => [v.name, v.description])].join(" ").toLocaleLowerCase().includes(q);
}

export async function copyText(text: string): Promise<void> {
  if (!text.trim()) throw new Error("没有可复制的内容");
  try {
    if (navigator.clipboard?.writeText) { await navigator.clipboard.writeText(text); return; }
  } catch { /* Some WebView clipboard policies require the selection fallback. */ }
  const previous = document.activeElement as HTMLElement | null;
  const input = document.createElement("textarea");
  input.value = text;
  input.style.cssText = "position:fixed;left:-9999px;top:0";
  document.body.appendChild(input);
  try {
    input.select();
    if (!document.execCommand("copy")) throw new Error("复制失败，请选中文字后按 Ctrl+C");
  } finally { input.remove(); previous?.focus(); }
}
