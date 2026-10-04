# Process

一款基于 Tauri v2 的媒体记录管理桌面应用。追踪电影、动漫、电视剧、书籍、纪录片、播客等观看/阅读进度。

## 功能

- **媒体追踪** — 记录名称、类型、状态、进度、标签等
- **搜索筛选** — 按关键词、类型、状态快速筛选
- **统计看板** — 类型分布、状态分布、完成率
- **本地媒体库** — 扫描本地目录的视频文件，点击直接用 PotPlayer 播放
- **快照备份** — 导出带校验的 JSON 快照到坚果云同步文件夹，可随时恢复

## 技术栈

| 层 | 技术 |
|---|---|
| 桌面壳 | Tauri v2 |
| 前端 | React + TypeScript + Vite |
| 后端 | Rust (sqlx) |
| 数据库 | SQLite（本地单文件，WAL 模式） |
| 备份 | JSON 快照 → 坚果云（WebDAV 目录或客户端同步目录均可） |

## 存储

数据存放在本地 SQLite 单文件 `~/.process-app/data.sqlite`，无需运行任何数据库服务。

**为什么用快照而不是 WebDAV API 直连**：坚果云免费版 WebDAV 限制为每 30 分钟 600 次请求、单次请求最多 750 个文件，且第三方客户端若未实现分页会静默丢数据。几千条记录做一次全量同步就会撞上频率限制。改由坚果云客户端同步快照文件，应用侧零网络代码，也顺带避免了对象存储无事务导致的并发覆盖。

数据库**刻意不放进坚果云同步目录**：SQLite 是 `.db` + `-wal` + `-shm` 三个文件，WAL 频繁变动会导致同步出损坏状态。备份请走快照。

### MySQL → SQLite 迁移

原实现使用远程 MySQL（`process` 表）。迁移只改方言，19 条聚合查询的逻辑与排序语义逐条保留。方言层面的三处关键处理：

| 差异 | 处理 |
|---|---|
| `NOW()`/`CURDATE()` 走 MySQL 会话时区，SQLite 内置函数是 UTC | 日期边界在 Rust 侧按本地时区算好后 bind 进 SQL，消除时区歧义 |
| `utf8mb4_0900_ai_ci` 不区分大小写 | `LIKE` / `=` / `GROUP BY` 统一加 `COLLATE NOCASE` |
| `CAST(year AS UNSIGNED)` 在 SQLite 不存在 | 改为 `CAST(... AS INTEGER)`，字段类型 `u64` → `i64` |

`COLLATE NOCASE` 是必需的：SQLite 的 `LIKE` 对 ASCII 也不敏感，但对非 ASCII 严格，不加会让中文与英文标题的匹配行为与 MySQL 不一致。

迁移路径为「导出 JSON → 导入」，与快照恢复共用同一套导入代码，运行时不再携带 MySQL 依赖。导入兼容快照格式与纯记录数组两种形态，并接受 `type` / `media_type` 两种键名。

### 快照

「设置」页可配置快照目录（建议指向坚果云同步文件夹）与保留份数。快照格式：

```json
{ "version": 1, "exported_at": "...", "checksum": "...", "records": [...] }
```

- 先写临时文件再 `rename`，同步中断时目标文件要么是旧内容要么是新内容，不会是半截
- 内置 FNV-1a 校验和，恢复前验证，损坏文件会被拒绝而非写入半份数据
- 文件名内嵌时间戳，同一秒内重复导出会追加序号，不会互相覆盖
- 超出保留份数时删除最旧的，`process_latest.json` 始终保留最新一份
- 恢复与导入操作会先自动导出当前数据，可回退

### 配置文件

| 文件 | 内容 |
|---|---|
| `~/.process-app/data.sqlite` | 记录数据（WAL 模式，含 `-wal`/`-shm`） |
| `~/.process-app/config.json` | 数据目录、快照目录、保留份数 |
| `~/.process-app/library.json` | 媒体库目录列表、PotPlayer 路径 |

## 本地媒体库

「媒体库」页扫描本地目录中的视频文件，并调用 PotPlayer 播放。该功能与记录数据完全独立。

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
4. 遍历 A–Z 固定盘的常见安装目录（覆盖装在非 C 盘的情况）

## 开发

```bash
npm install
npm run tauri:dev
```

### 测试

```bash
cd src-tauri
cargo test --lib
```

31 个测试，覆盖 CRUD 往返、大小写不敏感搜索、日期边界（含闰年与跨月）、进度分桶、标签拆分、id 序列延续、快照校验与裁剪。

以下诊断测试默认跳过，需外部环境变量：

```bash
# 抓取 get_stats() 基线，用于方言等价性比对
MYSQL_HOST=... MYSQL_PORT=... MYSQL_USER=... MYSQL_PW=... MYSQL_DB=... \
  STATS_OUT=stats-mysql.json \
  cargo test --lib -- --ignored dump_stats_baseline --nocapture

# 导入真实数据后比对
RECORDS_JSON=records.json STATS_OUT=stats-sqlite.json \
  cargo test --lib -- --ignored sqlite_baseline --nocapture

# 导入到真实用户数据库 / 导出真实快照 / 定位本机 PotPlayer
RECORDS_JSON=records.json cargo test --lib -- --ignored import_into_default_storage --nocapture
RECORDS_JSON=1 cargo test --lib -- --ignored export_real_snapshot --nocapture
cargo test --lib -- --ignored report_located_potplayer --nocapture
```

## 构建

```bash
npm run tauri:build
```

构建产物位于 `src-tauri/target/release/bundle/`。