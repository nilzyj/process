# Process

一款基于 Tauri v2 的媒体记录管理桌面应用。追踪电影、动漫、电视剧、书籍、纪录片、播客等观看/阅读进度。

## 功能

- **媒体追踪** — 记录名称、类型、状态、进度、标签等
- **搜索筛选** — 按关键词、类型、状态快速筛选
- **统计看板** — 类型分布、状态分布、完成率
- **数据持久化** — 连接远程 MySQL 数据库存储

## 技术栈

| 层 | 技术 |
|---|---|
| 桌面壳 | Tauri v2 |
| 前端 | React + TypeScript + Vite |
| 后端 | Rust (sqlx) |
| 数据库 | MySQL |

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
