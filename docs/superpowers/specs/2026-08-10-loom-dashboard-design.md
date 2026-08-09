# Loom Dashboard 可视化方案设计

> **日期**: 2026-08-10
> **分支**: `feature/loom-dashboard`
> **状态**: 设计已确认，待实现

## 1. 背景与动机

Loom 在运行过程中产生大量结构化数据：每次 delivery 包含 brainstorm 契约、技术基线、架构产物、任务计划与执行结果、review 发现、部署状态、知识库构建记录等。这些数据以 JSON 文件形式存储在项目本地 `.loom/` 和用户全局 `~/.loom/` 目录下。

当前 Loom **没有任何交互式呈现层**。所有"人读呈现"完全外包给 agent 聊天回复，Loom 自身只产出一行 `summary` 文本 + 面向 agent 的指令文本 + 两个调试日志文件（`loom-mcp.log` 与 `loom-mcp-trace.log`）。存在以下 9 个呈现缺口：

| 编号 | 缺口 | 说明 |
|------|------|------|
| A | 交付全貌/进度时间线 | `loom.status` 只返回当前单点快照，无阶段历史与时间线 |
| B | 部署状态人读视图 | `deployStatus` 返回裸 JSON，无服务健康表/预览 URL 列表 |
| C | 日志人类可读呈现 | `deployLogs` 返回原始行数组，无级别着色/错误筛选 |
| D | 知识搜索结果预览 | 搜索结果卡片无正文片段，需二次调用 `inspectChunk` |
| E | 产物聚合报告 | brainstorm/baseline/architecture/task/review 产物散落各处 JSON 文件，无聚合视图 |
| F | 修复历史与 review 信号 | 多轮修复历史、review findings 矩阵无汇总视图 |
| G | 运行时实时进度 | 部署运行期间无 push 式进度，需周期性轮询 |
| H | 跨 delivery 总览 | 无工具列出本项目所有历史 delivery 的人读视图 |
| I | 调试日志与用户呈现界限 | 唯一的"人类可读"持久产物是面向开发者的调试日志 |

本方案设计一个**独立 Web Dashboard**，解决上述全部缺口。

## 2. 架构决策（已确认）

通过四轮澄清问题，以下架构方向已确认：

| 决策维度 | 选定方案 | 理由 |
|---------|---------|------|
| 交付形态 | 独立 Web Dashboard | 独立于 agent，可并行查看，实时与事后回顾均支持 |
| 使用场景 | 实时跟进 + 事后回顾 | 既 live 跟进运行中的 delivery，又可浏览历史 delivery |
| 技术栈 | Rust 后端 + React 前端 | 后端与 Loom Rust 代码库一致，前端交互体验丰富 |
| 数据源 | 直接读 JSON 文件 + 复用 Serde 模型 | 零侵入 Loom 工作流，notify crate 监听文件变化做 SSE |
| 启动方式 | setup CLI 新增 dashboard 子命令 | 复用现有安装流程，不增加发布物 |

## 3. 整体架构与模块边界

### 3.1 新增 crate：`dashboard`

在 `src/rust/` 工作区新增 `dashboard` crate，与 `mcp-server`、`setup`、`deploy` 等领域 crate 平级：

```
src/rust/
├── dashboard/              ← 新增
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs          # 对外暴露 serve() 函数
│   │   ├── server.rs       # axum 路由 + HTTP 服务
│   │   ├── sse.rs          # SSE 事件流（文件变更推送）
│   │   ├── watcher.rs      # notify crate 文件监听
│   │   ├── embedded.rs     # rust-embed 静态资源嵌入
│   │   ├── routes/         # HTTP API 路由模块
│   │   │   ├── mod.rs
│   │   │   ├── project.rs      # GET /api/project/status
│   │   │   ├── deliveries.rs   # GET /api/deliveries, /api/deliveries/:id
│   │   │   ├── phases.rs       # GET /api/deliveries/:id/phases
│   │   │   ├── tasks.rs        # GET /api/deliveries/:id/phases/:pid/tasks
│   │   │   ├── reviews.rs      # GET /api/deliveries/:id/phases/:pid/reviews
│   │   │   ├── deploy.rs       # GET /api/deploy/status, /logs, /inspect
│   │   │   ├── knowledge.rs    # GET /api/knowledge/sources, /search
│   │   │   └── events.rs       # GET /api/events (SSE)
│   │   └── reader/         # 数据读取层（复用 contracts/state Serde 模型）
│   │       ├── mod.rs
│   │       ├── project_reader.rs
│   │       ├── delivery_reader.rs
│   │       ├── task_reader.rs
│   │       ├── review_reader.rs
│   │       ├── deploy_reader.rs
│   │       ├── knowledge_reader.rs
│   │       └── audit_reader.rs
│   ├── embedded/           # rust-embed 目标目录
│   │   └── dist/           # Vite 构建产物（编译期嵌入）
│   └── frontend/           # React 前端源码（Vite 构建）
│       ├── package.json
│       ├── vite.config.ts
│       ├── tsconfig.json
│       ├── tailwind.config.ts
│       ├── index.html
│       └── src/
│           ├── main.tsx
│           ├── App.tsx
│           ├── api/
│           ├── hooks/
│           ├── components/
│           └── pages/
└── ...
```

### 3.2 模块职责与依赖

```
┌─────────────────────────────────────────────────┐
│                 setup CLI                        │
│  (src/rust/setup/main.rs)                        │
│  新增 dashboard 子命令 → 调用 dashboard::serve() │
└──────────────────┬──────────────────────────────┘
                   │ depends on
                   ▼
┌─────────────────────────────────────────────────┐
│              dashboard crate                     │
│                                                  │
│  ┌──────────┐  ┌──────────┐  ┌───────────────┐  │
│  │  server   │  │  watcher  │  │    reader     │  │
│  │ (axum)    │  │ (notify)  │  │ (反序列化)     │  │
│  │ HTTP+SSE  │  │ 文件监听   │  │ .loom/*.json  │  │
│  └────┬─────┘  └─────┬────┘  └───────┬───────┘  │
│       │              │                │          │
│       │     文件变更事件 → SSE 推送     │          │
│       │              │                │          │
└───────┼──────────────┼────────────────┼──────────┘
        │              │                │
        │ depends on   │                │ depends on
        ▼              ▼                ▼
┌──────────────┐  ┌─────────┐  ┌──────────────────┐
│   contracts   │  │  state  │  │  core / deploy / │
│  (Serde 模型)  │  │ (paths) │  │  knowledge crate │
└──────────────┘  └─────────┘  └──────────────────┘
```

### 3.3 关键设计决策

1. **零侵入 Loom 工作流**：`dashboard` crate 只**读** `.loom/` 和 `~/.loom/` 下的 JSON 文件，不调用任何 MCP 工具，不修改任何状态文件。即使 Dashboard 崩溃也不影响正在运行的 delivery。

2. **复用 Serde 模型**：`reader/` 层直接 `use contracts::execution::{TaskPlanRun, TaskResult};` 等做反序列化，不重新定义模型。数据结构变化时自动跟随。

3. **复用 paths 模块**：`state::paths`、`deploy::paths`、`knowledge::paths` 已定义了所有文件路径常量，reader 层直接调用这些函数获取路径。

4. **前端嵌入方式**：Vite 构建产物 `dist/` 在编译期通过 `rust-embed` 嵌入二进制，最终 `loom dashboard` 单二进制启动，无需额外文件。

### 3.4 setup CLI 集成

在 `src/rust/setup/main.rs` 新增 `Dashboard` 子命令：

```
loom dashboard [--port 9876] [--no-open] [--project <path>]
```

- `--port`：默认 9876，端口冲突时自动递增（9876→9885）
- `--no-open`：不自动打开浏览器
- `--project`：指定项目路径（默认当前目录）
- 启动后阻塞运行 axum 服务，Ctrl-C 优雅退出

## 4. 视图组成与信息架构

### 4.1 全局布局

```
┌──────────────────────────────────────────────────────────┐
│ Loom Dashboard                    [项目: my-app ▾]  [刷新] │  ← 顶栏：项目切换 + 实时状态点
├────────┬─────────────────────────────────────────────────┤
│        │                                                 │
│ 总览    │                                                 │
│ 交付列表│            主内容区（按路由切换）                │
│ 知识库  │                                                 │
│ 部署    │                                                 │
│ 日志    │                                                 │
│        │                                                 │
├────────┴─────────────────────────────────────────────────┤
│ ● 实时  | delivery: del_abc | phase: ph_01 | task 3/7     │  ← 底栏：实时状态条（SSE 驱动）
└──────────────────────────────────────────────────────────┘
```

- **左侧导航**：固定 6 个一级页面
- **底栏状态条**：SSE 推送驱动，显示当前活跃 delivery/phase/task 进度计数，绿/黄/红状态点
- **顶栏项目切换**：读取 `~/.loom/projects/index.json` 列出所有注册项目

### 4.2 六个视图页面

#### 页面 1：总览（Overview）

**对应缺口**：A（交付全貌时间线）、H（跨 delivery 总览）

展示当前活跃交付卡片（含阶段、任务进度百分比、review findings 计数）+ 历史交付列表表格（deliveryId、标题、状态、phase 数、最后更新时间）。

- 数据源：`.loom/status.json`（ProjectStatus）+ 遍历 `.loom/deliveries/*/index.json`
- 进度百分比 = `TaskPlanRunSummary.completed / total`
- 实时刷新：监听 `status.json` 与 `deliveries/*/index.json` 变化

#### 页面 2：交付详情（Delivery Detail）

**对应缺口**：A（阶段时间线）、E（产物聚合）、F（修复历史）

这是最核心的页面，一次 delivery 的完整视图，包含：

- **阶段时间线**：从 `DeliveryIndex.phases[]` 渲染水平时间线，每个阶段标注状态（✓ 完成 / ▶ 进行中 / ⏳ 待办），点击可跳转到该阶段产物
- **任务列表**：`TaskPlanRun.taskStates` 聚合，展示 task 编号、标题、状态、耗时、结果状态、attempt 次数，点击"查看"展开 TaskResult 详情（changedFiles、verificationResults、evidence）
- **Review findings**：最新 `ReviewResult.findings[]`，按 severity 排序，展示严重级别、类别、摘要、关联 task，点击展开 evidence 与 readRefs
- **顶部 tab**：切换到 brainstorm/baseline/architecture/taskplan 的原始产物查看（只读结构化渲染，非裸 JSON）

数据源：`DeliveryIndex` + 各阶段 `latestRefs` 指向的 JSON 文件 + `TaskPlanRun` + 最新 `ReviewResult`。

#### 页面 3：知识库（Knowledge）

**对应缺口**：D（知识搜索结果无预览）

- 左侧来源列表：从 `~/.loom/knowledge/registry.json` 读取所有 KnowledgeSource，展示名称、chunk 数、构建状态（已发布/构建中/待构建）
- 右侧搜索结果：Dashboard 后端复用 `knowledge` crate 的 BM25 词项索引（读取 `lexical-index.json`），在进程内执行查询。不调用 MCP `knowledgeSearch` 工具，避免状态机耦合。
- **关键改进**：搜索结果直接内联 chunk 正文片段（从 `chunks/*.txt` 读取），展示 score、token 估算、headingPath，无需二次调用 `inspectChunk`
- 构建状态实时更新：监听 `semantic-state.json` 变化

#### 页面 4：部署（Deploy）

**对应缺口**：B（部署健康表）、C（日志着色）

- **服务拓扑图**：从 `DeploymentTopology.routes` 渲染节点与边，每个服务标注端口与健康状态
- **验证路由表**：previewPaths 与 apiProbes 的 HTTP 状态码、响应耗时
- **实时日志**：tail `logs/local.log`，行级解析 `[LEVEL]` 做着色（ERROR 红色、WARN 黄色、INFO 默认），SSE 推送新增行，环形缓冲区最多 1000 行
- **修复历史**：读取 `state/latest-failure.json` + `repairs/` 目录，展示失败类型、修复状态、尝试次数

#### 页面 5：日志（Logs）

**对应缺口**：I（调试日志与用户呈现界限）

- 统一日志查看器，类型切换：MCP trace / 运行时日志 / 部署日志
- 级别过滤（DEBUG/INFO/WARN/ERROR）、模块过滤、时间范围筛选
- trace 日志按 `>>`/`<<` 标记区分请求/响应方向

数据源：`~/.loom/log/loom-mcp.log` + `loom-mcp-trace.log` + `.loom/deployment/logs/local.log`。

#### 页面 6：审计（Audit）

展示协议合规审计数据：

- **Token 读取预算**：从 `request-size-audit.jsonl` 按阶段聚合，展示预算/实际/利用率表格，利用率超阈值标黄
- **字段读取记录**：从 `field-read-audit.jsonl` 读取最近 50 条，展示时间、请求、分组、字节数

数据源：`.loom/metrics/request-size-audit.jsonl` + `field-read-audit.jsonl`（JSONL 流式解析）。

## 5. 数据读取层与实时性设计

### 5.1 reader 层

`reader/` 模块是 Dashboard 与 Loom 数据的唯一接触面，直接复用现有 Serde 模型反序列化 JSON 文件。

| 数据类型 | 读取方式 | 缓存策略 |
|---------|---------|---------|
| `ProjectStatus` / `DeliveryIndex` | 全量反序列化 | 内存缓存 + 文件 mtime 失效 |
| 阶段产物（brainstorm/baseline/architecture/taskplan） | 按 `latestRefs` 路径按需读取 | LRU 缓存（最多 20 个文件） |
| `TaskPlanRun` + `TaskResult` | 按当前 phase 读取 | 内存缓存 + mtime 失效 |
| `ReviewResult` | 读 latest.json 指向的文件 | 内存缓存 + mtime 失效 |
| 部署日志 | tail 读取（最后 500 行） | 不缓存，每次请求读取 |
| 知识 chunks | 全量加载 chunk 元数据，正文按需读取 | 元数据缓存 + 正文 LRU |
| 审计 JSONL | 反向 seek 读取最后 N 条 | 不缓存 |

### 5.2 实时性：文件监听 + SSE

使用 `notify` crate 监听关键文件变更，通过 `tokio::broadcast` 广播到 SSE 通道。

#### 监听路径

- `.loom/status.json`
- `.loom/deliveries/*/index.json`
- `.loom/deliveries/*/tasks/*/runs/*.json`
- `.loom/deliveries/*/reviews/*/results/*.json`
- `.loom/deployment/state/*.json`
- `.loom/deployment/logs/local.log`
- `~/.loom/knowledge/registry.json`
- `~/.loom/knowledge/sources/*/build-runs/*/`

#### 事件去抖

100ms 去抖窗口，避免 Loom 批量写文件时产生事件风暴。

#### SSE 事件类型

| event | 触发条件 | 前端行为 |
|-------|---------|---------|
| `status` | `status.json` 变化 | 刷新总览页 + 底栏状态条 |
| `delivery` | `deliveries/*/index.json` 变化 | 刷新交付列表 + 当前详情页阶段时间线 |
| `tasks` | `runs/*.json` 变化 | 刷新任务列表（局部更新） |
| `reviews` | `reviews/*/results/*.json` 变化 | 刷新 review findings |
| `deploy` | `deployment/state/*.json` 变化 | 刷新部署状态卡片 |
| `deploy_logs` | `logs/local.log` 追加 | 追加日志行（不全量刷新） |
| `knowledge` | knowledge 目录变化 | 刷新知识库构建状态 |
| `heartbeat` | 每 15 秒 | 保活（防止代理超时断开 SSE） |

#### 部署日志实时流

日志文件是追加式的，特殊处理：记录上次读取的 offset，文件变更时从 offset 读取新增内容，按行拆分后通过 SSE 逐行推送。文件截断/truncate 时重置 offset。

前端日志组件维护环形缓冲区（最多 1000 行），收到新行时追加并自动滚动到底部（除非用户手动上滚查看历史）。

## 6. 前端技术细节与构建集成

### 6.1 技术栈

| 维度 | 选型 | 理由 |
|------|------|------|
| 框架 | React 18 | 生态最成熟，社区熟悉度高 |
| 构建 | Vite 5 | 快速 HMR，产物体积小 |
| 语言 | TypeScript | 与后端 JSON Schema 对齐，类型安全 |
| 路由 | React Router 6 | 多页面 SPA |
| 状态管理 | TanStack Query (React Query) | 天然契合"按需读取 + SSE 触发 refetch"模式 |
| 样式 | Tailwind CSS | 原子化，无需手写 CSS |
| 图标 | lucide-react | 轻量，tree-shakeable |
| 图表 | 轻量自绘（SVG/CSS） | 时间线、进度条、拓扑图用 SVG 自绘 |

### 6.2 SSE 与 TanStack Query 联动

SSE 事件触发对应 query 失效 → Query 自动 refetch，实现局部精确刷新：

- `status` 事件 → invalidate `['status']`
- `delivery` 事件 → invalidate `['deliveries']` + `['delivery', deliveryId]`
- `tasks` 事件 → invalidate `['tasks', deliveryId, phaseId]`
- `deploy_logs` 事件 → 直接追加日志行，不走 refetch

### 6.3 构建产物嵌入

Vite 构建输出到 `dashboard/embedded/dist/`，Rust 编译期通过 `rust-embed` 嵌入二进制。axum 提供静态文件服务：`GET /` → `index.html`，`GET /assets/*` → JS/CSS，其他路由 → `index.html`（SPA fallback）。

### 6.4 构建流程集成

修改 `./install.sh`，在 Rust 编译前增加前端构建步骤：

```bash
if [ -d "src/rust/dashboard/frontend" ]; then
  echo "Building dashboard frontend..."
  cd src/rust/dashboard/frontend
  npm ci --prefer-offline
  npm run build   # 产物输出到 ../embedded/dist/
  cd -
fi

cargo build --manifest-path src/rust/Cargo.toml -p setup -p mcp-server
```

如果 `embedded/dist/` 目录不存在，Rust 编译报清晰错误提示先运行前端构建。

### 6.5 类型同步

后端 axum 路由的响应类型用 `schemars` 派生 JSON Schema，构建脚本生成 TS 类型，保证前后端类型契约一致。

### 6.6 开发模式

支持 HMR 热更新：

```bash
# 终端 1：axum 后端（从磁盘读 frontend/dist，不嵌入）
LOOM_DASHBOARD_DEV=1 loom dashboard --port 9876

# 终端 2：Vite dev server（代理 API 到 9876）
cd src/rust/dashboard/frontend && npm run dev  # → http://localhost:5173
```

## 7. 错误处理与边界情况

### 7.1 后端错误分层

| 层级 | 错误类型 | 处理方式 | HTTP 响应 |
|------|---------|---------|-----------|
| L1 | 文件不存在（未初始化/未进入该阶段） | 返回空数据结构 | `200` + `{ "data": null }` |
| L2 | 反序列化失败（schema 变更/文件损坏） | 记录日志，返回 `data_error` | `200` + `{ "data": null, "warning": "..." }` |
| L3 | 路径越界（路径遍历攻击） | 拒绝，记录警告 | `400` + `{ "error": "invalid path" }` |
| L4 | 文件读取超时（200ms） | 取消读取，返回部分数据 | `200` + `{ "data": {...}, "partial": true }` |
| L5 | watcher 失败（inotify 资源耗尽） | 降级为轮询模式（每 5 秒） | SSE 发送 `degraded` 事件 |

### 7.2 前端错误处理

- **API 请求失败**：TanStack Query 内置 retry（2 次），失败后显示错误占位组件
- **SSE 断开**：自动重连（指数退避，最长 30 秒），底栏显示"连接中断"橙色标记
- **数据为 null**：显示"暂无数据"空状态，附说明
- **data_error 警告**：页面顶部黄色警告条

### 7.3 watcher 降级策略

```
正常模式: notify (inotify/FSEvents) → 实时 SSE
    ↓ 监听失败
降级模式: 每 5 秒轮询关键文件 mtime → SSE (延迟 ≤5s)
    ↓ 轮询也失败
最小模式: 仅按请求读取，无实时推送（前端手动刷新）
```

底栏始终显示当前模式：`● 实时` / `◐ 轮询(5s)` / `○ 手动`

### 7.4 边界情况

| 场景 | 处理 |
|------|------|
| 项目未 `loom initProject` | 显示引导页 |
| 多个 delivery 并行 | 交付列表全部展示，总览页高亮 active 的 |
| delivery 中途状态机转移 | SSE `delivery` 事件触发时间线刷新 |
| 大量 task（50+） | 任务表分页（每页 20）或虚拟滚动 |
| 超大日志文件（100MB+） | tail 最后 500 行，`fullLogRef` 供下载 |
| 知识库为空 | 显示空状态 + 引导 |
| `.loom/` 目录被删除 | 显示"项目状态丢失"错误页 |
| 端口被占用 | 自动递增 9876→9885，全占用则报错 |
| 并发 HTTP 请求 | reader 层 `RwLock` 保护缓存 |
| 前端版本与后端不匹配 | API 响应带 `schemaVersion`，前端检测显示升级提示 |

### 7.5 安全边界

- **仅监听 localhost**：axum 绑定 `127.0.0.1`，不暴露到网络
- **路径校验**：所有文件路径必须解析后落在 `projectRoot/.loom/` 或 `~/.loom/` 内，拒绝 `..` 越界
- **只读**：Dashboard 不提供任何写入/修改 API
- **敏感信息掩码**：日志查看器对 `secret`/`password`/`token`/`key` 字段做正则掩码（`***`）

## 8. 测试策略

### 8.1 Rust 后端测试

```
tests/rust/dashboard/
├── reader_test.rs         # reader 层反序列化测试
├── routes_test.rs         # HTTP API 集成测试
├── watcher_test.rs        # 文件监听 + SSE 测试
└── fixtures/              # 测试数据
    ├── minimal_project/   # 最小化 .loom/ 结构
    ├── full_delivery/     # 完整 delivery 产物
    ├── deploy_running/    # 部署运行中状态
    └── knowledge_built/   # 已构建知识库
```

测试重点：
- **reader 测试**：用 fixtures 模拟 `.loom/` 结构，验证各种数据组合的反序列化
- **路由测试**：axum 测试工具发请求，验证响应结构与错误码
- **watcher 测试**：临时目录写入/修改文件，验证 SSE 事件正确触发
- **降级测试**：模拟 watcher 失败，验证轮询降级

### 8.2 前端测试

- **组件单元测试**：Vitest + Testing Library，覆盖关键组件渲染与状态
- **SSE 联动测试**：mock EventSource，验证事件触发 Query invalidate
- **端到端测试**（可选）：Playwright 验证关键用户路径

### 8.3 集成测试

端到端集成测试：用 fixtures 准备完整 `.loom/` 项目 → 启动 dashboard 服务（随机端口）→ HTTP 请求各 API 验证响应 → 修改 fixtures 文件验证 SSE 推送 → 关闭服务。

遵循 AGENTS.md 约定：Rust 集成测试归入 `tests/rust/<domain>/`，先运行受影响包，再在发布前跑全量测试。

## 9. 数据规模参考

基于 Loom 现有常量，Dashboard 面临的数据规模：

| 维度 | 规模 | 来源 |
|------|------|------|
| 一个 delivery 的 phase 数 | 1-N（由 Roadmap 决定） | `contracts/brainstorm.rs:317-326` |
| 一个 phase 的 task 数 | 由 TaskPlan 决定 | `contracts/execution.rs:627-637` |
| 一个 task 的 attempt 数 | 受 repair 策略限制 | `execution.rs:600-625` |
| 知识 chunk 大小 | 700-1800 token | `knowledge/builder.rs:34-37` |
| 知识语义包大小 | ~6-8 chunk / 7000 token | `knowledge/builder.rs:39-41` |
| 部署日志尾取 | 120 行（Loom 端）→ 500 行（Dashboard 端） | `deploy/logs.rs:22` |

Dashboard 设计可承受单项目 50+ delivery、单 delivery 10+ phase、单 phase 50+ task 的规模。

## 10. 关键文件引用

| 主题 | 文件:行 |
|------|---------|
| `.loom` 常量 | `src/rust/state/paths.rs:7` |
| 项目路径结构 | `src/rust/state/paths.rs:38-59` |
| delivery 路径 | `src/rust/state/paths.rs:113-164` |
| ProjectStatus / DeliveryIndex | `src/rust/core/status.rs:82-150` |
| LoomMcpActionResult（7 状态） | `src/rust/core/action_result.rs:7-17` |
| BrainstormContract | `src/rust/contracts/brainstorm.rs:799-837` |
| TaskPlan / TaskDefinition | `src/rust/contracts/execution.rs:522-551, 325-374` |
| TaskPlanRun | `src/rust/contracts/execution.rs:649-666` |
| TaskResult | `src/rust/contracts/execution.rs:931-974` |
| ReviewResult | `src/rust/contracts/review.rs:139-161` |
| DeploymentSpec | `src/rust/contracts/deploy.rs:585-619` |
| 部署路径 | `src/rust/deploy/paths.rs:5-84` |
| 知识库路径 | `src/rust/knowledge/paths.rs:5-91` |
| KnowledgeChunk | `src/rust/knowledge/models.rs:262-283` |
| setup CLI | `src/rust/setup/main.rs` |
| Cargo 工作区 | `src/rust/Cargo.toml` |
