# Changelog

## v1.0.1 — 2026-11-03

- Windows 上可以从已打开的 Chrome 一键保存 BOSS 直聘职位，识别公司、岗位、城市和月薪。快照保留职责、要求和薪数，自动排除排名和推荐内容。
- 新建投递直接填入抓取到的岗位名，可手动编辑。同名岗位可分别保存薪资、链接和快照。
- 投递渠道支持下拉选择、历史渠道和自定义输入。BOSS 直聘链接会自动识别渠道。
- 精简经历库工具栏，移除复制操作提示。

### English

- Capture an open BOSS Zhipin posting from Chrome on Windows, including employer, title, location and salary. Snapshots keep the job description and omit rankings and recommendations.
- New applications use the captured title as editable text. Postings with the same title can keep separate salaries, links and snapshots.
- Choose a previous application channel or enter a new one. BOSS Zhipin links fill the channel automatically.
- Removed the copy instructions from the experience library toolbar.

## v1.0.0 — 2026-10-31

OfferFlow 首个正式版本，面向 Windows x64，支持中文与英文界面。

- 用看板或列表管理投递，支持自定义阶段、岗位分类和筛选。
- 从岗位链接、网页文件或文本保存 JD 快照，支持动态页面后台加载与可选 AI 信息提取。
- 管理面试安排、录音、音频标注与复盘笔记。
- 整理简历和经历，维护多份描述版本，按字段复制。
- 提供投递统计、Excel 导出、完整备份与恢复，以及可选增量镜像备份。
- 在「系统设置 → 关于 OfferFlow」显示当前版本，手动检查 GitHub 最新正式版并打开下载页面。
- 修复备份测试并行执行时临时目录可能重名的问题。

### 下载与更新

- `OfferFlow_1.0.0_x64-setup.exe`：NSIS 安装包。
- `OfferFlow_1.0.0_x64_en-US.msi`：MSI 安装包。
- `OfferFlow_1.0.0_x64.exe`：免安装程序；首次启动前在程序旁创建 `data/` 文件夹，可将新数据保存在该文件夹。
- `SHA256SUMS.txt`：下载文件的 SHA-256 校验值。

需要 Microsoft Edge WebView2 运行时。更新检测只读取版本信息，下载和安装由用户完成。已有数据继续使用原数据目录；改用其他数据目录时，可通过完整备份与恢复迁移。

### English

The first stable release of OfferFlow, for Windows x64 with Chinese and English interfaces.

- Track applications in Kanban or list views with custom stages, job categories, and filters.
- Archive job descriptions from links, saved web pages, or text, with background loading for dynamic pages and optional AI extraction.
- Manage interview schedules, recordings, audio annotations, and review notes.
- Organize resumes and experience entries, compare description versions, and copy individual fields.
- View application statistics, export Excel files, create full backups, restore data, and optionally mirror files incrementally.
- See the current version in Settings → About OfferFlow, check GitHub for stable releases, and open the download page.
- Fix temporary directory collisions in parallel backup tests.

Choose the NSIS installer (`*-setup.exe`), the MSI installer (`*.msi`), or the standalone executable (`OfferFlow_1.0.0_x64.exe`). `SHA256SUMS.txt` contains file checksums. The Microsoft Edge WebView2 Runtime is required.

Updates are downloaded and installed manually. Existing data stays in its original location. Use full backup and restore when moving to a different data directory. For a portable setup, create a `data/` folder beside the executable before the first launch.
