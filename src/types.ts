export interface Company {
  id: number;
  name: string;
  industry: string | null;
  city: string | null;
  website: string | null;
  notes: string | null;
  created_at: string;
}

export interface Job {
  id: number;
  company_id: number;
  title: string;
  department: string | null;
  location: string | null;
  category: string | null;
  jd_url: string | null;
  jd_text: string | null;
  salary_min: number | null;
  salary_max: number | null;
  created_at: string;
}

export interface Stage {
  id: number;
  name: string;
  sort_order: number;
}

export interface Application {
  id: number;
  job_id: number;
  channel: string | null;
  resume_version: string | null;
  stage_id: number;
  status: string;
  applied_at: string | null;
  notes: string | null;
  created_at: string;
  updated_at: string;
}

export interface Interview {
  id: number;
  application_id: number;
  round: number;
  format: string | null;
  location: string | null;
  interviewer: string | null;
  scheduled_at: string | null;
  rating: number | null;
  status: string;
  feedback: string | null;
  created_at: string;
}

export interface Attachment {
  id: number;
  interview_id: number;
  kind: string;
  filename: string;
  filepath: string;
  size: number;
  duration: number | null;
  created_at: string;
}

export interface AudioMark {
  id: number;
  interview_id: number;
  attachment_id: number;
  position: number;
  label: string;
  note: string | null;
  created_at: string;
}

export interface ExtractSettings {
  fetchMode: string; // "http" | "browser"
  cookie: string;
  cookieDomain: string;
  browserPath: string;
  provider: string; // "none" | "ollama" | "openai"
  ollamaUrl: string;
  ollamaModel: string;
  openaiUrl: string;
  openaiKey: string;
  openaiModel: string;
  proxy: string;
}

/** 数据保护：回收站 + 镜像备份，两项都可单独开关。 */
export interface ProtectSettings {
  trashEnabled: boolean;
  trashDays: number;
  mirrorEnabled: boolean;
  mirrorDir: string;
}

export interface DataDirInfo {
  path: string;
  // env：环境变量 OFFERFLOW_DATA_DIR；portable：exe 旁的 data/；appData：%APPDATA%\offerflow
  source: "env" | "portable" | "appData";
}

export interface MirrorReport {
  finishedAt: string;
  target: string;
  copiedFiles: number;
  copiedBytes: number;
  unchangedFiles: number;
  error: string | null;
}

export interface OllamaStatus {
  installed: boolean;
  running: boolean;
  url: string;
}

export interface ExtractedJob {
  title: string | null;
  company: string | null;
  salaryMin: number | null;
  salaryMax: number | null;
  location: string | null;
  department: string | null;
  category: string | null;
  summary: string | null;
}

export interface JobDetail {
  title: string;
  company: string;
  jdText: string | null;
  department: string | null;
  location: string | null;
  category: string | null;
  salaryMin: number | null;
  salaryMax: number | null;
}

/** 抓取到但尚未保存的 JD 快照。 */
export interface SnapshotDraft {
  token: string | null;
  sourceUrl: string | null;
  sourceKind: "url" | "file" | "paste";
  text: string;
  detail: JobDetail | null;
  /** structured：页面的 JobPosting 数据；site：已知招聘平台的页面结构；page：仅从页面标题推测。 */
  detailSource: "structured" | "site" | "page" | null;
}

/** 已保存的 JD 快照（保存后不再修改）。 */
export interface JobSnapshot {
  id: number;
  job_id: number;
  source_url: string | null;
  source_kind: string;
  text: string;
  html_path: string | null;
  captured_at: string;
}

/** 岗位表单字段：来自结构化数据、AI 提取或手动填写，统一按字符串编辑。 */
export interface JobFields {
  companyName: string;
  title: string;
  department: string;
  location: string;
  category: string;
  salaryMin: string;
  salaryMax: string;
}

export const EMPTY_JOB_FIELDS: JobFields = {
  companyName: "",
  title: "",
  department: "",
  location: "",
  category: "",
  salaryMin: "",
  salaryMax: "",
};

export const APPLICATION_STATUS = ["推进中", "已获Offer", "未通过筛选", "无回应超时"];
export const INTERVIEW_STATUS = ["待面试", "已完成", "已取消"];
export const INTERVIEW_FORMAT = ["线上视频", "电话", "线下"];

export interface Resume {
  id: number;
  name: string;
  filename: string;
  filepath: string;
  size: number;
  tags: string | null;
  created_at: string;
}

export interface UpcomingInterview {
  id: number;
  round: number;
  scheduled_at: string;
  status: string;
  format: string | null;
  job_title: string;
  company_name: string;
}

export interface ActivityLog {
  id: number;
  application_id: number;
  kind: string;
  detail: string;
  created_at: string;
}
export interface ReviewNote {
  id: number;
  round: number;
  scheduled_at: string | null;
  format: string | null;
  rating: number | null;
  status: string;
  feedback: string;
  job_title: string;
  company_name: string;
  category: string | null;
}

export const CATEGORIES = ["技术类", "产品类", "设计类", "运营类", "市场类", "职能类", "销售类", "数据类", "其他"];

/** 默认分类，加上已有岗位里用过的自定义分类（按出现顺序去重）。 */
export function categoryOptions(jobs: { category: string | null }[]): string[] {
  const custom = jobs.map((j) => j.category?.trim() ?? "").filter((c) => c && !CATEGORIES.includes(c));
  return [...CATEGORIES, ...new Set(custom)];
}
