use std::{
    fs,
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

use serde_json::{json, Value};

#[test]
fn trace_file_captures_full_session() {
    let tmp = std::env::temp_dir().join(format!(
        "loom-mcp-trace-smoke-{}",
        std::process::id()
    ));
    let _ = fs::remove_file(&tmp);

    let mut client = McpProcess::start_with_trace(&tmp);

    let init = client.request(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": { "name": "loom-test", "version": "0.0.1" }
        }
    }));
    assert_eq!(init["result"]["serverInfo"]["name"], "loom-mcp-server");

    client.notify(json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    }));

    let tools = client.request(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list"
    }));
    assert!(tools["result"]["tools"].is_array());

    drop(client);

    let content = fs::read_to_string(&tmp).expect("trace file exists");
    assert!(content.contains("loom-mcp-server started"), "banner missing");
    assert!(content.contains(">> "), "in marker missing");
    assert!(content.contains("<< "), "out marker missing");
    assert!(
        content.contains("initialize id=1"),
        "initialize label missing"
    );
    assert!(
        content.contains("notifications/initialized"),
        "notification missing"
    );
    assert!(content.contains("tools/list"), "tools/list missing");
    assert!(content.contains("\"jsonrpc\""), "pretty body missing");
}

#[test]
fn trace_disabled_creates_no_file() {
    let tmp = std::env::temp_dir().join(format!(
        "loom-mcp-trace-disabled-{}",
        std::process::id()
    ));
    let _ = fs::remove_file(&tmp);

    let mut client = McpProcess::start_without_trace();
    let _ = client.request(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": { "name": "loom-test", "version": "0.0.1" }
        }
    }));
    drop(client);

    assert!(
        !tmp.exists(),
        "trace file should not exist when disabled"
    );
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
        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn loom-mcp-server");
        let stdin = child.stdin.take().expect("child stdin");
        let stdout = BufReader::new(child.stdout.take().expect("child stdout"));
        Self {
            child,
            stdin,
            stdout,
        }
    }

    fn request(&mut self, req: Value) -> Value {
        writeln!(self.stdin, "{req}").expect("write message");
        self.stdin.flush().expect("flush message");
        self.read_message()
    }

    fn notify(&mut self, notif: Value) {
        writeln!(self.stdin, "{notif}").expect("write message");
        self.stdin.flush().expect("flush message");
    }

    fn read_message(&mut self) -> Value {
        let mut line = String::new();
        let n = self.stdout.read_line(&mut line).expect("read message");
        assert!(n > 0, "server closed stdout before response");
        serde_json::from_str(line.trim()).expect("valid JSON-RPC response")
    }
}

impl Drop for McpProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
