# OfferFlow

求职投递管理系统：用看板管理投递全流程，粘贴职位链接自动解析，沉淀面试复盘与简历版本。

## 功能特性

- 看板式投递管理：以阶段（待投递 / 简历初筛 / 技术一面 …）为单位组织投递，卡片可拖拽换阶段
- 阶段自由自定义：阶段可增删改，并支持上移 / 下移排序，适配不同公司的招聘流程
- JD 网页快照：粘贴岗位链接、导入浏览器另存的网页（.html / .mhtml），或直接粘贴文本，把投递时的 JD 原样存档；岗位下架后仍可回看
- 岗位信息识别：页面带有 schema.org JobPosting 结构化数据时直接识别，否则手动填写
- AI 信息提取（可选）：可接入本地 Ollama，或任意 OpenAI 兼容接口（填写地址与 API Key），从快照正文中提取公司、岗位、薪资与分类
- 公司与岗位管理：公司、岗位均可增删改，误解析的记录可直接修正而不必删掉整条投递
- 多元搜索与筛选：按公司 / 岗位 / 部门 / 城市 / 分类 / 渠道搜索；岗位支持分类属性（技术类 / 职能类 / 市场类等）
- 投递时间线：自动记录每个投递的创建、阶段流转、状态变更
- 简历版本管理：导入简历、系统默认程序预览、重命名、打标签
- 面试台账：记录面试安排与过程，支持录音与音频标注
- 数据看板与复盘题库：投递漏斗统计 + 近 14 天面试安排 + 面试题沉淀
- 备份与导出：一键导出 Excel 投递记录，支持完整备份 / 恢复

## 技术栈

- 前端：React 19 + TypeScript + Vite + Tailwind CSS + TipTap + dnd-kit
- 后端：Rust + Tauri 2
- 存储：SQLite（rusqlite，本地单文件）
- 其他：rust_xlsxwriter（Excel 导出）、ureq + encoding_rs（页面抓取，兼容 GBK 页面）、Ollama / OpenAI 兼容接口（AI 提取，可选）

## 目录结构

    .
    ├─ src/                  # 前端源码（React + TS）
    │   ├─ components/       # 视图组件
    │   ├─ api.ts            # 后端命令封装
    │   └─ types.ts          # 类型与分类定义
    ├─ src-tauri/            # 后端源码（Rust）
    │   ├─ src/              # 命令 / 数据库 / 爬取逻辑
    │   ├─ capabilities/     # Tauri 权限配置
    │   ├─ icons/            # 应用图标
    │   └─ Cargo.toml
    ├─ dist/                 # 前端构建产物（自动生成，勿提交）
    ├─ data/                 # 本地数据（隐私，勿提交）
    ├─ index.html            # 入口页
    ├─ package.json
    └─ vite.config.ts

## 环境要求

- Node.js 20+ 与 npm
- Rust 工具链（rustup + stable）
- Tauri 2 系统依赖（Windows 需安装 VS Build Tools，详见 Tauri 官方文档）
- 平台自带 WebView2 运行时（Windows 10/11 已内置）

## 快速开始

安装依赖：

    npm install

本地开发（同时启动 Vite 与 Rust 后端，支持热更新）：

    npm run tauri dev

## 构建与绿色版

一键打包（生成安装包）：

    npm run tauri build

绿色版（免安装）：构建产物中的单文件可执行程序即可直接运行：

    src-tauri\target\release\offerflow.exe

将它复制到任意文件夹（或 U 盘 / 其他电脑）双击即可启动，无需安装、不依赖同级 DLL。

## 数据存储

业务数据目录按以下顺序选择：

1. `OFFERFLOW_DATA_DIR` 环境变量指定的可写目录
2. 可执行文件旁**已存在**的 `data/`（绿色版：把 `data/` 和 exe 放一起带走即可）
3. 系统应用数据目录 `%APPDATA%\offerflow`

第 2 条要求目录已经存在，否则数据目录会跟着 exe 的位置走，同一台机器上从不同位置启动会各自生成一份数据。

首次启动时，若检测到 0.1 版遗留的 `D:/OfferFlow/data` 且新目录尚无数据库，会自动把数据迁移过去（原目录保留不动）。

    offerflow.db     # SQLite 数据库：投递 / 公司 / 岗位 / 简历 / 面试 / 设置
    resumes/         # 简历文件
    attachments/     # 面试附件
    snapshots/       # JD 网页快照原件
    settings.json            # 非敏感抓取与 AI 配置
    settings.secrets.json    # Windows DPAPI 加密的 Cookie / API Key

请勿将 data/ 目录提交到公开仓库（含个人求职隐私）。

## 使用说明

1. 新建投递：点击「新建投递」，粘贴岗位链接抓取快照（或导入网页文件 / 粘贴文本），核对岗位信息后保存
2. 自定义流程：在「公司与岗位」中新增 / 重命名 / 排序阶段
3. 简历：在「简历版本」导入简历，支持预览、重命名、打标签
4. AI 提取（可选）：在「系统设置」选择 Ollama，或选择「API 调用」并填写接口地址、API Key 与模型名

## 关于页面抓取

OfferFlow 只做普通的页面访问：按设置用 HTTP 请求或本机 Edge / Chrome 无头模式打开链接，保存看到的内容，不调用站点私有接口、不绕过任何加密或访问控制。需要登录才能查看的页面，建议在浏览器中登录后「另存为」网页再导入。请遵守目标站点的使用条款，快照仅供个人求职记录使用。

## 数据安全与备份

- Cookie 只会发送到设置中指定的域名及其子域名；岗位抓取拒绝本机、私网及非 HTTP(S) 地址，且重定向的每一跳都会重新校验
- 简历与附件只能通过后端命令打开，路径必须落在当前数据目录内
- Windows 下 Cookie 和 API Key 使用 DPAPI 加密保存，不会写入普通设置文件或备份
- 完整备份可在「系统设置」中导出；每次导出创建独立目录，包含数据库、简历与附件
- 恢复前会执行 SQLite 完整性、外键和架构版本检查；当前数据库及文件目录保留为 `.bak`
- 删除公司、岗位、投递或面试时，会同步清理其托管附件文件
- 恢复失败时待恢复文件会自动归档为 `.failed-*`，不会让应用卡在打不开的状态

## 质量检查

提交代码前运行：

    npm run check
    cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
    cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
    cargo test --manifest-path src-tauri/Cargo.toml

GitHub Actions 会在 push 和 pull request 时执行同样的检查。

## License

本项目采用 [MIT License](./LICENSE) 开源协议。
