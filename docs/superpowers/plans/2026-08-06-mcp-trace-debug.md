# MCP 流量追踪开关 — 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为 Loom MCP server 增加一个 `LOOM_MCP_TRACE` 环境变量开关，开启后把 stdio 上的全部 MCP JSON-RPC 请求/响应以 pretty-print 格式写入文件，便于调试。

**Architecture:** 在 `mcp-server` crate 中新增 `trace.rs` 模块，通过 `TeeRead`/`TeeWrite` 包装 stdin/stdout（字节级 tee），将复制出的字节经 std mpsc 发送到一个独立的 std writer 线程，该线程按行缓冲、解析 JSON、pretty-print 后写入 `$LOOM_HOME/log/loom-mcp-trace.log`。rmcp 的 `serve()` 泛型接受任意 `(AsyncRead, AsyncWrite)` 元组，无需 fork rmcp。

**Tech Stack:** Rust 2021, tokio (`AsyncRead`/`AsyncWrite`), `serde_json` (pretty-print), `chrono` (ISO 8601 时间戳), `std::sync::mpsc` (非阻塞 channel), `log` (warn 日志)。

## Global Constraints

- 不引入新的外部依赖（`chrono` 已是 workspace dep，`src/rust/Cargo.toml:37`）
- 代码保持 `rustfmt`-clean（`snake_case` 函数/模块，`UpperCamelCase` 类型）
- 关闭开关时零开销：不创建文件、不 spawn 线程、`append()` 仅一次 `Option` 检查
- tracing 失败绝不阻止 server 启动（降级为 null sink + `warn!`）
- 文档用简体中文（代码标识符/命令/路径/协议术语保持英文）

---

### Task 1: 模块脚手架 + 配置 + `loom_log_dir` 迁移

**Files:**
- Create: `src/rust/mcp-server/trace.rs`
- Modify: `src/rust/mcp-server/Cargo.toml` (加 `chrono.workspace = true`)
- Modify: `src/rust/mcp-server/lib.rs:4` (加 `pub mod trace;`)
- Modify: `src/rust/mcp-server/main.rs:41-53` (删除本地 `loom_log_dir`，改调 `trace::loom_log_dir()`)

**Interfaces:**
- Produces: `pub enum Dir { In, Out }`, `pub struct TraceConfig { enabled: bool, path: Option<PathBuf> }` with `from_env()`, `pub fn loom_log_dir() -> PathBuf`

- [ ] **Step 1: 写失败测试**

在 `trace.rs` 中写 `#[cfg(test)] mod tests`，测试 `TraceConfig::from_env()` 的各种取值：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_disabled_when_unset() {
        std::env::remove_var("LOOM_MCP_TRACE");
        let cfg = TraceConfig::from_env();
        assert!(!cfg.enabled);
        assert!(cfg.path.is_none());
    }

    #[test]
    fn config_disabled_when_zero() {
        std::env::set_var("LOOM_MCP_TRACE", "0");
        let cfg = TraceConfig::from_env();
        assert!(!cfg.enabled);
        std::env::remove_var("LOOM_MCP_TRACE");
    }

    #[test]
    fn config_disabled_when_empty() {
        std::env::set_var("LOOM_MCP_TRACE", "   ");
        let cfg = TraceConfig::from_env();
        assert!(!cfg.enabled);
        std::env::remove_var("LOOM_MCP_TRACE");
    }

    #[test]
    fn config_enabled_default_path_when_one() {
        std::env::set_var("LOOM_MCP_TRACE", "1");
        let cfg = TraceConfig::from_env();
        assert!(cfg.enabled);
        assert!(cfg.path.is_some());
        let path = cfg.path.unwrap();
        assert!(path.to_string_lossy().ends_with("loom-mcp-trace.log"));
        std::env::remove_var("LOOM_MCP_TRACE");
    }

    #[test]
    fn config_enabled_custom_path() {
        std::env::set_var("LOOM_MCP_TRACE", "/tmp/custom-trace.log");
        let cfg = TraceConfig::from_env();
        assert!(cfg.enabled);
        assert_eq!(cfg.path.as_deref(), Some(std::path::Path::new("/tmp/custom-trace.log")));
        std::env::remove_var("LOOM_MCP_TRACE");
    }

    #[test]
    fn loom_log_dir_resolves_loom_home() {
        std::env::set_var("LOOM_HOME", "/tmp/fake-loom-home");
        let dir = loom_log_dir();
        assert_eq!(dir, std::path::PathBuf::from("/tmp/fake-loom-home/log"));
        std::env::remove_var("LOOM_HOME");
    }
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --manifest-path src/rust/Cargo.toml -p mcp-server trace::tests`
Expected: 编译失败（`TraceConfig`、`Dir`、`loom_log_dir` 未定义）

- [ ] **Step 3: 写最小实现**

在 `trace.rs` 中实现 `Dir`、`TraceConfig`、`loom_log_dir()`：

```rust
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Dir {
    In,
    Out,
}

impl Dir {
    fn marker(self) -> &'static str {
        match self {
            Dir::In => ">>",
            Dir::Out => "<<",
        }
    }
}

#[derive(Debug)]
pub struct TraceConfig {
    pub enabled: bool,
    pub path: Option<PathBuf>,
}

impl TraceConfig {
    pub fn from_env() -> Self {
        let raw = std::env::var("LOOM_MCP_TRACE").unwrap_or_default();
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed == "0" {
            return TraceConfig { enabled: false, path: None };
        }
        if trimmed == "1" {
            let path = loom_log_dir().join("loom-mcp-trace.log");
            return TraceConfig { enabled: true, path: Some(path) };
        }
        TraceConfig { enabled: true, path: Some(PathBuf::from(trimmed)) }
    }
}

pub fn loom_log_dir() -> PathBuf {
    let loom_home = std::env::var("LOOM_HOME")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".loom")
        });
    loom_home.join("log")
}
```

在 `lib.rs` 加 `pub mod trace;`。在 `Cargo.toml` 加 `chrono.workspace = true`。在 `main.rs` 把 `loom_log_dir()` 本地函数删除，`init_logging` 中改调 `mcp_server::trace::loom_log_dir()`。

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test --manifest-path src/rust/Cargo.toml -p mcp-server trace::tests`
Expected: PASS（6 个测试全过）

- [ ] **Step 5: 确认整体编译**

Run: `cargo build --manifest-path src/rust/Cargo.toml -p mcp-server`
Expected: PASS

---

### Task 2: `TraceSink` + writer 线程 + 记录输出

**Files:**
- Modify: `src/rust/mcp-server/trace.rs`（追加 `TraceEvent`、`TraceSink`、writer 线程、记录输出辅助函数）

**Interfaces:**
- Consumes: `Dir`、`TraceConfig`（from Task 1）
- Produces: `pub struct TraceSink` with `open(cfg: &TraceConfig) -> TraceSink`、`append(&self, dir: Dir, bytes: &[u8])`、`disabled() -> TraceSink`、`clone()` (derive)

- [ ] **Step 1: 写失败测试**

在 `trace.rs` 的 `tests` 模块追加测试：

```rust
use std::io::Read;

#[test]
fn disabled_sink_creates_no_file() {
    let sink = TraceSink::disabled();
    sink.append(Dir::In, b"hello");
    // 无 panic、无文件
}

#[test]
fn sink_writes_startup_banner_and_record() {
    let tmp = std::env::temp_dir().join(format!("loom-trace-test-{}", std::process::id()));
    let _ = std::fs::remove_file(&tmp);
    let cfg = TraceConfig { enabled: true, path: Some(tmp.clone()) };
    let sink = TraceSink::open(&cfg);
    sink.append(Dir::In, b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"loom.status\"}}\n");
    sink.append(Dir::Out, b"{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n");
    // 等 writer 线程处理完
    drop(sink);
    let mut content = String::new();
    std::fs::File::open(&tmp).unwrap().read_to_string(&mut content).unwrap();
    assert!(content.contains("loom-mcp-server started"), "banner: {}", content);
    assert!(content.contains(">> "), "in marker: {}", content);
    assert!(content.contains("<< "), "out marker: {}", content);
    assert!(content.contains("tools/call id=1"), "method label: {}", content);
    assert!(content.contains("\"jsonrpc\""), "pretty body: {}", content);
}

#[test]
fn sink_reassembles_line_across_chunks() {
    let tmp = std::env::temp_dir().join(format!("loom-trace-chunk-{}", std::process::id()));
    let _ = std::fs::remove_file(&tmp);
    let cfg = TraceConfig { enabled: true, path: Some(tmp.clone()) };
    let sink = TraceSink::open(&cfg);
    sink.append(Dir::In, b"{\"a\":");
    sink.append(Dir::In, b"1}\n{\"b\":2}\n");
    drop(sink);
    let mut content = String::new();
    std::fs::File::open(&tmp).unwrap().read_to_string(&mut content).unwrap();
    assert_eq!(content.matches(">> ").count(), 2, "two records: {}", content);
}

#[test]
fn sink_tags_non_json_line() {
    let tmp = std::env::temp_dir().join(format!("loom-trace-bad-{}", std::process::id()));
    let _ = std::fs::remove_file(&tmp);
    let cfg = TraceConfig { enabled: true, path: Some(tmp.clone()) };
    let sink = TraceSink::open(&cfg);
    sink.append(Dir::In, b"not json\n");
    drop(sink);
    let mut content = String::new();
    std::fs::File::open(&tmp).unwrap().read_to_string(&mut content).unwrap();
    assert!(content.contains("unparseable"), "unparseable tag: {}", content);
}

#[test]
fn sink_flushes_incomplete_on_drop() {
    let tmp = std::env::temp_dir().join(format!("loom-trace-inc-{}", std::process::id()));
    let _ = std::fs::remove_file(&tmp);
    let cfg = TraceConfig { enabled: true, path: Some(tmp.clone()) };
    let sink = TraceSink::open(&cfg);
    sink.append(Dir::In, b"{\"a\":1}\npartial");
    drop(sink);
    let mut content = String::new();
    std::fs::File::open(&tmp).unwrap().read_to_string(&mut content).unwrap();
    assert!(content.contains("incomplete"), "incomplete tag: {}", content);
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --manifest-path src/rust/Cargo.toml -p mcp-server trace::tests`
Expected: 编译失败（`TraceSink`、`TraceEvent` 未定义）

- [ ] **Step 3: 写实现**

在 `trace.rs` 追加 `TraceEvent`、`TraceSink`、writer 线程函数、记录输出辅助函数。关键实现要点：
- `TraceEvent { dir: Dir, bytes: Vec<u8> }`
- `TraceSink { tx: Option<std::sync::mpsc::Sender<TraceEvent>> }`，derive `Clone`
- `open()`: 若 `!cfg.enabled` 返回 `disabled()`；否则打开文件（`create+append`），spawn std 线程，返回持有 sender 的 sink。任何错误 `warn!` 并返回 `disabled()`
- `append()`: `if let Some(tx) = &self.tx { let _ = tx.send(TraceEvent { dir, bytes: bytes.to_vec() }); }`
- writer 线程：持有 `Receiver`、`HashMap<Dir, Vec<u8>>` 两个缓冲、`BufWriter<File>`。循环 `recv()`，追加字节到对应 dir 缓冲，按 `\n` 切分行，每行调用 `emit_record()`。`RecvError` 时 flush 残留缓冲标记 `(incomplete)` 后退出
- `emit_record()`: 解析行为 `serde_json::Value`，推导 method/id 标签，pretty-print，写入 `marker + ts + label + \n + body + \n\n`，flush
- 启动横幅：`=== loom-mcp-server started {ts} pid={pid} trace=enabled ===\n`
- Drop 时 sender 被 drop → receiver 收到 `RecvError` → 线程自然退出并 flush

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test --manifest-path src/rust/Cargo.toml -p mcp-server trace::tests`
Expected: PASS（全部测试通过）

- [ ] **Step 5: 确认整体编译**

Run: `cargo build --manifest-path src/rust/Cargo.toml -p mcp-server`
Expected: PASS

---

### Task 3: `TeeRead` + `TeeWrite`

**Files:**
- Modify: `src/rust/mcp-server/trace.rs`（追加 `TeeRead`、`TeeWrite`）

**Interfaces:**
- Consumes: `TraceSink`、`Dir`（from Task 1-2）
- Produces: `pub struct TeeRead<R> { inner: R, sink: TraceSink, dir: Dir }` with `new()`；`impl<R: AsyncRead + Unpin> AsyncRead for TeeRead<R>`。`pub struct TeeWrite<W>` 同理。

- [ ] **Step 1: 写失败测试**

在 `trace.rs` 的 `tests` 模块追加测试（需要 `#[tokio::test]`）：

```rust
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn tee_read_copies_bytes_without_altering() {
    let input = b"hello world\n";
    let sink = TraceSink::disabled();
    let mut reader = TeeRead::new(std::io::Cursor::new(input), sink, Dir::In);
    let mut buf = [0u8; 32];
    let n = reader.read(&mut buf).await.unwrap();
    assert_eq!(&buf[..n], input);
}

#[tokio::test]
async fn tee_write_copies_bytes_without_altering() {
    let mut output = Vec::new();
    let sink = TraceSink::disabled();
    {
        let mut writer = TeeWrite::new(&mut output, sink, Dir::Out);
        writer.write_all(b"response\n").await.unwrap();
        writer.flush().await.unwrap();
    }
    assert_eq!(&output[..], b"response\n");
}

#[tokio::test]
async fn tee_read_handles_chunked_reads() {
    let input = b"{\"a\":1}\n{\"b\":2}\n";
    let sink = TraceSink::disabled();
    let mut reader = TeeRead::new(std::io::Cursor::new(input), sink, Dir::In);
    let mut buf = [0u8; 5];
    let mut collected = Vec::new();
    loop {
        let n = reader.read(&mut buf).await.unwrap();
        if n == 0 { break; }
        collected.extend_from_slice(&buf[..n]);
    }
    assert_eq!(&collected[..], input);
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --manifest-path src/rust/Cargo.toml -p mcp-server trace::tests`
Expected: 编译失败（`TeeRead`、`TeeWrite` 未定义）

- [ ] **Step 3: 写实现**

在 `trace.rs` 追加。关键实现：
- `TeeRead`：`poll_read` 中先 `Pin::new(&mut self.inner).poll_read(cx, buf)`，返回 `Poll::Ready(Ok(()))` 时取 `buf.filled()` 的字节调 `self.sink.append(self.dir, filled)`，然后返回原始 `Poll`
- `TeeWrite`：`poll_write` 中先 `Pin::new(&mut self.inner).poll_write(cx, buf)`，返回 `Poll::Ready(Ok(n))` 时调 `self.sink.append(self.dir, &buf[..n])`，然后返回 `Ok(n)`。`poll_flush`/`poll_close` 直接透传
- 因为 `R: Unpin` / `W: Unpin`，可以用 `Pin::new(&mut self.inner)` 做手动 pin 投影

```rust
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

pub struct TeeRead<R> {
    inner: R,
    sink: TraceSink,
    dir: Dir,
}

impl<R> TeeRead<R> {
    pub fn new(inner: R, sink: TraceSink, dir: Dir) -> Self {
        Self { inner, sink, dir }
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for TeeRead<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let before = buf.filled().len();
        match Pin::new(&mut this.inner).poll_read(cx, buf) {
            Poll::Ready(Ok(())) => {
                let filled = buf.filled();
                if filled.len() > before {
                    this.sink.append(this.dir, &filled[before..]);
                }
                Poll::Ready(Ok(()))
            }
            other => other,
        }
    }
}

pub struct TeeWrite<W> {
    inner: W,
    sink: TraceSink,
    dir: Dir,
}

impl<W> TeeWrite<W> {
    pub fn new(inner: W, sink: TraceSink, dir: Dir) -> Self {
        Self { inner, sink, dir }
    }
}

impl<W: AsyncWrite + Unpin> AsyncWrite for TeeWrite<W> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        match Pin::new(&mut this.inner).poll_write(cx, buf) {
            Poll::Ready(Ok(n)) => {
                this.sink.append(this.dir, &buf[..n]);
                Poll::Ready(Ok(n))
            }
            other => other,
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
    }
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test --manifest-path src/rust/Cargo.toml -p mcp-server trace::tests`
Expected: PASS

- [ ] **Step 5: 确认整体编译**

Run: `cargo build --manifest-path src/rust/Cargo.toml -p mcp-server`
Expected: PASS

---

### Task 4: 接线 + 集成测试

**Files:**
- Modify: `src/rust/mcp-server/server.rs:933-937`（`run_stdio_server()`）
- Create: `tests/rust/mcp-server/mcp_trace_smoke.rs`
- Modify: `src/rust/mcp-server/Cargo.toml`（加 `[[test]]` 条目）

**Interfaces:**
- Consumes: `TeeRead`、`TeeWrite`、`TraceSink`、`TraceConfig`、`Dir`（from Task 1-3）

- [ ] **Step 1: 写失败集成测试**

创建 `tests/rust/mcp-server/mcp_trace_smoke.rs`，复用 `server_smoke.rs` 的 harness 模式：

```rust
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};
use serde_json::{json, Value};

#[test]
fn trace_file_captures_full_session() {
    let tmp = std::env::temp_dir().join(format!("loom-mcp-trace-smoke-{}", std::process::id()));
    let _ = fs::remove_file(&tmp);

    let mut client = McpProcess::start_with_trace(&tmp);

    let init = client.request(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": { "name": "loom-test", "version": "0.0.1" }
        }
    }));
    assert_eq!(init["result"]["serverInfo"]["name"], "loom-mcp-server");

    client.notify(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));

    let tools = client.request(json!({
        "jsonrpc": "2.0", "id": 2, "method": "tools/list"
    }));
    assert!(tools["result"]["tools"].is_array());

    drop(client);

    let content = fs::read_to_string(&tmp).expect("trace file exists");
    assert!(content.contains("loom-mcp-server started"), "banner missing");
    assert!(content.contains(">> "), "in marker missing");
    assert!(content.contains("<< "), "out marker missing");
    assert!(content.contains("initialize id=1"), "initialize label missing");
    assert!(content.contains("notifications/initialized"), "notification missing");
    assert!(content.contains("tools/list"), "tools/list missing");
    assert!(content.contains("\"jsonrpc\""), "pretty body missing");
}

#[test]
fn trace_disabled_creates_no_file() {
    let tmp = std::env::temp_dir().join(format!("loom-mcp-trace-disabled-{}", std::process::id()));
    let _ = fs::remove_file(&tmp);

    let mut client = McpProcess::start_without_trace();
    let _ = client.request(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": { "name": "loom-test", "version": "0.0.1" }
        }
    }));
    drop(client);

    assert!(!tmp.exists(), "trace file should not exist when disabled");
}

struct McpProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl McpProcess {
    fn start_with_trace(trace_path: &std::path::Path) -> Self {
        Self::start(Some(trace_path))
    }
    fn start_without_trace() -> Self {
        Self::start(None)
    }
    fn start(trace: Option<&std::path::Path>) -> Self {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_loom-mcp-server"));
        if let Some(p) = trace {
            cmd.env("LOOM_MCP_TRACE", p);
        } else {
            cmd.env_remove("LOOM_MCP_TRACE");
        }
        let mut child = cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().expect("spawn");
        let stdin = child.stdin.take().expect("stdin");
        let stdout = BufReader::new(child.stdout.take().expect("stdout"));
        Self { child, stdin, stdout }
    }
    fn request(&mut self, req: Value) -> Value {
        writeln!(self.stdin, "{req}").unwrap();
        self.stdin.flush().unwrap();
        self.read_message()
    }
    fn notify(&mut self, notif: Value) {
        writeln!(self.stdin, "{notif}").unwrap();
        self.stdin.flush().unwrap();
    }
    fn read_message(&mut self) -> Value {
        let mut line = String::new();
        let n = self.stdout.read_line(&mut line).expect("read");
        assert!(n > 0, "server closed stdout");
        serde_json::from_str(line.trim()).expect("valid JSON-RPC")
    }
}

impl Drop for McpProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
```

在 `Cargo.toml` 加 `[[test]]` 条目：
```toml
[[test]]
name = "mcp_trace_smoke"
path = "../../../tests/rust/mcp-server/mcp_trace_smoke.rs"
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --manifest-path src/rust/Cargo.toml -p mcp-server --test mcp_trace_smoke`
Expected: 失败（`run_stdio_server` 还没接线，trace 文件不存在）

- [ ] **Step 3: 接线 `run_stdio_server()`**

修改 `src/rust/mcp-server/server.rs:933-937`：

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

- [ ] **Step 4: 运行集成测试确认通过**

Run: `cargo test --manifest-path src/rust/Cargo.toml -p mcp-server --test mcp_trace_smoke`
Expected: PASS（2 个测试通过）

- [ ] **Step 5: 运行全部 mcp-server 测试确认无回归**

Run: `cargo test --manifest-path src/rust/Cargo.toml -p mcp-server`
Expected: PASS（所有现有测试 + 新测试全过）

---

### Task 5: README 文档

**Files:**
- Modify: `README.md`（英文，在 "How to Use" 末尾加 subsection）
- Modify: `README.zh-CN.md`（中文，在 "如何使用" 末尾加 subsection）

- [ ] **Step 1: 在 `README.zh-CN.md` 的 "如何使用" section 末尾（"运行交付" subsection 之后、"工作方式" 之前）加入**

```markdown
### 调试 MCP 流量

设置 `LOOM_MCP_TRACE=1` 可把每条 MCP 请求和响应（包括 `initialize` 握手和通知）dump 到 `$LOOM_HOME/log/loom-mcp-trace.log`。该文件为人类可读的 pretty-print 格式；复现问题时可用 `tail -f` 实时观看。设置 `LOOM_MCP_TRACE=/some/path.log` 可写到别处。开关默认关闭，关闭时无任何开销。
```

- [ ] **Step 2: 在 `README.md` 的 "How to Use" section 末尾加入对应英文段**

```markdown
### Debug MCP Traffic

Set `LOOM_MCP_TRACE=1` to dump every MCP request and response (including the `initialize` handshake and notifications) to `$LOOM_HOME/log/loom-mcp-trace.log`. The file is human-readable and pretty-printed; use `tail -f` to watch it live while reproducing an issue. Set `LOOM_MCP_TRACE=/some/path.log` to write elsewhere. The toggle is off by default and adds no overhead when disabled.
```

- [ ] **Step 3: 运行 fmt + 全量测试确认无回归**

Run: `cargo fmt --manifest-path src/rust/Cargo.toml --all --check && cargo test --manifest-path src/rust/Cargo.toml -p mcp-server`
Expected: PASS
