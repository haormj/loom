# Loom 知识模块日志设计文档

## 1. 背景与目标

在调试知识库功能时发现整个 knowledge 模块没有任何日志输出，无法定位搜索失败、provider 连接异常、构建中断等问题。本方案为知识模块的关键路径引入结构化日志，方便开发调试和线上问题排查。

## 2. 技术选型

| 候选方案 | 优点 | 缺点 | 结论 |
|----------|------|------|------|
| `log` + `env_logger` | 标准生态、轻量、零侵入、同步无性能损耗 | 无异步、无 span/trace | ✅ 选定 |
| `tracing` + `tracing-subscriber` | 结构化 span/trace、异步友好 | 引入较重依赖、API 复杂、对现有同步代码侵入大 | ❌ 过重 |
| `slog` | 结构化 JSON 输出、插件式 | 生态较窄、API 与现有代码风格不一致 | ❌ 不选 |

**选定理由**：loom knowledge 模块是同步代码库（无 tokio），`log` + `env_logger` 是最轻量够用的方案，不引入 async 运行时依赖，与现有 `ureq` 同步 HTTP 客户端的风格一致。

## 3. 日志架构

### 3.1 输出目标

| 项 | 说明 |
|----|------|
| **输出方式** | 直接写入文件 |
| **日志文件路径** | `$LOOM_HOME/log/loom-mcp.log`（默认 `~/.loom/log/loom-mcp.log`） |
| **原因** | MCP 服务器通过 stdio 传输协议消息，stdout/stderr 不便查看；直接写入文件方便用户随时 `tail -f` 查看和排查问题 |
| **写入模式** | 追加（append），多次启动不会覆盖历史日志 |
| **格式** | `[timestamp LEVEL module] message` |
| **时间戳** | 毫秒精度，ISO 8601 格式（如 `2026-08-06T08:15:23.456Z`） |

`LOOM_HOME` 解析规则（与知识模块 `paths.rs` 一致）：
1. 优先使用环境变量 `LOOM_HOME`
2. 未设置时回退到 `~/.loom`（`$HOME` 或 `$USERPROFILE`）

### 3.2 日志级别

| 级别 | 用途 | 示例 |
|------|------|------|
| `ERROR` | 不可恢复的错误（目前由 `Result` 错误返回处理，暂不使用） | — |
| `WARN` | 可恢复的异常：provider 连接失败、HTTP 错误、非本地 provider 拒绝构建 | `provider 'confluence-kb' failed: connection refused` |
| `INFO` | 关键操作入口/出口：搜索请求、构建开始/完成、检视请求、配置加载、语义包提交 | `knowledgeSearch: query="...", returning 8 cards` |
| `DEBUG` | 调试细节：provider 分发、HTTP 请求/响应详情、知识源数量、文档发现结果 | `create_provider: source 'x' -> OpenViking (endpoint=...)` |
| `TRACE` | 极细粒度（暂未使用，预留） | — |

### 3.3 默认级别与环境变量

```bash
# 默认（不设置 RUST_LOG 时）
# 等效于 loom=info,knowledge=info

# 知识模块调试
export RUST_LOG="knowledge=debug"

# 全量调试
export RUST_LOG="debug"

# 仅看搜索和 provider
export RUST_LOG="knowledge::search=debug,knowledge::provider=debug"

# 生产环境（仅警告）
export RUST_LOG="warn"
```

`env_logger` 初始化代码位于 `mcp-server/main.rs`：

```rust
fn init_logging() -> anyhow::Result<()> {
    let log_dir = loom_log_dir();
    std::fs::create_dir_all(&log_dir)?;
    let log_file = log_dir.join("loom-mcp.log");

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)?;

    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("loom=info,knowledge=info"),
    )
    .format_timestamp_millis()
    .target(env_logger::Target::Pipe(Box::new(file)))
    .format(|buf, record| {
        writeln!(
            buf,
            "[{} {} {}] {}",
            buf.timestamp_millis(),
            record.level(),
            record.target(),
            record.args()
        )
    })
    .init();

    Ok(())
}
```

`loom_log_dir()` 按 `LOOM_HOME` → `~/.loom` 的优先级解析日志目录（与 `paths.rs` 一致），返回 `<loom_home>/log`。

## 4. 关键路径日志覆盖

### 4.1 搜索路径（`search.rs`）

| 日志点 | 级别 | 内容 |
|--------|------|------|
| `search_knowledge` 入口 | INFO | 查询参数、知识源过滤、语义焦点、block、limit |
| `search_cards` 源统计 | DEBUG | 启用源数量（本地/Provider 分类） |
| Provider 查询开始 | DEBUG | 知识源名称 |
| Provider 返回结果 | DEBUG | 知识源名称、返回卡片数 |
| Provider 失败 | WARN | 知识源名称、错误信息 |
| `search_knowledge` 出口 | INFO | 最终返回卡片数 |

### 4.2 Provider 路径（`provider.rs`）

| 日志点 | 级别 | 内容 |
|--------|------|------|
| `create_provider` 分发 | DEBUG | 知识源名称 → provider 类型（Local/OpenViking）+ 关键配置 |
| OpenViking 搜索请求 | INFO | 知识源名称、查询（截断至 100 字符）、focus 数量、limit、min_score |
| OpenViking 搜索结果 | INFO | 返回总数（resources/memories/skills 分类）、过滤数、保留数、min_score |
| OpenViking 检视请求 | DEBUG | 知识源名称、chunk_id（viking:// URI） |
| HTTP 错误响应 | WARN | HTTP 状态码、响应体 |
| 传输层错误 | WARN | 传输错误详情 |

> **业务关键信息提升至 INFO**：OpenViking 搜索的入参与结果（含过滤数）在 INFO 级别即可见，无需开启 DEBUG。这样默认日志级别下用户即可看到每次查询的完整链路：请求入参 → provider 返回/过滤 → 最终返回卡片数。
>
> **ureq 日志抑制**：`init_logging` 通过 `filter_module("ureq", Warn)` 屏蔽 ureq 内部 DEBUG 日志（HTTP prelude、请求头、响应行），避免每次调用打印完整 HTTP 报文。HTTP 错误已由 `map_ureq_error` 转为结构化 WARN 日志。

### 4.3 构建路径（`builder.rs`）

| 日志点 | 级别 | 内容 |
|--------|------|------|
| `build_source` 入口 | INFO | 知识源名称、project_root |
| 非本地 provider 拒绝 | WARN | 知识源名称 |
| 文档发现结果 | DEBUG | 发现文件数、跳过文件数 |
| 构建完成 | INFO | 知识源名称、chunk 数、build_id |

### 4.4 检视路径（`inspect.rs`）

| 日志点 | 级别 | 内容 |
|--------|------|------|
| `inspect_chunk` 入口 | INFO | 知识源名称、build_id、chunk_id |

### 4.5 配置加载路径（`store.rs`）

| 日志点 | 级别 | 内容 |
|--------|------|------|
| `load_providers_yaml` 文件不存在 | DEBUG | 路径 |
| `load_providers_yaml` 加载成功 | DEBUG | 路径、源数量 |
| `load_merged_registry` 合并开始 | INFO | yaml 源数、registry 源数 |
| 合并：更新已有源 | DEBUG | 源名称 |
| 合并：新增源 | DEBUG | 源名称 |

### 4.6 操作路径（`operations.rs`）

| 日志点 | 级别 | 内容 |
|--------|------|------|
| `add_source` 入口 | INFO | 知识源名称、路径数 |

### 4.7 语义包提交路径（`semantic.rs`）

| 日志点 | 级别 | 内容 |
|--------|------|------|
| `submit_semantic_pack` 入口 | INFO | request_ref、project_root |

## 5. 日志输出示例

### 正常搜索流程

```
[2026-08-06T08:15:23.456Z INFO  knowledge::operations] knowledgeAdd: name='confluence-kb', 3 paths
[2026-08-06T08:15:30.123Z INFO  knowledge::search] knowledgeSearch: query="用户认证配置", sources=["confluence-kb"], focus=[], block=None, limit=20
[2026-08-06T08:15:30.124Z INFO  knowledge::store] load_merged_registry: merging 1 providers.yaml sources into 2 registry sources
[2026-08-06T08:15:30.124Z DEBUG knowledge::store] load_merged_registry: adding new OpenViking source 'confluence-kb'
[2026-08-06T08:15:30.125Z DEBUG knowledge::provider] create_provider: source 'confluence-kb' -> OpenViking (endpoint=http://openviking.internal:1933)
[2026-08-06T08:15:30.125Z DEBUG knowledge::search] search_cards: 1 enabled sources (local=0, provider=1)
[2026-08-06T08:15:30.126Z DEBUG knowledge::search] search_cards: querying provider 'confluence-kb'
[2026-08-06T08:15:30.127Z INFO  knowledge::provider] openviking[confluence-kb]: search query="用户认证配置" focus=0 limit=20 min_score=0.2
[2026-08-06T08:15:30.341Z INFO  knowledge::provider] openviking[confluence-kb]: result 5 returned (resources=5, memories=0, skills=0), 2 filtered by min_score=0.2, 3 kept
[2026-08-06T08:15:30.342Z DEBUG knowledge::search] search_cards: provider 'confluence-kb' returned 3 cards
[2026-08-06T08:15:30.342Z INFO  knowledge::search] knowledgeSearch: returning 3 cards
```

### Provider 故障场景

```
[2026-08-06T08:16:10.000Z INFO  knowledge::search] knowledgeSearch: query="部署指南", sources=[], focus=[], block=None, limit=20
[2026-08-06T08:16:10.001Z DEBUG knowledge::search] search_cards: 2 enabled sources (local=1, provider=1)
[2026-08-06T08:16:10.002Z DEBUG knowledge::search] search_cards: querying provider 'confluence-kb'
[2026-08-06T08:16:10.003Z INFO  knowledge::provider] openviking[confluence-kb]: search query="部署指南" focus=0 limit=20 min_score=0.2
[2026-08-06T08:16:10.005Z WARN  knowledge::provider] openviking[confluence-kb]: transport error: Connection refused
[2026-08-06T08:16:10.005Z WARN  knowledge::search] search_cards: provider 'confluence-kb' failed: OpenViking source 'confluence-kb' unreachable: Connection refused
[2026-08-06T08:16:10.100Z INFO  knowledge::search] knowledgeSearch: returning 3 cards
```

## 6. 日志保存策略

### 6.1 当前策略：直接写入文件

loom MCP 服务器作为 stdio 进程运行，stdout/stderr 用于 MCP 协议传输，不便查看日志。因此日志直接写入文件 `$LOOM_HOME/log/loom-mcp.log`（默认 `~/.loom/log/loom-mcp.log`），用户可随时 `tail -f` 查看。

| 项 | 说明 |
|----|------|
| **日志文件路径** | `$LOOM_HOME/log/loom-mcp.log` |
| **写入模式** | 追加（append），多次启动不会覆盖历史日志 |
| **目录创建** | 启动时自动创建 `log/` 目录（`create_dir_all`） |
| **文件创建** | 不存在时自动创建（`OpenOptions::create`） |

### 6.2 查看日志

```bash
# 实时查看
tail -f ~/.loom/log/loom-mcp.log

# 查看最近 100 行
tail -n 100 ~/.loom/log/loom-mcp.log

# 过滤搜索请求
grep "knowledgeSearch" ~/.loom/log/loom-mcp.log

# 过滤 provider 错误
grep "WARN" ~/.loom/log/loom-mcp.log
```

### 6.3 日志轮转

**设计决策**：不在 Rust 代码中实现日志文件轮转。原因：

1. `env_logger` 本身不提供文件轮转功能，引入 `tracing-appender` 等会增加复杂度
2. 日志轮转是运维层面关注点，应由基础设施处理
3. 追加模式下单次会话日志量有限（INFO 级别约 2-4 条/请求），长期积累可手动清理或配合 logrotate

#### logrotate 配置（可选）

```
~/.loom/log/loom-mcp.log {
    daily
    rotate 7
    compress
    delaycompress
    missingok
    notifempty
    size 50M
}
```

## 7. 性能影响

| 项 | 说明 |
|----|------|
| 日志宏开销 | `log` crate 的宏在编译时展开为级别判断，关闭的级别零开销（被优化掉） |
| 默认级别（info） | 仅 INFO 及以上输出，单次搜索约 2-4 条日志，可忽略 |
| DEBUG 级别 | 单次搜索约 8-15 条日志，含 HTTP 请求详情，适用于调试场景 |
| 格式化开销 | 仅在日志级别启用时才格式化字符串参数 |
| I/O 阻塞 | 文件写入是同步阻塞的（append 模式），但单条日志 < 1KB，开销可忽略 |

## 8. 依赖变更

| crate | 版本 | 用途 |
|-------|------|------|
| `log` | `0.4` | 日志 facade（workspace 级） |
| `env_logger` | `0.11` | 日志实现（仅 mcp-server 二进制入口） |

## 9. 变更文件清单

| 文件 | 变更 |
|------|------|
| `src/rust/Cargo.toml` | workspace 新增 `log`、`env_logger` 依赖 |
| `src/rust/mcp-server/Cargo.toml` | 新增 `env_logger` 依赖 |
| `src/rust/mcp-server/main.rs` | 初始化 `env_logger`，写入文件 `$LOOM_HOME/log/loom-mcp.log`（append 模式、毫秒时间戳、默认级别） |
| `src/rust/knowledge/Cargo.toml` | 新增 `log` 依赖 |
| `src/rust/knowledge/search.rs` | 搜索路径日志（info/debug/warn） |
| `src/rust/knowledge/provider.rs` | provider 调度与 HTTP 请求日志（debug/warn） |
| `src/rust/knowledge/inspect.rs` | 检视路径日志（info） |
| `src/rust/knowledge/builder.rs` | 构建路径日志（info/debug/warn） |
| `src/rust/knowledge/operations.rs` | 操作路径日志（info） |
| `src/rust/knowledge/store.rs` | 配置加载与合并日志（info/debug） |
| `src/rust/knowledge/semantic.rs` | 语义包提交日志（info） |

## 10. 未来扩展

| 方向 | 何时考虑 |
|------|----------|
| 迁移到 `tracing` | 当 loom 引入 async/tokio 运行时时 |
| JSON 结构化日志 | 当需要接入 ELK/Loki 等日志聚合系统时 |
| 请求 ID 关联 | 当需要追踪单次 MCP 请求的完整调用链时 |
| 日志采样 | 当 DEBUG 级别在高频路径产生过多日志时 |
