# 飞书同步

## 1. 安装 CLI

安装 [Node.js LTS](https://nodejs.org/)，打开 PowerShell，执行：

```powershell
npx @larksuite/cli@latest install
lark-cli --version
```

## 2. 配置与授权

```powershell
lark-cli config init
```

按向导创建飞书应用，或填入已有应用的 App ID 和 App Secret。然后授权个人账号访问多维表格：

```powershell
lark-cli auth login --domain base
lark-cli auth status --json --verify
```

在浏览器完成授权。状态中的 `user` 显示 `ready`、`verified: true` 后，重新打开 OfferFlow。

## 3. 连接投递表

进入 **系统设置 → 飞书同步**，选择一种方式：

- **新建投递表**：填写名称，点击「创建并连接」。表格会出现在飞书云盘中。
- **绑定已有表格**：粘贴多维表格链接，点击「读取数据表」，选择数据表后点击「绑定表格」。OfferFlow 会补齐投递字段，保留已有内容；同名字段类型不匹配时会显示对应字段。

点击「立即同步到飞书」上传当前投递。「打开飞书表格」可直接查看结果。

## 4. 自动同步

开启「自动同步到飞书」后，OfferFlow 在运行期间每 15 秒检查本地变化；失败后间隔 5 分钟重试。开关即时保存。

同步方向为 **OfferFlow → 飞书**。本地值覆盖对应字段；本地删除后，飞书行保留。附件、简历和录音保存在本地。

## 常见问题

| 情况 | 处理 |
|---|---|
| 找不到 `lark-cli` | 重新打开 PowerShell 和 OfferFlow。仍找不到时，在「CLI 设置」填写 `lark-cli.exe` 的完整路径。 |
| npm 安装后需要找原生程序 | 在 PowerShell 运行 `npm root -g`，进入返回目录下的 `@larksuite\cli\bin`，选择 `lark-cli.exe`。 |
| 登录过期 | 重新执行 `lark-cli auth login --domain base`。 |
| 提示缺少权限 | 在飞书应用后台开通错误中列出的权限并发布版本，再执行授权命令。 |
| 已授权但无法绑定 | 确认授权账号有目标表格的编辑权限。 |
| 建表中断 | 先到飞书云盘查找。已有表格直接绑定；没有则点击「重新创建投递表」。 |

「Bridge 配置名称」通常留空。已使用 Lark Channel Bridge 的用户，填入已授权的 profile 名称。

安装与授权命令参考[飞书 CLI 官方文档](https://github.com/larksuite/cli#installation--quick-start)。
