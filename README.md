# Process

一款基于 Tauri v2 的媒体记录管理桌面应用。追踪电影、动漫、电视剧、书籍、纪录片、播客等观看/阅读进度。

## 功能

- **媒体追踪** — 记录名称、类型、状态、进度、标签等
- **搜索筛选** — 按关键词、类型、状态快速筛选
- **统计看板** — 类型分布、状态分布、完成率
- **本地媒体库** — 扫描本地目录的视频文件，点击直接用 PotPlayer 播放
- **数据持久化** — 连接远程 MySQL 数据库存储

## 技术栈

| 层 | 技术 |
|---|---|
| 桌面壳 | Tauri v2 |
| 前端 | React + TypeScript + Vite |
| 后端 | Rust (sqlx) |
| 数据库 | MySQL |

## 本地媒体库

「媒体库」页扫描本地目录中的视频文件，并调用 PotPlayer 播放。该功能**完全独立于 MySQL**，未配置数据库也可使用。

添加库目录有三种方式，功能等价：

- **浏览…** — 原生目录选择对话框，支持多选
- **手动输入** — 粘贴目录绝对路径
- **拖放** — 直接把文件夹拖进窗口

识别范围为扩展名 `mkv mp4 avi mov wmv flv ts m2ts mpg mpeg vob webm rm rmvb`。扫描会跳过隐藏目录与 NAS 垃圾目录（`@eaDir` 等）；单个文件读取失败只计数跳过，不中断整体扫描。不存在的目录会以「不可访问」标记，仍保留在列表中以便移除。

点击卡片即用 PotPlayer 打开；右键菜单可打开所在文件夹或复制完整路径。

### PotPlayer 定位

按以下顺序查找，失败时可在「设置播放器」手动指定路径：

1. 手动指定的路径（`~/.process-app/library.json`）
2. 注册表 `HKCU/HKLM\SOFTWARE\Classes\potplay\shell\open\command`
3. 注册表 `App Paths\PotPlayerMini64.exe`
4. 遍历 A–Z 固定盘的常见安装目录

### 配置文件

| 文件 | 内容 |
|---|---|
| `~/.process-app/config.json` | MySQL 连接信息 |
| `~/.process-app/library.json` | 库目录列表、PotPlayer 路径 |

媒体库配置独立于数据库配置：前端的「断开」按钮会以空值覆写 `config.json`，若合并存储会连带清空已添加的目录。

## 开发

```bash
npm install
npm run tauri:dev
```

## 构建

```bash
npm run tauri:build
```

构建产物位于 `src-tauri/target/release/bundle/`。
