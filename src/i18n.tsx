import { createContext, useContext, useEffect, useState, type ReactNode } from "react";

export type Lang = "zh" | "en";

// 以中文原文作为 key；英文模式下查表，找不到时回退原文。
const EN: Record<string, string> = {
  "关于 OfferFlow": "About OfferFlow",
  "手动检查 GitHub 最新正式版，下载后自行安装。": "Check GitHub for the latest stable release, then download and install it.",
  "检查更新": "Check for updates",
  "检查中…": "Checking…",
  "查看发布页面": "View releases",
  "下载新版本": "Download update",
  "发现新版本 v{version}": "New version available: v{version}",
  "当前已是最新正式版": "You are on the latest stable version",
  "暂时无法获取版本信息，仓库可能尚未公开或没有正式版。可在浏览器中查看发布页面。": "Release information is unavailable. The repository may be private or have no stable release. Open the releases page in your browser.",
  "GitHub 请求暂时受限，请稍后重试，或打开发布页面。": "GitHub requests are temporarily limited. Try again later or open the releases page.",
  "无法连接 GitHub，请检查网络后重试。": "Cannot connect to GitHub. Check your connection and try again.",
  "收到的版本信息无效，请打开发布页面查看。": "Invalid release information received. Open the releases page to check.",
  "检查更新失败，请稍后重试。": "Update check failed. Please try again later.",
  "无法打开浏览器，请手动访问 GitHub 发布页面。": "Could not open the browser. Visit the GitHub releases page manually.",
  // ===== 经历库 =====
  "复制": "Copy",
  "经历库": "Experience Library",
  "实习 / 工作": "Internships / Work",
  "教育经历": "Education",
  "项目经历": "Projects",
  "研究经历": "Research",
  "作品集": "Portfolio",
  "常用文本": "Reusable text",
  "岗位分类": "Job category",
  "自定义…": "Custom…",
  "输入自定义分类": "Enter a custom category",
  "从列表选": "Pick from list",
  "岗位信息（保存时更新，同一岗位的投递共用）": "Job details (updated on save, shared by applications for this job)",
  "全部": "All",
  "其他经历": "Other Experiences",
  "公司名称": "Company name",
  "职位名称": "Job title",
  "开始时间": "Start date",
  "结束时间": "End date",
  "工作地点": "Work location",
  "学校名称": "School",
  "专业": "Major",
  "学历 / 学位": "Degree",
  "学院": "Faculty",
  "学习形式": "Study mode",
  "主修课程": "Coursework",
  "荣誉奖项": "Honors & Awards",
  "项目名称": "Project name",
  "担任角色": "Role",
  "所属组织": "Organization",
  "研究题目": "Research title",
  "研究状态": "Research status",
  "游戏平台": "Platform",
  "体验年限": "Years of experience",
  "擅长类型": "Preferred genres",
  "作品名称": "Work title",
  "作品链接": "Portfolio URL",
  "语言能力": "Languages",
  "分析方法": "Methods",
  "工具技能": "Tools",
  "复制失败，请选中文字后按 Ctrl+C": "Copy failed. Select the text and press Ctrl+C.",
  "复制{field}": "Copy {field}",
  "描述": "Description",
  "复制描述": "Copy description",
  "尚未填写描述": "No description yet",
  "来源": "Source",
  "复制整段": "Copy entire entry",
  "当前版本": "Current version",
  "{n} 个版本": "{n} versions",
  "对照版本": "Compare with",
  "不对照": "None",
  "放弃尚未保存的经历修改？": "Discard unsaved experience changes?",
  "关闭编辑": "Close editor",
  "{name} 副本 {n}": "{name} copy {n}",
  "删除当前版本？其他版本会保留，保存后生效。": "Remove this version? Other versions are kept. Save to apply.",
  "编辑经历": "Edit experience",
  "新增经历": "New experience",
  "经历名称": "Experience name",
  "经历类型": "Experience type",
  "复制为新版本": "Duplicate version",
  "删除此版本": "Remove version",
  "版本名称": "Version name",
  "例如：通用版、游戏运营版": "e.g. General, Game operations",
  "字段名称 {n}": "Field name {n}",
  "移除{field}": "Remove {field}",
  "+ 添加字段": "+ Add field",
  "基本信息": "Details",
  "所有版本共用这些信息，每项都能单独复制；日期填 YYYY-MM，结束时间可填“至今”。": "Shared by all versions. Copy each field on its own. Dates: YYYY-MM; use 至今 for an ongoing end date.",
  "字段名称": "Field name",
  "字段内容 {n}": "Field value {n}",
  "描述版本": "Description versions",
  "同一段经历可以按岗位写几版描述，例如通用版、游戏运营版。": "Write several descriptions of the same experience for different roles, e.g. General and Game operations.",
  "填写可直接粘贴到招聘表单的文字，保留分段。": "Enter text ready to paste into job forms. Line breaks are preserved.",
  "来源（可选）": "Source (optional)",
  "删除这段经历及其所有版本？此操作无法撤销。": "Delete this experience and all its versions? This cannot be undone.",
  "文件不能超过 10 MB": "File must be under 10 MB",
  "请选择经历 JSON 数据文件": "Select an experience JSON file",
  "已导入 {n} 段经历，重复条目已跳过。": "Imported {n} experiences. Existing entries were skipped.",
  "导入经历": "Import experiences",
  "通用版": "General",
  "+ 新增经历": "+ New experience",
  "搜索经历": "Search experiences",
  "搜索公司、学校、项目或版本内容…": "Search companies, schools, projects or versions…",
  "刷新": "Refresh",
  "{n} 段经历": "{n} experiences",
  "没有匹配的经历": "No matching experiences",
  "把常用经历放在这里": "Keep reusable experience text here",
  "试试其他关键词或分类。": "Try a different search or category.",
  "新增或导入经历后，可以分别复制公司、职位、时间和描述，也可以保留不同版本。": "Add or import experiences to copy companies, roles, dates and descriptions, with separate versions.",
  // ===== 通用 =====
  "加载中…": "Loading…",
  "取消": "Cancel",
  "保存": "Save",
  "保存中…": "Saving…",
  "删除": "Delete",
  "添加": "Add",
  "重命名": "Rename",
  "编辑": "Edit",
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
  "重试": "Retry",
  "数据加载失败": "Failed to load data",
  "恢复确认": "Confirm restore",
  "备份完成": "Backup complete",
  "导出完成：": "Export completed: ",
  "备份已创建：": "Backup created: ",
  "删除投递": "Delete application",
  "删除投递会同时删除面试、复盘和附件，确认继续？": "Delete this application and all interviews, reviews, and attachments?",
  "删除该阶段？仍被投递使用的阶段无法删除。": "Delete this stage? A stage still used by applications cannot be deleted.",
  "确认删除这份简历？开启回收站时，文件会先移到回收站。": "Delete this resume? If the recycle bin is on, the file is moved there first.",
  "中文": "中文",
  "English": "English",

  // ===== 数据位置 =====
  "数据位置": "Data location",
  "打开数据目录": "Open data folder",
  "系统应用数据目录": "System app data folder",
  "程序旁的 data 文件夹（绿色版）": "data folder next to the app (portable)",
  "环境变量 OFFERFLOW_DATA_DIR": "OFFERFLOW_DATA_DIR environment variable",
  "想把数据和程序放在一起，可在 exe 旁新建 data 文件夹；已有数据不会自动搬过去，需要先用备份导出再导入。": "To keep data next to the app, create a data folder beside the exe. Existing data is not moved automatically; export a backup first, then import it.",

  // ===== 界面风格 =====
  "界面风格": "Theme",
  "杂志": "Magazine",
  "简约": "Clean",
  "夜间": "Midnight",
  "波普": "Pop",
  "米白纸面、衬线标题、墨蓝赭石": "Cream paper, serif headlines, navy & ochre",
  "浅灰白底、无衬线、柔和圆角": "Light gray, sans-serif, soft corners",
  "深色护眼、靛蓝与琥珀点缀": "Dark and easy on the eyes, indigo & amber",
  "粗黑描边、硬投影、高饱和撞色": "Thick outlines, hard shadows, loud colors",

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
  "求职": "Job Search",
  "投递": "Applications",
  "投递统计": "Insights",
  "积累": "Library",
  "排列方式": "Layout",
  "简历版本": "Resumes",
  "系统设置": "Settings",
  "公司、岗位与阶段管理": "Company, job, and stage management",
  "公司与岗位": "Companies & Jobs",
  "导出中…": "Exporting…",
  "导出 Excel": "Export Excel",
  "备份与恢复": "Backup & Restore",
  "数据保护": "Data protection",
  "删除的文件先放进回收站": "Move deleted files to the recycle bin first",
  "删除投递、面试、附件或简历时，录音等文件先移到回收站，到期后自动清理。期间可以打开回收站手动找回。": "When you delete an application, interview, attachment or resume, its files (recordings included) go to the recycle bin first and are cleaned up once they expire. Until then you can open the bin and recover them.",
  "回收站保留天数": "Days to keep in the recycle bin",
  "天后自动清理": "days, then clean up automatically",
  "打开回收站": "Open recycle bin",
  "每次启动时自动同步到备份位置": "Sync to a backup location on every launch",
  "把新增的录音、简历、JD 快照和最新数据库同步到另一块硬盘或网盘同步文件夹。只复制新增文件，本地删除不会影响备份；数据库另按天保留最近 14 份。需要时用「从备份恢复」选择其中的 OfferFlow-Mirror 文件夹即可还原。": "Copies new recordings, resumes, JD snapshots and the latest database to another drive or a cloud-synced folder. Only new files are copied, and local deletions never touch the backup; the database is also kept as daily copies for the last 14 days. To restore, choose the OfferFlow-Mirror folder in “Restore from backup”.",
  "尚未选择备份位置": "No backup location chosen",
  "备份位置": "Backup location",
  "选择备份位置": "Choose backup location",
  "选择位置": "Choose…",
  "同步中…": "Syncing…",
  "立即同步": "Sync now",
  "上次同步失败（{time}）：{error}": "Last sync failed ({time}): {error}",
  "上次同步：{time}，新增 {n} 个文件（{size}），{same} 个已存在": "Last sync: {time}, {n} new files ({size}), {same} already there",
  "导出完整备份": "Export full backup",
  "从备份恢复": "Restore from backup",
  "选择备份保存位置": "Choose backup destination",
  "选择 OfferFlow 备份目录": "Choose an OfferFlow backup folder",
  "恢复会在重启后替换当前业务数据，当前数据会保留为 .bak，确认继续？": "Restoring replaces current business data after restart. Current data will be retained as .bak. Continue?",
  "Cookie 仅发送到此域名及其子域名": "Only send the cookie to this domain and its subdomains",
  "简历文档": "Resume documents",
  "面试附件": "Interview attachments",
  "搜索公司 / 岗位 / 部门 / 城市 / 分类 / 渠道":
    "Search company / job / dept / city / category / channel",
  "全部分类": "All categories",
  "快速粘贴链接": "Paste job link",
  "看板": "Board",
  "列表": "List",
  "导出为 Excel": "Export as Excel",
  "{total} 份投递，{active} 份正在推进。": "{total} applications, {active} in progress.",
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
  "投递渠道": "Channel",
  "选择渠道": "Choose a channel",
  "输入投递渠道": "Enter a channel",
  "官网": "Company website",
  "内推": "Referral",
  "请填写岗位名称": "Enter a job title",
  "如 Boss直聘 / 官网": "e.g. Boss / official site",
  "当前阶段": "Stage",
  "备注": "Notes",
  "面试复盘": "Interview Notes",
  "请先粘贴岗位链接": "Paste a job link first",
  "请选择公司与岗位": "Select a company and a job",
  "今天": "Today",

  // ===== 公司与岗位管理 =====
  "新增阶段名称": "New stage name",
  "公司名称 *": "Company name *",
  "行业（可选）": "Industry (optional)",
  "城市（可选）": "City (optional)",
  "添加公司": "Add Company",
  "暂无公司": "No companies",
  "抓取中…": "Fetching…",
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

  // ===== 数据看板 · 面试安排 =====
  "近 {n} 天面试安排": "Interviews in the next {n} days",
  "暂无待办面试": "No upcoming interviews",

  // ===== 新建投递 =====
  "请先在「公司与岗位」中添加至少一个阶段": "Add at least one stage under Companies & Jobs first",

  // ===== JD 快照 =====
  "JD 快照": "JD Snapshot",
  "JD 快照（可选）": "JD snapshot (optional)",
  "岗位链接": "Job link",
  "网页文件": "Web page file",
  "粘贴文本": "Paste text",
  "粘贴岗位页面链接": "Paste the job page link",
  "抓取快照": "Capture",
  "Chrome 调试连接": "Chrome debug connection",
  "从 Chrome 抓取": "Capture from Chrome",
  "抓取 Chrome 当前 BOSS 职位": "Capture open BOSS job",
  "在 Chrome 展开目标职位，直接切回这里抓取正文。": "Open the job details in Chrome, then switch back here to capture the text.",
  "在 Chrome 展开目标职位，复制地址栏完整链接。首次使用请在 Chrome 144+ 的 chrome://inspect/#remote-debugging 启用远程调试，抓取时允许连接。": "Open the job details in Chrome and copy the full address. On first use, enable remote debugging at chrome://inspect/#remote-debugging in Chrome 144 or newer, then allow the connection when capturing.",
  "读取中…": "Reading…",
  "选择 .html / .mhtml 文件": "Choose .html / .mhtml file",
  "选择保存的网页文件": "Choose a saved web page",
  "在浏览器中打开岗位页，按 Ctrl+S 保存为「网页，单个文件」即可。需要登录才能看到的页面也适用。": "Open the job page in your browser and press Ctrl+S to save it as \"Webpage, Single File\". Works for pages that require login, too.",
  "把岗位描述全文粘贴到这里": "Paste the full job description here",
  "来源链接（可选）": "Source link (optional)",
  "使用这段文本": "Use this text",
  "请先粘贴 JD 文本": "Paste the job description first",
  "来源地址必须是 http:// 或 https:// 开头的网址": "The source link must start with http:// or https://",
  "快照已获取": "Snapshot captured",
  "已从页面结构化数据识别岗位信息，请核对": "Job details found in the page's structured data. Please review.",
  "快照已获取。未启用 AI 提取，请手动填写岗位信息": "Snapshot captured. AI extraction is off, so fill in the job details manually.",
  "AI 提取中…": "Extracting with AI…",
  "AI 未识别出岗位名称，请手动填写": "AI couldn't find a job title. Please fill it in.",
  "AI 提取失败，请手动填写：": "AI extraction failed, please fill in manually: ",
  "快照已获取，页面里没有结构化岗位信息。可以手动填写，或使用 AI 提取。": "Snapshot captured, but the page has no structured job data. Fill in the details manually, or use AI extraction.",
  "已从页面内容识别岗位信息，请核对": "Job details read from the page content. Please review.",
  "已从页面标题推测出岗位名称，可能不准确，请核对。其余信息可以手动填写，或使用 AI 提取。": "Job title guessed from the page heading and may be inaccurate. Please review it. Fill in the rest manually, or use AI extraction.",
  "已从页面标题推测出岗位名称，可能不准确，请核对并补全其余信息": "Job title guessed from the page heading and may be inaccurate. Please review it and fill in the rest.",
  "用 {name} 提取岗位信息": "Extract job details with {name}",
  "正文会发送到 {host}，发送前会再次确认": "Text will be sent to {host}; you'll be asked to confirm first",
  "在本机运行，正文不会离开这台电脑": "Runs locally; the text never leaves this computer",
  "发送到外部服务": "Send to external service",
  "发送并提取": "Send and extract",
  "将把快照正文（约 {n} 字）发送到 {host} 进行识别。该服务可能保存所收到的内容，也可能按用量计费。确认发送？": "About {n} characters of snapshot text will be sent to {host} for extraction. The service may keep what it receives and may charge by usage. Send it?",
  "已由 AI 提取岗位信息，只填入了空着的字段，请核对": "Job details extracted by AI. Only empty fields were filled in. Please review.",
  "抓取快照不会自动调用 AI。能从页面本身识别岗位信息时会直接填入；识别不到时，启用后会出现「用 AI 提取」按钮，由你决定是否使用。": "Capturing a snapshot never calls AI on its own. Job details that can be read from the page itself are filled in directly; otherwise, when enabled, an \"Extract with AI\" button appears and it's up to you whether to use it.",
  "使用时快照正文会发送到下面的接口，每次发送前都会先征求你的确认。": "When used, snapshot text is sent to this endpoint. You'll be asked to confirm every time before anything is sent.",
  "JD 正文": "Job description",
  "新公司（在下方填写）": "New company (fill in below)",
  "新岗位（在下方填写）": "New job (fill in below)",
  "新岗位信息（保存时创建）": "New job details (created on save)",
  "已匹配到现有岗位，快照会追加到该岗位的快照记录里。": "Matched an existing job; the snapshot will be added to its history.",
  "薪资下限": "Minimum salary",
  "薪资上限": "Maximum salary",
  "薪资下限 k": "Min salary (k)",
  "薪资上限 k": "Max salary (k)",
  "{name}必须是非负整数": "{name} must be a non-negative whole number",
  "薪资下限不能高于上限": "Minimum salary cannot exceed maximum",
  "请选择公司，或填写公司名称": "Select a company or enter a company name",
  "请选择已有岗位，或填写新岗位名称": "Select an existing job or enter a new job title",
  "快照": "Snapshots",
  "+ 新增快照": "+ New snapshot",
  "保存快照": "Save snapshot",
  "原链接": "Source link",
  "打开网页原件": "Open saved page",
  "复制文本": "Copy text",
  "已复制": "Copied",
  "删除这份快照？删除后无法恢复。": "Delete this snapshot? This cannot be undone.",
  "这个岗位还没有 JD 快照。岗位下架后链接会失效，建议投递时顺手存一份。": "No snapshots for this job yet. Job links stop working once a posting is taken down, so it's worth saving one when you apply.",
  "链接抓取": "Captured from link",
  "旧版 JD 文本": "Legacy JD text",
  "删除公司会同时删除其岗位、投递、面试、附件和 JD 快照，确认继续？": "Delete this company and all related jobs, applications, interviews, attachments, and JD snapshots?",
  "删除岗位会同时删除相关投递、面试、附件和 JD 快照，确认继续？": "Delete this job and all related applications, interviews, attachments, and JD snapshots?",
  "JD 快照抓取": "JD snapshot capture",
  "粘贴岗位链接时先按下面的方式访问页面；拿不到岗位内容（例如 Moka 这类由脚本加载的页面）时，会自动在后台窗口里打开页面再读取。需要登录或验证时，那个窗口会显示出来让你处理。": "Pasted links are fetched as set below first. If that returns no job content (e.g. script-rendered pages like Moka), the page is opened in a background window and read from there. If it needs a login or verification, that window is shown so you can finish it.",
  "由脚本加载的页面需要在后台打开，可能要十几秒。": "Script-rendered pages are opened in the background and may take 10–20 seconds.",
  "AI 信息提取（可选）": "AI extraction (optional)",
  "提取方式": "Extraction method",
  "不启用（保存快照，手动填写）": "Off (save snapshot, fill in manually)",
  "Ollama（本地模型，数据不出本机）": "Ollama (local model, data stays on this computer)",
  "API 调用（OpenAI 兼容接口）": "API (OpenAI-compatible)",
  "任何兼容 OpenAI Chat Completions 的服务均可，如 DeepSeek、通义千问、Moonshot 等；远程地址必须使用 HTTPS": "Any OpenAI Chat Completions-compatible service works, such as DeepSeek, Qwen, or Moonshot. Remote endpoints must use HTTPS",
  "备份包含业务数据、简历、附件和 JD 快照，不包含 Cookie 与 API Key。": "Backups include your data, resumes, attachments, and JD snapshots, but not cookies or API keys.",
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
  "面试日期与时间": "Interview date and time",
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
  "抓取方式": "Fetch method",
  "HTTP 请求": "HTTP request",
  "无头浏览器（渲染后抓取）": "Headless browser (render & fetch)",
  "浏览器路径（留空自动检测 Edge/Chrome）": "Browser path (blank = auto-detect Edge/Chrome)",
  "Cookie（登录态注入，可选）": "Cookie (session injection, optional)",
  "代理（可选，如 http://127.0.0.1:7890）": "Proxy (optional, e.g. http://127.0.0.1:7890)",
  "Ollama 地址": "Ollama URL",
  "模型": "Model",
  "获取中…": "Fetching…",
  "获取模型": "Fetch models",
  "接口地址": "API URL",
  "模型名": "Model name",
  "已保存": "Saved",
  "Ollama 未返回模型，请检查服务是否启动，或手动输入模型名":
    "Ollama returned no models. Check the service or type a model name manually.",
  "已获取 {n} 个模型，可从下拉选择或手动输入":
    "{n} models found. Pick from the list or type manually.",
  "可手动输入模型名（适用于手动下载的模型）":
    "Type a model name manually (for manually downloaded models)",
  "Ollama 服务状态": "Ollama service status",
  "重新检测": "Re-check",
  "检测中…": "Detecting…",
  "服务运行中，可直接获取模型": "Service running — you can fetch models",
  "已安装但未启动，请运行 ollama serve": "Installed but not running. Run: ollama serve",
  "未检测到 Ollama，请先安装：": "Ollama not detected. Install it first: ",

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
