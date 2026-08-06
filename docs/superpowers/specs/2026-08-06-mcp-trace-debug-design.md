# MCP 流量追踪开关 — 设计规格

**日期：** 2026-08-06
**状态：** 已批准（待实现）
**范围：** 仅限 Loom 的 `mcp-server` crate —— 不引入新的外部依赖（复用已有的 workspace `chrono`），不与 rmcp 耦合，不改动 setup/registration。

## 1. 问题

调试 Loom MCP server 不方便，因为 stdio 上的原始 JSON-RPC 请求/响应流量不可见。
代码库中没有任何现成的 MCP 线上流量 dump、trace 或 debug 日志。开发者希望有一个
可选开关，开启后把每条 MCP 请求和响应以人类可读的形式写入文件，便于在复现问题时
通过 `tail -f` 等方式查看。

经头脑风暴确认的决策是：**Loom 内置开关**（而非通用独立 MCP proxy），捕获
**全部原始线上流量**（initialize、tools/list、tools/call、resources/read、notifications
—— stdio 上的所有内容），输出为**人类可读的 pretty-print** 记录。

## 2. 背景（已从代码库核实）

- MCP server 是**仅 stdio** 的，基于 `rmcp` crate（v1.7.0）。
  `src/rust/mcp-server/server.rs:933` 的 `run_stdio_server()` 调用
  `LoomMcpServer::from_env().serve(stdio())`。
- rmcp 的 `serve()` 是泛型的，接受任意 `(R, W)` 元组，其中
  `R: AsyncRead + Send + 'static + Unpin`、`W: AsyncWrite + Send + 'static + Unpin`
  （`IntoTransport for (R, W)` 位于 `rmcp/transport/async_rw.rs:24-33`）。因此我们
  可以用 tee 式的 reader/wrapper 包装替换 `stdio()`，rmcp 会透明接受，无需 fork 也
  无需 trait 耦合。
- MCP stdio 帧格式是**换行分隔的 JSON-RPC**：rmcp 用 `read_until(b'\n')` 读取
  （`async_rw.rs:128`），编码器在末尾追加 `\n`（`async_rw.rs:436`）。因此按行缓冲是
  从字节流重组完整消息的正确策略。
- 现有日志使用 `log` + `env_logger`，在 `main.rs:11-39` 初始化。日志刻意写入**文件**
  （`$LOOM_HOME/log/loom-mcp.log`），绝不写 stdout/stderr（那两个承载 MCP 协议流量）。
  `main.rs:41-53` 的 `loom_log_dir()` 解析 `$LOOM_HOME/log`（回退到 `~/.loom/log`）。
- 配置是**基于环境变量**的（`LOOM_HOME`、`LOOM_HOST`、`RUST_LOG`、`LOOM_RUNTIME_HOME`），
  没有 server 配置文件。新增一个 env-var 开关完全契合现有模式。
- `setup` crate 把启动配置（`command`/`args`/`env`）写入各 agent 的原生配置文件。由于
  本特性是进程内 env-var 开关，**无需改动 registration 或 plugin** —— 用户只需在环境
  中（或 agent 的 env 块里）设置 `LOOM_MCP_TRACE=1` 即可启用。
- mcp-server crate 的依赖已包含 `tokio`、`serde_json`、`log`。唯一的 `Cargo.toml` 改动
  是给 `mcp-server/Cargo.toml` 加一行 `chrono.workspace = true`（`chrono = "0.4"` 已是
  workspace 依赖，见 `src/rust/Cargo.toml:37`，被兄弟 crate 使用，因此不引入新的外部依赖
  —— 只是复用 lockfile 中已有的内容）。

## 3. 目标与非目标

### 目标

- 捕获 stdio 传输双向上的**每一条** MCP 线上消息（请求、响应、通知、server 侧通知、
  解析错误）。
- 产出**人类可读**、pretty-print 的文件，适合 `tail -f` 和肉眼调试。
- **可选开启**且**关闭时零开销**：不创建文件、不 spawn 线程、无可测量的性能影响。
- **与 rmcp 内部解耦** —— 在 rmcp 升级时无需改动。
- 绝不因 tracing 导致 server 启动失败。

### 非目标

- 通用的、独立的 MCP proxy，可配合第三方 server（头脑风暴中已明确排除；可作为后续
  跟进）。
- 捕获 HTTP/SSE 流量（当前 server 仅 stdio）。
- 对捕获流量中的密钥做脱敏或过滤（该文件是 `$LOOM_HOME/log` 下的本地调试产物，按现有
  `loom-mcp.log` 同等对待）。
- 结构化 / NDJSON 输出、远程流式推送、或 TUI 查看器。
- 大消息截断（v1 按 YAGNI 处理；未来可加 `LOOM_MCP_TRACE_MAX_BYTES` 上限，但本次不实现）。

## 4. 架构

### 4.1 数据流

```
agent ──stdin──▶ TeeRead ──▶ rmcp AsyncRwTransport ──▶ LoomMcpServer
                   │ 复制已读字节                              │
                   ▼                                          ▼
               TraceSink ◀─────────────────────────────── TeeWrite ──stdout──▶ agent
                   │ send(TraceEvent{dir, bytes})  (非阻塞，std mpsc)
                   ▼
            writer 线程 (std) ── 按 dir 行缓冲 ──▶ 解析 JSON ──▶ pretty-print ──▶ BufWriter<File>
```

- `TeeRead<R: AsyncRead>` 包装真实 stdin。在 `poll_read` 中，内部 read 返回
  `Poll::Ready(Ok(n))` 后，把已读的 `n` 字节复制到 `TraceSink`（方向 `Dir::In`），再返回
  原始 `Poll` 结果。它绝不修改字节 —— 协议行为与今天逐字节一致。
- `TeeWrite<W: AsyncWrite>` 包装真实 stdout。在 `poll_write` 中，内部 write 返回
  `Poll::Ready(Ok(n))` 后，把已写的 `n` 字节复制到 `TraceSink`（方向 `Dir::Out`），再返回
  原始 `Poll` 结果。`poll_flush` 和 `poll_close` 直接透传（tee 侧的刷新由 writer 线程的行
  缓冲 + 每条记录 flush 处理；我们**不**在每次 `poll_flush` 时刷新 trace 文件，因为 rmcp
  可能在消息中途调用 flush）。
- `TraceSink` 是一个轻量、`Clone`、`Send + Sync` 的句柄，持有
  `std::sync::mpsc::Sender<TraceEvent>`（包在 `Option` 里，使关闭场景成为无分配的 null
  sink）。`append(&self, dir, bytes)` 做非阻塞的 `send().ok()` —— 若 receiver 已被丢弃
  （writer 线程已死），静默无操作。从 `poll_read`/`poll_write` 调用是安全的，因为
  `mpsc::Sender::send` 从不阻塞（无界 channel）。
- 一个专用 **std 线程**持有 `mpsc::Receiver`、两个行缓冲（每个 `Dir` 一个，因为 in/out
  的 chunk 会任意交错）和一个 `BufWriter<File>`。它在 async runtime 之外执行阻塞式文件
  I/O。channel 挂断时（server 关闭、`TeeRead`/`TeeWrite` 被 drop），它把任何残留的不完整
  行缓冲 flush 出来并标记 `(incomplete)`，然后退出。

### 4.2 行重组

由于 MCP stdio 以 `\n` 分隔，writer 线程为每个 `Dir` 维护一个 `Vec<u8>` 缓冲。对每条
到来的 `TraceEvent`：

1. 把 `bytes` 追加到该 `Dir` 的缓冲。
2. 当缓冲中包含 `\n` 时：
   - 提取到并包含 `\n` 为止的字节作为一行。
   - 去掉末尾的 `\n`（若存在末尾 `\r` 也一并去掉，以增强鲁棒性）。
   - 为该行输出一条 trace 记录。
3. 最后一个 `\n` 之后的任何字节留在缓冲中，等待下一个事件。

这正确处理了：一条消息被拆分到多次 `poll_read`/`poll_write` 调用；一次调用中交付多条
消息；以及一次调用结束时行不完整（缓冲到下一个 `\n` 到来）。

### 4.3 为什么是字节级 tee（而非 Transport-trait 包装）

用 rmcp 的 `AsyncRwTransport` 外包一层 `Transport<RoleServer>` 包装，会在
`receive()`/`send()` 中记录已解析的 `RxJsonRpcMessage`/`TxJsonRpcMessage`。此方案被
否决，原因：

- 它只能看到 rmcp yield 的内容。rmcp 会静默丢弃非标准通知
  （`try_parse_with_compatibility` 返回 `Ok(None)` → `continue`，见 `async_rw.rs:144`），
  这些将不可见 —— 与“全部原始流量”的要求相悖。
- 它重新序列化消息，而非展示精确的线上字节，有字段顺序或未知字段漂移的风险。
- 它把实现耦合到 rmcp 的 `Transport` trait API，该 API 跨 rmcp 版本可能变化。

字节级 tee 看到精确的线上字节、不丢弃任何内容，且只依赖稳定的 tokio
`AsyncRead`/`AsyncWrite` trait。

## 5. 配置

单一新增环境变量 `LOOM_MCP_TRACE`。`from_env()` 读取后**先 trim**，再分类：

| trim 后的值                       | 效果                                                                  |
|-----------------------------------|-----------------------------------------------------------------------|
| 未设置 / `0` / 空                 | **关闭**。`TraceConfig::from_env()` 返回关闭配置，`TraceSink::open` 返回 null sink。不创建文件、不 spawn 线程。 |
| `1`                               | **开启**，默认路径 `$LOOM_HOME/log/loom-mcp-trace.log`。              |
| 其他任何非空值                    | **开启**，按自定义文件路径处理（初始 trim 之后的原始值）。            |

- `LOOM_HOME` 解析复用现有 `loom_log_dir()` 逻辑（移入新的 `trace` 模块，并由 `main.rs`
  的 `init_logging` 共享，消除重复）。
- trace 文件以 `create(true).append(true)` 打开（与现有 `loom-mcp.log` 行为一致，见
  `main.rs:16-19`）。
- 打开时写一条启动横幅，以便在 append 文件中区分多次启动：
  ```
  === loom-mcp-server started 2026-08-06T12:34:56.789Z pid=12345 trace=enabled ===
  ```
- **失败不致命。** 若文件无法打开或 writer 线程无法 spawn，`TraceSink::open` 向现有
  `loom-mcp.log` 记一条 `warn!` 并返回 null（关闭的）sink。server 必须仍能正常启动并服务。

## 6. 输出格式

每条线上消息恰好产出一条记录。一条记录为：

```
<标记> <时间戳> <method-or-id 标签>
<pretty-print 的 JSON 体>

```

- `<标记>`：`>>` 表示入站（client→server：请求和通知），`<<` 表示出站（server→client：
  响应和 server 发出的通知）。
- `<时间戳>`：ISO 8601，毫秒精度，`Z` 后缀，例如 `2026-08-06T12:34:56.789Z`，**UTC**。
  由 writer 线程经 `chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ")` 生成。`chrono`
  作为 `mcp-server` 依赖加入（已是 workspace dep —— 见 §2）。
- `<method-or-id 标签>`：
  - 请求和通知：`method` 字段，若存在 `id` 则附带 `id=N`（请求有 id；通知没有）。示例：
    `tools/call id=1`。
  - 响应：`id=N`（匹配请求 id）。示例：`id=1`。
  - 无 `id` 的响应（rmcp 会发出 `id: null` 的 parse-error 响应）：`id=null`。
- `<pretty-print 的 JSON 体>`：对解析后的行做 `serde_json::to_string_pretty`，2 空格缩进。
  后接一个空行。

### 示例

```
>> 2026-08-06T12:34:56.789Z initialize id=1
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": "2025-11-25",
    "capabilities": {},
    "clientInfo": { "name": "codex", "version": "1.0.0" }
  }
}

>> 2026-08-06T12:34:56.790Z notifications/initialized
{
  "jsonrpc": "2.0",
  "method": "notifications/initialized"
}

<< 2026-08-06T12:34:56.801Z id=1
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocolVersion": "2025-11-25",
    "serverInfo": { "name": "loom-mcp-server", "version": "0.2.7" },
    "capabilities": { "tools": {}, "resources": {} }
  }
}

>> 2026-08-06T12:34:57.000Z tools/call id=4
{
  "jsonrpc": "2.0",
  "id": 4,
  "method": "tools/call",
  "params": {
    "name": "loom.status",
    "arguments": { "projectRoot": "/home/haoshijie/project/loom" }
  }
}

<< 2026-08-06T12:34:57.012Z id=4
{
  "jsonrpc": "2.0",
  "id": 4,
  "result": {
    "content": [ { "type": "text", "text": "..." } ],
    "structuredContent": { "state": "failed", "error": { "code": "STATE_NOT_INITIALIZED" } }
  }
}

```

### 边界情况记录

- **非 JSON 或非 UTF8 行**（rmcp 会记 parse error 并跳过，但 tee 仍看到了字节）：
  ```
  >> 2026-08-06T12:34:58.000Z (unparseable, 42 bytes)
  <原始字节的 lossy-utf8 渲染>
  
  ```
- **关闭时残留的不完整行**（channel 关闭后，writer 线程排出一个没有结尾 `\n` 的缓冲）：
  ```
  >> 2026-08-06T12:35:00.000Z (incomplete, 17 bytes)
  <lossy-utf8 渲染>
  
  ```

### 刷新

writer 线程在每条记录后调用 `BufWriter::flush()`，使 `tail -f` 能实时看到记录。由于
tracing 是可选的调试用途，这是可接受的。

## 7. 模块放置与接线

### 新模块：`src/rust/mcp-server/trace.rs`

包含：

- `pub enum Dir { In, Out }` —— 显示为 `>>` / `<<`。
- `pub struct TraceConfig { enabled: bool, path: Option<PathBuf> }`，`from_env()` 读取
  `LOOM_MCP_TRACE` 并通过 `loom_log_dir()` 解析默认路径。
- `pub fn loom_log_dir() -> PathBuf` —— 从 `main.rs:41-53` **移入**此处（`main.rs` 中的
  本地副本删除；`init_logging` 改调 `trace::loom_log_dir()`）。
- `pub struct TraceEvent { dir: Dir, bytes: Vec<u8> }`。
- `pub struct TraceSink { tx: Option<std::sync::mpsc::Sender<TraceEvent>> }` —— `Clone`、
  `Send + Sync`。`Option` 使关闭场景无分配。方法：
  - `open(cfg: &TraceConfig) -> TraceSink` —— 若关闭，返回 null sink。若开启，打开文件
    （`create+append`）、spawn writer 线程、返回持有 sender 的 sink。任何错误都 `warn!`
    并返回 null sink。
  - `append(&self, dir: Dir, bytes: &[u8])` ——
    `if let Some(tx) = &self.tx { let _ = tx.send(TraceEvent { dir, bytes: bytes.to_vec() }); }`。
    每次实际 read/write 分配一个 `Vec`；关闭场景不分配。
  - `disabled() -> TraceSink` —— 直接构造 null sink（测试用）。
- `pub struct TeeRead<R> { inner: R, sink: TraceSink, dir: Dir }` ——
  `impl<R: AsyncRead + Unpin> AsyncRead for TeeRead<R>`。
- `pub struct TeeWrite<W> { inner: W, sink: TraceSink, dir: Dir }` ——
  `impl<W: AsyncWrite + Unpin> AsyncWrite for TeeWrite<W>`。
- writer 线程函数：持有 `Receiver<TraceEvent>`、两个按 `Dir` 分的 `Vec<u8>` 行缓冲、一个
  `BufWriter<File>`。循环 `recv()`，把字节喂入对应 dir 的缓冲，按 `\n` 切分，输出记录。
  遇 `RecvError`（channel 关闭）时，把任何残留缓冲标记 `(incomplete)` flush 出来并返回。
- 记录输出辅助函数：接收 `&[u8]` 行 + `Dir` + 时间戳，解析为 `serde_json::Value`，推导
  method/id 标签，pretty-print，写入头部 + 体 + 空行，flush。

### `lib.rs`

新增 `pub mod trace;`（与现有 `pub mod resource_registry;`、`pub mod server;`、
`pub mod tool_registry;` 并列）。

### `Cargo.toml`

在 `mcp-server/Cargo.toml` 的 `[dependencies]` 中加 `chrono.workspace = true`（一行；
`chrono = "0.4"` 已在 `src/rust/Cargo.toml:37` 声明）。

### `main.rs`

`init_logging` 改调 `mcp_server::trace::loom_log_dir()` 而非本地的 `loom_log_dir()`。
本地 `loom_log_dir` 函数（第 41-53 行）删除。`main.rs` 无其他改动。

### `server.rs` —— `run_stdio_server()`（第 933-937 行）

改为：

```rust
pub async fn run_stdio_server() -> anyhow::Result<()> {
    let cfg = trace::TraceConfig::from_env();
    let (stdin, stdout) = stdio();
    let sink = trace::TraceSink::open(&cfg);
    let stdin = trace::TeeRead::new(stdin, sink.clone(), trace::Dir::In);
    let stdout = trace::TeeWrite::new(stdout, sink, trace::Dir::Out);
    let service = LoomMcpServer::from_env().serve((stdin, stdout)).await?;
    service.waiting().await?;
    Ok(())
}
```

`serve()` 通过现有的 `IntoTransport for (R, W)` 实现接受 `(stdin, stdout)` —— 无其他
管道改动。tracing 关闭时，`TeeRead`/`TeeWrite` 仍包装真实句柄，但其 `sink.append()` 只是
一次 `Option::is_none` 检查，开销可忽略（每次 read/write 一个分支）。

## 8. 并发与安全

- `TraceSink` 是 `Clone + Send + Sync`。`mpsc::Sender` 是 `Send + Sync`。clone 给
  `TeeRead` 和 `TeeWrite` 各一个指向同一 writer 线程的句柄。
- 无界 channel 上的 `mpsc::Sender::send` 从不阻塞，因此从 `poll_read`/`poll_write` 调用
  不会卡住 async runtime。每次 read/write 分配一个 `Vec<u8>`，对可选的调试路径是可接受的。
- writer 线程是普通的 `std::thread::spawn`，做阻塞式 `File` I/O —— 不触碰 tokio runtime。
  panic 时 `Receiver` 被 drop，后续 `send().ok()` 静默无操作。
- 若 trace 文件在长时间调试会话中无限增长，由用户负责（开关是可选的调试用途）。未来的
  `max_bytes`/rotation 上限是 v1 的已注明非目标。

## 9. 测试

### 单元测试（`trace.rs`，`#[cfg(test)]`）

- `tee_read_copies_bytes_without_altering_them` —— `TeeRead` 套在 `Cursor` 上，以奇数
  chunk 大小读取，断言返回字节等于输入，且 sink 收到相同字节。
- `tee_write_copies_bytes_without_altering_them` —— `TeeWrite` 套在 `Vec<u8>` 上，类似。
- `line_buffer_reassembles_across_chunks` —— 把 `b'{"a":1}\n{"b":2}\n'` 分块喂入
  （`b'{"a"'`、`b':1}\n{"b'`、`b':2}\n'`），断言输出两条完整记录。
- `line_buffer_handles_partial_final_line` —— 喂入 `b'{"a":1}\n{"b"'` 后关闭 channel，
  断言一条完整记录 + 一条 `(incomplete)` 记录。
- `record_header_for_request` / `_response` / `_notification` —— 断言各类消息的 `>>`/`<<`
  标记、时间戳格式、method/id 标签。
- `disabled_sink_creates_no_file` —— 在临时目录中 `TraceSink::open(&disabled_cfg)` 不产生
  文件、不 spawn 线程；`append` 是无操作。
- `non_json_line_tagged_unparseable` —— 喂入 `b'not json\n'`，断言记录标记为
  `(unparseable, N bytes)` 并带 lossy 渲染。

### 集成测试（`tests/rust/mcp-server/mcp_trace_smoke.rs`）

复用 `server_smoke.rs:103-153` 的 `McpProcess` harness 模式（把 `loom-mcp-server` 二进制
作为子进程 spawn，通过其 stdin/stdout 驱动 JSON-RPC）。用例：

1. `trace_file_captures_full_session` —— 以 `LOOM_MCP_TRACE=<临时路径>` spawn；依次跑
   `initialize` → `notifications/initialized` → `tools/list` → `tools/call`
   （`loom.status`）；等进程退出（或 kill 之）；读 trace 文件；断言其中包含：启动横幅；
   `>>` 记录覆盖 `initialize`、`notifications/initialized`、`tools/list`、`tools/call`，
   method/id 正确、体为 pretty-print；`<<` 记录覆盖 `initialize` 和 `tools/call` 的响应，
   id 匹配、体为 pretty-print。
2. `trace_disabled_creates_no_file` —— 以 `LOOM_MCP_TRACE` 未设置 spawn；跑一次
   `initialize`；断言进程退出后默认路径下不存在 trace 文件。

集成测试必须把 `LOOM_HOME` 设为临时目录（使默认路径可预测解析），并在 `Drop` 中清理。

## 10. 文档

在 README 中加一段简短的排障说明（若已有排障/调试小节则并入，否则新建一个小节）：

> **调试 MCP 流量。** 设置 `LOOM_MCP_TRACE=1` 可把每条 MCP 请求和响应（包括
> `initialize` 握手和通知）dump 到 `$LOOM_HOME/log/loom-mcp-trace.log`。该文件为人类可读
> 的 pretty-print 格式；复现问题时可用 `tail -f` 实时观看。设置
> `LOOM_MCP_TRACE=/some/path.log` 可写到别处。开关默认关闭，关闭时无任何开销。

## 11. 范围外 / 后续工作

- 面向第三方 server 的通用独立 MCP proxy（v1 排除；可能的后续）。
- HTTP/SSE 传输捕获（当前 server 仅 stdio）。
- 捕获流量中的密钥脱敏。
- trace 文件的大小上限 / 轮转（`LOOM_MCP_TRACE_MAX_BYTES`）。
- NDJSON / 机器可解析的输出模式。
- trace 的 TUI 或 web 查看器。
