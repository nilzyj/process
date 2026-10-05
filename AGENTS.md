# AGENTS.md

Tauri v2 桌面应用（媒体记录管理）。前端 React 19 + TypeScript + Vite，后端 Rust + sqlx/SQLite。
**用户可见文案与代码注释一律用中文**；commit message 用 Conventional Commits + 中文描述（`feat:` / `fix:` / `style:` / `refactor:`）。

## 命令

```bash
npm install
npm run dev            # 仅前端 vite dev server
npm run tauri:dev      # 完整桌面应用（含 Rust 重编译，首次很慢）
npm run build          # tsc -b && vite build（类型检查 + 打包）
npm run lint           # oxlint；修复用 npx oxlint --fix
npm run preview
npm run tauri:build    # 产物在 src-tauri/target/release/bundle/
```

Rust 侧（在 `src-tauri/` 下执行）：

```bash
cargo test --lib                        # 全部单测（50 个，分布在 6 个模块内）
cargo test --lib db::tests               # 只跑 db 模块
cargo test --lib crud_roundtrip          # 按名字跑单个测试
cargo test --lib -- --nocapture          # 打印 println! 输出
cargo test --lib -- --ignored dump_stats # 跑默认跳过的诊断测试
cargo clippy --lib -- -D warnings        # lint
cargo fmt                               # 仅 build.rs / main.rs 未格式化，其余已符合 rustfmt
```

前端没有测试框架，不要臆造 `npm test`。**改 Rust 代码后必须 `cargo test --lib`**；
改 `src/types.ts` 或任何 `invoke` 调用后必须 `npm run build`（`tsc -b` 是唯一的类型检查手段）。

依赖诊断测试默认 `#[ignore]`，需要环境变量，doc 注释里写了完整命令行，参考
`src-tauri/src/db.rs:1054`（`sqlite_baseline`）、`src-tauri/src/commands.rs:429`。

## 架构

```
src/                     React 前端
  types.ts               所有跨端类型 + 展示用常量（MEDIA_TYPES / STATUS_STYLES / TYPE_LIST / STATUS_LIST）
  App.tsx                顶层布局 + 页面切换 + init_storage 引导
  pages/                 Home / Stats / Library / Settings（Stats、Library、Settings 用 lazy 加载）
  components/            可复用 UI
  App.css                全局唯一样式表（CSS 变量 + 语义化 class）
src-tauri/src/
  lib.rs                 Builder、插件注册、manage(AppState)、generate_handler! 命令表
  commands.rs            所有 #[tauri::command]（前后端唯一边界）
  models.rs              serde 数据结构，纯类型无逻辑
  db.rs                  SQLite 连接、schema、CRUD、19 条聚合统计查询
  snapshot.rs            快照导出/恢复/导入 + FNV-1a 校验
  config.rs              ~/.process-app/config.json 与 library.json 读写
  scan.rs / player.rs    本地视频扫描、PotPlayer 定位与拉起
```

数据存 `~/.process-app/data.sqlite`（WAL），**刻意不放进坚果云同步目录**。

## 新增 Tauri 命令清单

1. `models.rs` 增删 serde 结构（`#[derive(Debug, Serialize, Deserialize, Clone)]` + 逐字段 `pub`）
2. `commands.rs` 写 `#[tauri::command] async fn`，签名统一 `Result<T, String>`
3. `lib.rs` 的 `tauri::generate_handler![...]` 里注册（**漏注册前端调用会静默失败**）
4. `src/types.ts` 加同名镜像 interface（snake_case 字段名必须与 serde 一致）
5. 用到新插件时在 `src-tauri/capabilities/default.json` 补 permission

## Rust 风格

- edition 2021，4 空格缩进，`snake_case` 函数 / 模块，`PascalCase` 类型，`SCREAMING_SNAKE` 常量
- 内部函数返回 `anyhow::Result<T>` 并用 `.context("中文说明")` / `anyhow::bail!("...")`；
  只有 `#[tauri::command]` 边界转成 `Result<T, String>`（`format!("导出失败: {}", e)`）
- 分区用 `// ---------- 存储 ----------` 这类横线注释（见 `commands.rs`）
- 模块私有函数不加 `pub`；类型放 `models.rs`，不要在 `commands.rs` 里堆 DTO（`StorageInfo` 例外）
- 平台相关代码用 `#[cfg(windows)]`，并在文件顶部集中放常量（见 `player.rs`）
- 注释解释**为什么**这么写（方言差异、踩过的坑、不可替代的取舍），不要复述代码在做什么

## SQL / SQLite 硬性规则

- 绝不用 `format!` 拼接用户输入；条件值一律 `?` 绑定。动态 WHERE 子句是既有模式：
  收集 `Vec<String>` 再 `join(" AND ")`，bind 顺序与条件 push 顺序严格一致（`db.rs:109`）
- **所有 `LIKE` / `=` 比较必须显式 `COLLATE NOCASE`**，用来对齐 MySQL `utf8mb4_0900_ai_ci` 语义
- 日期时间**在 Rust 侧用 `TS_FORMAT`（`%Y-%m-%d %H:%M:%S`）算好后 bind 进 SQL**，
  不要用 SQLite 的 `datetime('now')`（UTC，会偏 8 小时）。统计窗口用 `DateWindow`
- `type` / `year` 是 SQL 保留字，必须反引号；`CAST(\`year\` AS INTEGER)` 显式转型
- SQL 通过 `sqlx::query` / `query_as` 动态执行，不使用编译期校验宏（无 `DATABASE_URL`）

## Rust 测试风格

`#[cfg(test)] mod tests` 放在**所属模块文件末尾**（`db.rs` 的 `Default for RecordFilter` 之后没有测试，
注意 `db.rs` 里 `mod tests` 在 `impl Default` 之前）。约定：

- 涉及 DB/文件系统的测试用 `temp_pool("tag")` / `temp_dir()`，结尾 `drop(pool)` 后
  `let _ = std::fs::remove_dir_all(&dir);`
- 异步用 `#[tokio::test]`，纯函数用 `#[test]`
- 测试名描述行为而非实现：`search_is_case_insensitive_like_mysql`
- `assert!` 必须带中文消息说明期望与实际：`assert_eq!(r.total, 1, "搜索 {needle} 应命中 1 条")`
- 需要外部环境的测试加 `#[ignore = "原因"]`，并在 doc 注释写出手动运行命令

## TypeScript 风格

- 2 空格缩进、单引号、分号、尾随逗号；无 Prettier 配置，手工保持一致
- `verbatimModuleSyntax: true` → **纯类型导入必须写 `import type { X } from '...'`**，
  否则 `tsc -b` 报错
- `erasableSyntaxOnly` → 不用 enum / namespace / 构造器参数属性，用 union type 和普通函数
- `noUnusedLocals` / `noUnusedParameters` → 不用 `_` 前缀也没关系，但未用参数必须删掉
- `noFallthroughCasesInSwitch`；`jsx: "react-jsx"` → **不需要 `import React`**
- 组件 `export default function X()`，props 定义 `interface Props { ... }` 放在组件上方。
  例外：需要 `memo` 的列表行写成 `function X() {...}` + `export default memo(X)`（见下）
- **列表行组件必须 `memo`，且父组件传入的回调全部 `useCallback`**。
  `records` 一次渲染 200 行，任一行变动都会 `setRecords`；不 `memo` 则整表重渲染，
  行内动画要等 200 次 reconcile 完才播，表现为卡顿。
  回调每次渲染新建引用会让 `memo` 完全失效——这是最容易踩空的地方
- **固定宽度元素加 `flex-shrink: 0`**：热力图 53 周约 900px，祖先链任一层 flex
  都会把它压扁并裁掉右侧。配 `overflow-x: auto` 兜住窄窗口
- 后端字段一律 snake_case，与 Rust 对齐；不要在 TS 侧改名
- 保持 hook 依赖数组完整（oxlint `react/rules-of-hooks` 是 error）。
  数据加载写 `useCallback(async (silent?: boolean) => {...}, [deps])` 再在 `useEffect` 里调
- **输入即查询的场景要防抖**：`search` 直接进 `useCallback` 依赖会让每敲一个字
  发一次请求。拆成 `search`（即时回显）与 `query`（实际查询），`setTimeout` 250ms 后同步，
  `cleanup` 里 `clearTimeout`。顺带避开中文输入法拼字期
- **查询期间不要清空列表**：渲染分支用 `loading && records.length === 0` 才显示 spinner，
  否则每次查询都把已有结果顶掉，看着就是闪烁
- 前端错误处理：`const [error, setError] = useState('')`，catch 里 `setError(String(e))` 并渲染出来；
  后台刷新等非关键路径用 `.catch(() => {})` 或 `catch {}` 静默
- 调用后端：`await invoke<T>('command_name', { argName })`，命令名与 Rust fn 同名；
  泛型参数是唯一的类型保障，别省

## CSS 风格

- 所有样式进 `src/App.css`（无 CSS-in-JS、无 CSS Module）。新样式追加到文件对应区块末尾
- **字号只用 `--fs-*` 九档变量**，不写 px 字面量。改字号只改 `App.css` 的那一段，
  全站联动。列表/徽标等紧凑控件还要同步核对：固定 `height` 是否小于字号×行高、
  固定 `width` 是否容得下最长文案（中文按 1.0em、ASCII 按 0.55em 估算）
- 颜色 / 圆角 / 阴影一律用 `:root` 里的 CSS 变量（`--bg-card`、`--accent`、`--danger` …），
  不要写死十六进制，除非是 `MEDIA_TYPES` 这类数据驱动的品牌色
- **背景不用纯黑**：纯黑会让浅色文字产生光晕，且卡片与底色只差几个色阶、边界糊在一起。
  用 `--bg-primary` 抬底 + 逐级 +6 明度的阶梯（primary → secondary → card → elevated），
  边界靠 `--border` 分隔
- **文字对比度按 WCAG AA 定标**，别只靠肉眼。`--text-muted` 用在 10px 角标上时
  比 AA 下限多留一档余量
- **不使用 `transition: all`**：会让浏览器监听包括 layout 尺寸类在内的所有可动画属性。
  显式列出实际会变的属性
- **不给列表行内的元素加 `will-change`**：每个实例都会被永久提升为合成层，
  上百行就是上百层常驻 GPU 内存。一次性动画直接写 `animation` 即可
- **不加常驻无限动画**：`.progress-fill::after` 曾是每行一个 `2s infinite`，
  合成线程永远无法空闲，滚动和点击都要抢帧。改成 `:hover` 时才播一次
- class 用 kebab-case 语义命名：`.form-group`、`.btn.btn-primary`、`.tab-btn.active`、
  `.modal-overlay`；修饰符用 `.active` / `.danger` 这类单类
- 只有数据驱动的值（颜色、宽度百分比、坐标）才用 inline `style={{}}`。
  **静态排版一律提进 class**——`.cell-center` / `.cell-md` / `.cell-empty` / `.record-tags`
  就是为此抽出来的，逐处写 inline 会导致改字号时漏改
- 加完 class 记得回收：TSX 里 0 引用且与实际选择器不符的规则要删。
  用 `closest('.foo')` 做事件委托时，选择器必须与真实渲染的 class 一致，否则判断恒为真

## 提交

改完跑 `npm run lint && npm run build` 与 `cd src-tauri && cargo test --lib`。
写 snapshot / 导入相关改动时额外验证 checksum 校验失败路径与 `keep_snapshots` 裁剪测试。
