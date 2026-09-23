# OfferFlow

本地运行的求职投递管理工具：看板管理投递流程，JD 网页快照存档，面试录音与复盘，简历版本管理。数据全部保存在本机。

## 功能

- **投递**：看板 / 列表两种排法，卡片拖拽换阶段；阶段可自定义；按公司、岗位、城市、分类、渠道搜索筛选
- **JD 快照**：粘贴岗位链接、导入另存的网页（.html / .mhtml）或粘贴文本，原样存档，岗位下架后仍可回看
- **岗位识别**：页面带 schema.org JobPosting 数据时自动识别；可选接入本地 Ollama 或 OpenAI 兼容接口做 AI 提取（发送前会确认）
- **面试复盘**：记录每轮面试，支持录音、音频标注与富文本复盘笔记
- **投递统计**：阶段分布、分类分布、近 14 天面试安排
- **简历版本**：导入、预览、重命名、打标签
- **数据保护**：删除的文件先进回收站；可选把数据增量同步到另一块硬盘或网盘文件夹；数据库升级前自动留副本
- **导出与备份**：导出 Excel；完整备份 / 恢复

## 下载与使用

构建产物位于 `src-tauri/target/release/`：

| 文件 | 说明 |
|---|---|
| `offerflow.exe` | 免安装单文件，双击即用 |
| `bundle/nsis/*-setup.exe` | 安装版，装给当前用户，无需管理员权限 |
| `bundle/msi/*.msi` | 安装版（MSI），装给所有用户 |

需要 WebView2 运行时（Windows 10 / 11 已内置）。

**数据位置**（按优先级）：环境变量 `OFFERFLOW_DATA_DIR` → exe 旁已存在的 `data/` 文件夹 → `%APPDATA%\offerflow`。想把数据和 exe 一起带走，在 exe 旁新建空的 `data/` 即可。

## 开发

需要 Node.js 20+、Rust stable，以及 Tauri 2 的系统依赖（Windows 需 VS Build Tools）。

    npm install
    npm run tauri dev      # 开发，支持热更新
    npm run tauri build    # 构建 exe 与安装包

提交前运行（CI 会执行同样的检查）：

    npm run check
    cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
    cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
    cargo test --manifest-path src-tauri/Cargo.toml

## 数据与隐私

- 所有数据只存本机；Cookie 与 API Key 用 Windows DPAPI 加密，不进入备份
- 恢复备份前会校验数据库完整性，原数据保留为 `.bak`
- 页面抓取只做普通访问（HTTP 或本机 Edge / Chrome 无头模式），不调用站点私有接口、不绕过访问控制；需要登录的页面请在浏览器另存后导入。请遵守目标站点条款，快照仅供个人使用
- 数据目录含个人求职隐私，请勿提交到公开仓库

## 贡献者

- [@iTakeRu](https://github.com/iTakeRu)：项目作者
- [Claude Code](https://claude.com/claude-code)（Anthropic）：AI 结对编程，参与界面改版、数据保护与备份等功能的开发

## License

[MIT](./LICENSE)
