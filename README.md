# OfferFlow

[![Release](https://img.shields.io/github/v/release/iTakeRu/OfferFlow)](https://github.com/iTakeRu/OfferFlow/releases/latest)
[![CI](https://github.com/iTakeRu/OfferFlow/actions/workflows/ci.yml/badge.svg)](https://github.com/iTakeRu/OfferFlow/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/github/license/iTakeRu/OfferFlow)](./LICENSE)

**简体中文** | [English](./README.en.md)

本地运行的 Windows 求职投递管理工具：管理投递流程、存档职位描述（JD）、记录面试与复盘、整理简历和经历。支持中文 / English 界面与主题切换，求职资料保存在本地。

<!-- 截图：放入 docs/screenshots/ 后取消注释
![投递看板](./docs/screenshots/board.png)
-->

## 功能

- **投递**：看板 / 列表两种视图，拖拽卡片切换阶段；阶段和岗位分类可自定义；按公司、岗位、城市、分类、渠道搜索筛选
- **JD 快照**：通过岗位链接、网页文件（.html / .mhtml）或粘贴文本存档，岗位下架后仍可回看；支持 HTTP、无头浏览器抓取，内容未能提取时自动尝试后台窗口加载
- **岗位识别**：从页面结构或 JobPosting 结构化数据提取岗位信息；可选接入 Ollama 或 OpenAI 兼容接口进行 AI 提取
- **面试复盘**：记录每轮面试，支持录音、音频标注与富文本复盘笔记
- **投递统计**：阶段分布、分类分布、未来 14 天面试安排
- **简历版本**：导入、预览、重命名、打标签
- **经历库**：分类整理工作、教育、项目等经历，共用基础信息，维护和对照多个描述版本；支持逐项复制、JSON 导入及完整备份
- **数据保护**：删除的文件先进回收站；可选把数据增量同步到另一块硬盘或网盘文件夹；数据库升级前自动留副本
- **导出与备份**：导出 Excel；完整备份 / 恢复

<!-- 截图：放入 docs/screenshots/ 后取消注释
| JD 快照 | 面试复盘 | 经历库 |
|---|---|---|
| ![JD 快照](./docs/screenshots/snapshot.png) | ![面试复盘](./docs/screenshots/interview.png) | ![经历库](./docs/screenshots/experience.png) |
-->

## 安装

[下载最新版本](https://github.com/iTakeRu/OfferFlow/releases/latest)。v1.0.2 提供以下四个发布文件，前三项任选其一；`SHA256SUMS.txt` 用于校验下载文件。源码构建方式见下方「开发」。

| 发布文件 | 说明 |
|---|---|
| `OfferFlow_1.0.2_x64-setup.exe` | NSIS 安装包 |
| `OfferFlow_1.0.2_x64_en-US.msi` | MSI 安装包 |
| `OfferFlow_1.0.2_x64.exe` | 免安装程序，下载后直接运行，无需改名 |
| `SHA256SUMS.txt` | 上述三个文件的 SHA-256 校验值 |

**运行环境**

- Windows x64，需要 [Microsoft Edge WebView2 运行时](https://developer.microsoft.com/microsoft-edge/webview2/)（Windows 11 已自带，Windows 10 多数已预装）
- 通过无头浏览器抓取 JD 时需要本机安装 Edge 或 Chrome

**首次运行提示**：程序暂未进行代码签名，Windows SmartScreen 可能提示「Windows 已保护你的电脑」，点击「更多信息 → 仍要运行」即可。如需确认下载文件未被篡改，可与发布页的 `SHA256SUMS.txt` 对照：

```powershell
certutil -hashfile OfferFlow_1.0.2_x64-setup.exe SHA256
```

**检查更新**：「系统设置 → 关于 OfferFlow」显示当前版本，可手动检查更新并打开发布页面下载。更新检测只读取 GitHub 正式版信息，下载和安装由用户自行完成。

## 快速上手

1. 新建一条投递，填写公司和岗位，或直接粘贴岗位链接生成 JD 快照
2. 在看板中拖动卡片，随投递进展切换阶段
3. 收到面试后添加面试轮次，面试结束后上传录音、写复盘笔记
4. 在简历版本和经历库中维护材料，投递时按需复制使用

界面提供中文 / English 切换按钮，也支持切换主题。

## 数据位置

数据目录按以下优先级确定：

1. 环境变量 `OFFERFLOW_DATA_DIR` 指定的目录
2. 程序旁**已存在**的 `data/` 文件夹
3. `%APPDATA%\offerflow`

- **安装版**：数据默认保存在 `%APPDATA%\offerflow`
- **免安装版**：首次启动前在 exe 旁新建空的 `data/` 文件夹，数据就会保存在程序目录中，便于整体拷贝带走

卸载程序不会删除 `%APPDATA%\offerflow` 中的求职数据；如需彻底清除，请手动删除该文件夹。更换数据目录时，可通过完整备份与恢复迁移数据。

## 数据与隐私

- 求职记录、简历、录音和网页快照保存在本地，支持备份与恢复，以及同步到指定文件夹
- AI 提取可使用本地 Ollama 或配置的模型接口；发送到外部服务前会显示目标地址并确认
- Cookie 与 API Key 使用 Windows DPAPI 加密保存，不包含在导出的备份中
- 恢复前校验备份数据库完整性，原数据库保留为 `.bak`

## 开发

需要 Node.js 22.18+、Rust stable，以及 Tauri 2 的系统依赖（Windows 需 VS Build Tools）。

```bash
npm ci
npm run tauri dev      # 开发，支持热更新
npm run tauri build    # 构建 exe 与安装包
```

构建产物位于 `src-tauri/target/release/`：

| 文件 | 说明 |
|---|---|
| `offerflow.exe` | 本地构建的可执行文件，发布时命名为 `OfferFlow_1.0.2_x64.exe` |
| `bundle/nsis/*-setup.exe` | NSIS 安装包 |
| `bundle/msi/*.msi` | MSI 安装包 |

本地验证（与 CI 一致）：

```bash
npm run check
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

## 反馈

遇到问题或有功能建议，欢迎在 [GitHub Issues](https://github.com/iTakeRu/OfferFlow/issues) 中提出。

## 作者与开发工具

由 [@iTakeRu](https://github.com/iTakeRu) 开发，使用 Codex 与 Claude Code 交替辅助实现、代码审查与交叉验证。

## License

[MIT](./LICENSE)
