import { createContext, useContext, useEffect, useState, type ReactNode } from "react";

export type Lang = "zh" | "en";

// 以中文原文作为 key；英文模式下查表，找不到时回退原文。
const EN: Record<string, string> = {
  // ===== 通用 =====
  "加载中…": "Loading…",
  "取消": "Cancel",
  "保存": "Save",
  "保存中…": "Saving…",
  "删除": "Delete",
  "添加": "Add",
  "重命名": "Rename",
  "预览": "Preview",
  "打开": "Open",
  "关闭": "Close",
  "未知": "Unknown",
  "未命名": "Untitled",
  "选择公司": "Select company",
  "选择岗位": "Select job",
  "选择分类": "Select category",
  "暂无数据": "No data",
  "可选": "Optional",
  "字节": "bytes",
  "中文": "中文",
  "English": "English",

  // ===== 投递状态 =====
  "推进中": "In Progress",
  "已获Offer": "Offer",
  "未通过筛选": "Rejected",
  "无回应超时": "No Response",

  // ===== 默认阶段 =====
  "待投递": "To Apply",
  "简历筛选": "Screening",
  "笔试": "Assessment",
  "一面": "Round 1",
  "二面": "Round 2",
  "HR面": "HR Round",
  "Offer": "Offer",

  // ===== 分类 =====
  "技术类": "Engineering",
  "产品类": "Product",
  "设计类": "Design",
  "运营类": "Operations",
  "市场类": "Marketing",
  "职能类": "Corporate",
  "销售类": "Sales",
  "数据类": "Data",
  "其他": "Other",
  "未分类": "Uncategorized",

  // ===== 面试 =====
  "待面试": "Upcoming",
  "已完成": "Completed",
  "已取消": "Cancelled",
  "线上视频": "Video",
  "电话": "Phone",
  "线下": "Onsite",
  "音频": "Audio",
  "视频": "Video",
  "文件": "File",

  // ===== 侧边栏 / 顶栏 =====
  "+ 新建投递": "+ New Application",
  "视图": "Views",
  "全部看板": "Kanban Board",
  "列表明细": "List View",
  "数据看板": "Dashboard",
  "沉淀": "Archive",
  "复盘题库": "Review Bank",
  "简历版本": "Resumes",
  "系统设置": "Settings",
  "公司与岗位": "Companies & Jobs",
  "导出中…": "Exporting…",
  "导出 Excel": "Export Excel",
  "搜索公司 / 岗位 / 部门 / 城市 / 分类 / 渠道":
    "Search company / job / dept / city / category / channel",
  "全部分类": "All categories",
  "快速粘贴链接": "Paste job link",
  "看板": "Board",
  "列表": "List",
  "导出为 Excel": "Export as Excel",
  "导出失败：": "Export failed: ",

  // ===== 看板 =====
  "未知岗位": "Unknown job",
  "未知公司": "Unknown company",
  "暂无投递": "No applications",

  // ===== 列表 =====
  "公司": "Company",
  "岗位": "Job",
  "分类": "Category",
  "部门": "Department",
  "城市": "City",
  "渠道": "Channel",
  "阶段": "Stage",
  "状态": "Status",
  "投递日期": "Applied date",
  "暂无投递记录": "No application records",

  // ===== 数据看板 =====
  "投递总数": "Total",
  "已获 Offer": "Offers",
  "覆盖公司": "Companies",
  "投递阶段分布": "Stage distribution",
  "岗位分类分布": "Category distribution",
  "状态分布": "Status distribution",

  // ===== 投递弹窗 =====
  "编辑投递": "Edit Application",
  "新建投递": "New Application",
  "从链接解析": "Parse from link",
  "粘贴岗位链接，如 MokaHR / 官网 / Boss 链接":
    "Paste a job link (MokaHR / career site / Lever / Greenhouse)",
  "解析": "Parse",
  "解析中…": "Parsing…",
  "解析后自动匹配或新建公司与岗位": "Auto-match or create company & job after parsing",
  "投递渠道": "Channel",
  "如 Boss直聘 / 官网": "e.g. Boss / official site",
  "当前阶段": "Stage",
  "备注": "Notes",
  "面试复盘": "Interview Review",
  "请先粘贴岗位链接": "Paste a job link first",
  "未能解析出岗位名称": "Could not parse the job title",
  "未能创建公司与岗位记录": "Could not create company & job records",
  "请选择公司与岗位": "Select a company and a job",

  // ===== 公司与岗位管理 =====
  "新增阶段名称": "New stage name",
  "公司名称 *": "Company name *",
  "行业（可选）": "Industry (optional)",
  "城市（可选）": "City (optional)",
  "添加公司": "Add Company",
  "暂无公司": "No companies",
  "URL 抓取 / 信息提取": "URL fetch / info extraction",
  "粘贴职位链接，点击抓取": "Paste a job link, then fetch",
  "抓取": "Fetch",
  "抓取中…": "Fetching…",
  "JD 描述文本（可手动粘贴，或从链接抓取）": "JD description (paste manually or fetch from link)",
  "提取中…": "Extracting…",
  "提取信息填充": "Extract & fill",
  "所属公司": "Company",
  "识别公司：": "Detected company: ",
  "（请在上方选择或到「公司」页新增）": "(select above or add in Companies tab)",
  "岗位名称 *": "Job title *",
  "部门（可选）": "Department (optional)",
  "岗位分类（可选）": "Category (optional)",
  "薪资下限（k/月，可选）": "Salary min (k/month, optional)",
  "薪资上限（k/月，可选）": "Salary max (k/month, optional)",
  "如 12": "e.g. 12",
  "如 20": "e.g. 20",
  "添加岗位": "Add Job",
  "岗位列表": "Job list",
  "请先选择公司": "Select a company first",

  // ===== 简历 =====
  "简历版本（{n}）": "Resumes ({n})",
  "导入中…": "Importing…",
  "+ 导入简历": "+ Import resume",
  "简历名称": "Resume name",
  "标签，用逗号分隔": "Tags, comma separated",
  "暂无简历版本，导入后可在这里预览、命名和打标签":
    "No resumes yet. Import one to preview, rename and tag.",
  "选择简历文件": "Select resume file",

  // ===== 复盘题库 =====
  "已沉淀 {n} 条复盘记录": "{n} review notes archived",
  "第 {n} 轮": "Round {n}",
  "暂无复盘记录，在面试中填写复盘笔记后会自动沉淀到这里":
    "No review notes yet. Notes saved during interviews appear here.",

  // ===== 面试复盘 =====
  "+ 新增一轮": "+ Add round",
  "暂无面试记录，点击右上角「新增一轮」开始复盘。": "No interviews yet. Click \"+ Add round\" to start.",
  "删除本轮": "Delete round",
  "地点": "Location",
  "如 线上/北京": "e.g. Online/Beijing",
  "面试官": "Interviewer",
  "姓名/职位": "Name / title",
  "面试日期": "Date",
  "评分 (1-5)": "Rating (1-5)",
  "复盘笔记": "Review notes",
  "附件（{n}）": "Attachments ({n})",
  "+ 导入附件": "+ Import attachment",
  "无附件": "No attachments",
  "保存本轮": "Save round",
  "删除后该轮面试及其复盘笔记、附件将一并删除，确认？":
    "This round and its notes/attachments will be deleted. Continue?",
  "删除确认": "Confirm delete",

  // ===== 设置 =====
  "设置 · 抓取与信息提取": "Settings · Fetch & Extraction",
  "岗位 URL 抓取": "Job URL fetching",
  "抓取方式": "Fetch method",
  "HTTP 请求": "HTTP request",
  "无头浏览器（渲染后抓取）": "Headless browser (render & fetch)",
  "浏览器路径（留空自动检测 Edge/Chrome）": "Browser path (blank = auto-detect Edge/Chrome)",
  "Cookie（登录态注入，可选）": "Cookie (session injection, optional)",
  "代理（可选，如 http://127.0.0.1:7890）": "Proxy (optional, e.g. http://127.0.0.1:7890)",
  "信息提取模型": "Extraction model",
  "模型提供商": "Model provider",
  "不启用（纯手动录入）": "Disabled (manual entry)",
  "Ollama（本地离线 / 服务）": "Ollama (local / server)",
  "OpenAI 兼容接口": "OpenAI-compatible API",
  "Ollama 地址": "Ollama URL",
  "模型": "Model",
  "获取中…": "Fetching…",
  "获取模型": "Fetch models",
  "模型下载镜像源（可选，供模型下载加速）": "Model mirror (optional, for download acceleration)",
  "接口地址": "API URL",
  "模型名": "Model name",
  "已保存": "Saved",
  "Ollama 未返回模型，请检查服务是否启动":
    "Ollama returned no models. Check that the service is running.",
  "已获取 {n} 个模型，已选择首个": "{n} models found, first one selected",

  // ===== 时间线 =====
  "投递时间线": "Application timeline",
  "添加一条备注记录…": "Add a note…",
  "暂无记录": "No records",

  // ===== 音频复盘 =====
  "音频复盘 · {filename}": "Audio review · {filename}",
  "音频解码中…": "Decoding audio…",
  "暂停": "Pause",
  "播放": "Play",
  "打点标签": "Marker label",
  "备注（可选）": "Note (optional)",
  "打点 @ {time}": "Mark @ {time}",
  "请填写打点标签": "Enter a marker label",

  // ===== 富文本 =====
  "加粗": "Bold",
  "斜体": "Italic",
  "删除线": "Strikethrough",
  "无序列表": "Bullet list",
  "有序列表": "Ordered list",
  "标题2": "Heading 2",
  "标题3": "Heading 3",
  "撤销": "Undo",
  "重做": "Redo",
  "音频加载失败": "Audio load failed",
};

interface I18nValue {
  lang: Lang;
  setLang: (l: Lang) => void;
  t: (key: string, params?: Record<string, string | number>) => string;
}

const I18nContext = createContext<I18nValue>({
  lang: "zh",
  setLang: () => {},
  t: (k) => k,
});

function applyParams(s: string, params?: Record<string, string | number>): string {
  if (!params) return s;
  let out = s;
  for (const [k, v] of Object.entries(params)) {
    out = out.split(`{${k}}`).join(String(v));
  }
  return out;
}

export function I18nProvider({ children }: { children: ReactNode }) {
  const [lang, setLang] = useState<Lang>(() => {
    try {
      return localStorage.getItem("offerflow.lang") === "en" ? "en" : "zh";
    } catch {
      return "zh";
    }
  });

  useEffect(() => {
    try {
      localStorage.setItem("offerflow.lang", lang);
    } catch {
      /* ignore */
    }
  }, [lang]);

  const t = (key: string, params?: Record<string, string | number>): string => {
    const base = lang === "en" ? (EN[key] ?? key) : key;
    return applyParams(base, params);
  };

  return <I18nContext.Provider value={{ lang, setLang, t }}>{children}</I18nContext.Provider>;
}

export function useI18n(): I18nValue {
  return useContext(I18nContext);
}