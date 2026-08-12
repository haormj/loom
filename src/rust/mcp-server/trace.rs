use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::mpsc;
use std::task::{Context, Poll};

use chrono::Utc;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

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
            return TraceConfig {
                enabled: false,
                path: None,
            };
        }
        if trimmed == "1" {
            let path = loom_log_dir().join("loom-mcp-trace.log");
            return TraceConfig {
                enabled: true,
                path: Some(path),
            };
        }
        TraceConfig {
            enabled: true,
            path: Some(PathBuf::from(trimmed)),
        }
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

struct TraceEvent {
    dir: Dir,
    bytes: Vec<u8>,
}

pub struct TraceSink {
    tx: Option<mpsc::Sender<TraceEvent>>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Clone for TraceSink {
    fn clone(&self) -> Self {
        TraceSink {
            tx: self.tx.clone(),
            handle: None,
        }
    }
}

impl TraceSink {
    pub fn open(cfg: &TraceConfig) -> Self {
        if !cfg.enabled {
            return Self::disabled();
        }
        let path = match &cfg.path {
            Some(p) => p,
            None => return Self::disabled(),
        };

        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                log::warn!("LOOM_MCP_TRACE: cannot create dir {:?}: {}", parent, e);
                return Self::disabled();
            }
        }

        let file = match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            Ok(f) => f,
            Err(e) => {
                log::warn!("LOOM_MCP_TRACE: cannot open {:?}: {}", path, e);
                return Self::disabled();
            }
        };

        let mut writer = BufWriter::new(file);
        let ts = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        let pid = std::process::id();
        let _ = writeln!(
            writer,
            "=== loom-mcp-server started {} pid={} trace=enabled ===",
            ts, pid
        );
        let _ = writer.flush();

        let (tx, rx) = mpsc::channel::<TraceEvent>();
        let handle = std::thread::spawn(move || {
            writer_loop(rx, writer);
        });

        TraceSink {
            tx: Some(tx),
            handle: Some(handle),
        }
    }

    pub fn disabled() -> Self {
        TraceSink {
            tx: None,
            handle: None,
        }
    }

    pub fn append(&self, dir: Dir, bytes: &[u8]) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(TraceEvent {
                dir,
                bytes: bytes.to_vec(),
            });
        }
    }
}

impl Drop for TraceSink {
    fn drop(&mut self) {
        self.tx.take();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn writer_loop(rx: mpsc::Receiver<TraceEvent>, mut writer: BufWriter<std::fs::File>) {
    let mut buf_in: Vec<u8> = Vec::new();
    let mut buf_out: Vec<u8> = Vec::new();

    loop {
        match rx.recv() {
            Ok(event) => {
                let buf = match event.dir {
                    Dir::In => &mut buf_in,
                    Dir::Out => &mut buf_out,
                };
                buf.extend_from_slice(&event.bytes);
                drain_lines(buf, event.dir, &mut writer);
            }
            Err(_) => {
                if !buf_in.is_empty() {
                    emit_record(&buf_in, Dir::In, &mut writer, true);
                }
                if !buf_out.is_empty() {
                    emit_record(&buf_out, Dir::Out, &mut writer, true);
                }
                let _ = writer.flush();
                return;
            }
        }
    }
}

fn drain_lines(buf: &mut Vec<u8>, dir: Dir, writer: &mut BufWriter<std::fs::File>) {
    while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
        let line: Vec<u8> = buf.drain(..=pos).collect();
        let mut line = line.as_slice();
        line = line.strip_suffix(b"\n").unwrap_or(line);
        line = line.strip_suffix(b"\r").unwrap_or(line);
        if line.is_empty() {
            continue;
        }
        emit_record(line, dir, writer, false);
    }
}

fn emit_record(line: &[u8], dir: Dir, writer: &mut BufWriter<std::fs::File>, incomplete: bool) {
    let ts = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    let marker = dir.marker();

    if incomplete {
        let _ = writeln!(
            writer,
            "{} {} (incomplete, {} bytes)",
            marker,
            ts,
            line.len()
        );
        let _ = writeln!(writer, "{}", String::from_utf8_lossy(line));
        let _ = writeln!(writer);
        let _ = writer.flush();
        return;
    }

    match serde_json::from_slice::<serde_json::Value>(line) {
        Ok(value) => {
            let label = build_label(&value);
            if label.is_empty() {
                let _ = writeln!(writer, "{} {}", marker, ts);
            } else {
                let _ = writeln!(writer, "{} {} {}", marker, ts, label);
            }
            let body = serde_json::to_string_pretty(&value).unwrap_or_default();
            let _ = writeln!(writer, "{}", body);
            let _ = writeln!(writer);
        }
        Err(_) => {
            let _ = writeln!(
                writer,
                "{} {} (unparseable, {} bytes)",
                marker,
                ts,
                line.len()
            );
            let _ = writeln!(writer, "{}", String::from_utf8_lossy(line));
            let _ = writeln!(writer);
        }
    }
    let _ = writer.flush();
}

fn build_label(value: &serde_json::Value) -> String {
    let method = value.get("method").and_then(|v| v.as_str());
    let id = value.get("id");

    match (method, id) {
        (Some(m), Some(id_val)) => format!("{} {}", m, format_id(id_val)),
        (Some(m), None) => m.to_string(),
        (None, Some(id_val)) => format_id(id_val),
        (None, None) => String::new(),
    }
}

fn format_id(id: &serde_json::Value) -> String {
    match id {
        serde_json::Value::Number(n) => format!("id={}", n),
        serde_json::Value::String(s) => format!("id={}", s),
        serde_json::Value::Null => "id=null".to_string(),
        other => format!("id={}", other),
    }
}

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
    ) -> Poll<std::io::Result<()>> {
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
    ) -> Poll<std::io::Result<usize>> {
        let this = self.get_mut();
        match Pin::new(&mut this.inner).poll_write(cx, buf) {
            Poll::Ready(Ok(n)) => {
                this.sink.append(this.dir, &buf[..n]);
                Poll::Ready(Ok(n))
            }
            other => other,
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static TRACE_ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn config_disabled_when_unset() {
        let _guard = TRACE_ENV_LOCK.lock().unwrap();
        std::env::remove_var("LOOM_MCP_TRACE");
        let cfg = TraceConfig::from_env();
        assert!(!cfg.enabled);
        assert!(cfg.path.is_none());
    }

    #[test]
    fn config_disabled_when_zero() {
        let _guard = TRACE_ENV_LOCK.lock().unwrap();
        std::env::set_var("LOOM_MCP_TRACE", "0");
        let cfg = TraceConfig::from_env();
        assert!(!cfg.enabled);
        std::env::remove_var("LOOM_MCP_TRACE");
    }

    #[test]
    fn config_disabled_when_empty() {
        let _guard = TRACE_ENV_LOCK.lock().unwrap();
        std::env::set_var("LOOM_MCP_TRACE", "   ");
        let cfg = TraceConfig::from_env();
        assert!(!cfg.enabled);
        std::env::remove_var("LOOM_MCP_TRACE");
    }

    #[test]
    fn config_enabled_default_path_when_one() {
        let _guard = TRACE_ENV_LOCK.lock().unwrap();
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
        let _guard = TRACE_ENV_LOCK.lock().unwrap();
        std::env::set_var("LOOM_MCP_TRACE", "/tmp/custom-trace.log");
        let cfg = TraceConfig::from_env();
        assert!(cfg.enabled);
        assert_eq!(
            cfg.path.as_deref(),
            Some(std::path::Path::new("/tmp/custom-trace.log"))
        );
        std::env::remove_var("LOOM_MCP_TRACE");
    }

    #[test]
    fn loom_log_dir_resolves_loom_home() {
        std::env::set_var("LOOM_HOME", "/tmp/fake-loom-home");
        let dir = loom_log_dir();
        assert_eq!(dir, std::path::PathBuf::from("/tmp/fake-loom-home/log"));
        std::env::remove_var("LOOM_HOME");
    }

    #[test]
    fn disabled_sink_creates_no_file() {
        let sink = TraceSink::disabled();
        sink.append(Dir::In, b"hello");
    }

    #[test]
    fn sink_writes_startup_banner_and_record() {
        let tmp = std::env::temp_dir().join(format!("loom-trace-test-{}", std::process::id()));
        let _ = std::fs::remove_file(&tmp);
        let cfg = TraceConfig {
            enabled: true,
            path: Some(tmp.clone()),
        };
        let sink = TraceSink::open(&cfg);
        sink.append(
            Dir::In,
            b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"loom.status\"}}\n",
        );
        sink.append(Dir::Out, b"{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n");
        drop(sink);
        let content = std::fs::read_to_string(&tmp).unwrap();
        assert!(
            content.contains("loom-mcp-server started"),
            "banner: {}",
            content
        );
        assert!(content.contains(">> "), "in marker: {}", content);
        assert!(content.contains("<< "), "out marker: {}", content);
        assert!(
            content.contains("tools/call id=1"),
            "method label: {}",
            content
        );
        assert!(content.contains("\"jsonrpc\""), "pretty body: {}", content);
    }

    #[test]
    fn sink_reassembles_line_across_chunks() {
        let tmp = std::env::temp_dir().join(format!("loom-trace-chunk-{}", std::process::id()));
        let _ = std::fs::remove_file(&tmp);
        let cfg = TraceConfig {
            enabled: true,
            path: Some(tmp.clone()),
        };
        let sink = TraceSink::open(&cfg);
        sink.append(Dir::In, b"{\"a\":");
        sink.append(Dir::In, b"1}\n{\"b\":2}\n");
        drop(sink);
        let content = std::fs::read_to_string(&tmp).unwrap();
        assert_eq!(
            content.matches(">> ").count(),
            2,
            "two records: {}",
            content
        );
    }

    #[test]
    fn sink_tags_non_json_line() {
        let tmp = std::env::temp_dir().join(format!("loom-trace-bad-{}", std::process::id()));
        let _ = std::fs::remove_file(&tmp);
        let cfg = TraceConfig {
            enabled: true,
            path: Some(tmp.clone()),
        };
        let sink = TraceSink::open(&cfg);
        sink.append(Dir::In, b"not json\n");
        drop(sink);
        let content = std::fs::read_to_string(&tmp).unwrap();
        assert!(
            content.contains("unparseable"),
            "unparseable tag: {}",
            content
        );
    }

    #[test]
    fn sink_flushes_incomplete_on_drop() {
        let tmp = std::env::temp_dir().join(format!("loom-trace-inc-{}", std::process::id()));
        let _ = std::fs::remove_file(&tmp);
        let cfg = TraceConfig {
            enabled: true,
            path: Some(tmp.clone()),
        };
        let sink = TraceSink::open(&cfg);
        sink.append(Dir::In, b"{\"a\":1}\npartial");
        drop(sink);
        let content = std::fs::read_to_string(&tmp).unwrap();
        assert!(
            content.contains("incomplete"),
            "incomplete tag: {}",
            content
        );
    }

    #[tokio::test]
    async fn tee_read_copies_bytes_without_altering() {
        let input = b"hello world\n";
        let sink = TraceSink::disabled();
        let mut reader = TeeRead::new(std::io::Cursor::new(input), sink, Dir::In);
        let mut buf = [0u8; 32];
        let n = tokio::io::AsyncReadExt::read(&mut reader, &mut buf)
            .await
            .unwrap();
        assert_eq!(&buf[..n], input);
    }

    #[tokio::test]
    async fn tee_write_copies_bytes_without_altering() {
        let mut output = Vec::new();
        let sink = TraceSink::disabled();
        {
            let mut writer = TeeWrite::new(&mut output, sink, Dir::Out);
            tokio::io::AsyncWriteExt::write_all(&mut writer, b"response\n")
                .await
                .unwrap();
            tokio::io::AsyncWriteExt::flush(&mut writer).await.unwrap();
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
            let n = tokio::io::AsyncReadExt::read(&mut reader, &mut buf)
                .await
                .unwrap();
            if n == 0 {
                break;
            }
            collected.extend_from_slice(&buf[..n]);
        }
        assert_eq!(&collected[..], input);
    }
}
