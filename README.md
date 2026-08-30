# OfferFlow

求职投递管理系统：用看板管理投递全流程，粘贴职位链接自动解析，沉淀面试复盘与简历版本。

## 功能特性

- 看板式投递管理：以阶段（待投递 / 简历初筛 / 技术一面 …）为单位组织投递，卡片可拖拽换阶段
- 阶段自由自定义：阶段可增删改，并支持上移 / 下移排序，适配不同公司的招聘流程
- 链接一键解析：粘贴职位链接自动抓取职位、公司、薪资、JD 内容（已适配 MokaHR 等）
- AI 信息提取（可选）：配置 Ollama 或 OpenAI，自动从 JD 提取结构化信息与岗位分类
- 多元搜索与筛选：按公司 / 岗位 / 部门 / 城市 / 分类 / 渠道搜索；岗位支持分类属性（技术类 / 职能类 / 市场类等）
- 投递时间线：自动记录每个投递的创建、阶段流转、状态变更
- 简历版本管理：导入简历、系统默认程序预览、重命名、打标签
- 面试台账：记录面试安排与过程，支持录音与音频标注
- 数据看板与复盘题库：投递漏斗统计 + 面试题沉淀
- 备份与导出：一键导出 Excel 投递记录，支持完整备份 / 恢复

## 技术栈

- 前端：React 19 + TypeScript + Vite + Tailwind CSS + TipTap + dnd-kit
- 后端：Rust + Tauri 2
- 存储：SQLite（rusqlite，本地单文件）
- 其他：rust_xlsxwriter（Excel 导出）、ureq（HTTP 抓取）、Ollama / OpenAI（AI 提取，可选）

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

- Node.js 18+ 与 npm
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

业务数据默认放在本地目录 D:/OfferFlow/data（代码设定；若该目录不可写，则回退到 %APPDATA%\offerflow）：

    offerflow.db     # SQLite 数据库：投递 / 公司 / 岗位 / 简历 / 面试 / 设置
    resumes/         # 简历文件
    attachments/     # 面试附件
    settings.json    # AI 提取配置（Ollama / OpenAI）

请勿将 data/ 目录提交到公开仓库（含个人求职隐私）。

## 使用说明

1. 新建投递：点击「新建投递」或「快速粘贴链接」，粘贴职位链接自动解析
2. 自定义流程：在「公司与岗位」中新增 / 重命名 / 排序阶段
3. 简历：在「简历版本」导入简历，支持预览、重命名、打标签
4. AI 提取（可选）：在「系统设置」配置 Ollama 地址或 OpenAI Key

## License

（按需补充）
