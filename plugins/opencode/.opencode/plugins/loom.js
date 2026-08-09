const PENDING_TTL_MS = 30 * 60 * 1000;
const IDLE_PROMPT_COOLDOWN_MS = 15 * 1000;

export const LoomPlugin = async ({ client, directory }) => {
  const pendingBySession = new Map();
  const promptLocks = new Map();

  function remember(sessionID, pending) {
    pendingBySession.set(sessionID, {
      ...pending,
      createdAt: Date.now(),
      lastPromptAt: 0,
    });
  }

  function clear(sessionID) {
    pendingBySession.delete(sessionID);
    promptLocks.delete(sessionID);
  }

  return {
    "command.execute.before": async (input) => {
      if (input.command === "loom" || input.command === "loom-deploy") {
        remember(input.sessionID, {
          projectRoot: directory,
          state: "prompt",
          signature: `prompt:${input.command}:${Date.now()}`,
        });
      }
    },

    "tool.execute.after": async (input, output) => {
      const result = extractActionResult(output.output);
      if (!result || !isLoomActionResult(result)) {
        return;
      }
      const state = String(result.state || "unknown");
      if (state === "auto_runnable") {
        const pending = {
          projectRoot: projectRootFromResult(result, directory),
          state,
          nextKind: result.next?.kind || null,
          requestRef: result.next?.requestRef || result.requestRef || null,
          submitTool: result.next?.submitTool || null,
          writeTargets: result.next?.writeTargets || [],
          signature: signatureForResult(result),
        };
        remember(input.sessionID, pending);
        output.title = "Loom MCP 需要继续执行";
        output.metadata = {
          ...(output.metadata ?? {}),
          loomAutoRunnable: true,
          loomNextKind: pending.nextKind,
          loomRequestRef: pending.requestRef,
        };
        output.output = `${continuationBanner(pending)}\n\n${output.output}`;
        return;
      }
      if (state === "active_operation") {
        remember(input.sessionID, {
          projectRoot: projectRootFromResult(result, directory),
          state,
          nextKind: "observe",
          signature: signatureForResult(result),
        });
        return;
      }
      if (["user_gate", "repairable_error"].includes(state)) {
        remember(input.sessionID, {
          projectRoot: projectRootFromResult(result, directory),
          state,
          nextKind: state,
          signature: signatureForResult(result),
        });
        return;
      }
      clear(input.sessionID);
    },

    event: async ({ event }) => {
      if (event.type !== "session.idle") {
        return;
      }
      const sessionID = event.properties.sessionID;
      const pending = pendingBySession.get(sessionID);
      if (!pending) {
        return;
      }
      if (Date.now() - pending.createdAt > PENDING_TTL_MS) {
        clear(sessionID);
        return;
      }
      if (!["auto_runnable", "active_operation"].includes(pending.state)) {
        return;
      }
      const lock = promptLocks.get(sessionID);
      if (lock && Date.now() - lock < IDLE_PROMPT_COOLDOWN_MS) {
        return;
      }
      promptLocks.set(sessionID, Date.now());
      await client.session.promptAsync({
        path: { id: sessionID },
        query: { directory: pending.projectRoot || directory },
        body: {
          agent: "build",
          system: "继续执行当前的 Loom MCP 工作流。在最近一次结果为 auto_runnable 或 active 时，不要总结、标记本地计划完成、发送最终答复，或询问是否继续。",
          parts: [
            {
              type: "text",
              metadata: { loomAutoContinue: true, signature: pending.signature },
              text: idlePrompt(pending),
            },
          ],
        },
      });
    },
  };
};

function extractActionResult(text) {
  if (typeof text !== "string") {
    return null;
  }
  const first = text.indexOf("{");
  const last = text.lastIndexOf("}");
  if (first < 0 || last <= first) {
    return null;
  }
  try {
    const parsed = JSON.parse(text.slice(first, last + 1));
    if (typeof parsed.state === "string") {
      return parsed;
    }
    if (typeof parsed.structuredContent?.state === "string") {
      return parsed.structuredContent;
    }
    if (typeof parsed.content?.[0]?.text === "string") {
      return extractActionResult(parsed.content[0].text);
    }
  } catch {
    return null;
  }
  return null;
}

function isLoomActionResult(result) {
  return typeof result === "object" && typeof result.state === "string";
}

function projectRootFromResult(result, fallback) {
  return result.projectRoot || result.next?.projectRoot || fallback;
}

function signatureForResult(result) {
  return [
    result.state,
    result.next?.kind || "",
    result.next?.requestRef || "",
    result.next?.submitTool || "",
  ].join(":");
}

function continuationBanner(pending) {
  return [
    "Loom MCP 返回了一个 auto-runnable 的后续操作。",
    `后续操作类型：${pending.nextKind || "未知"}。`,
    pending.requestRef ? `请求：${pending.requestRef}。` : null,
    "在报告完成之前，先执行返回的 MCP 操作。",
  ].filter(Boolean).join("\n");
}

function idlePrompt(pending) {
  if (pending.state === "active_operation") {
    return "最近的 Loom MCP 结果为 active_operation。仅调用该结果指定的观察工具，然后继续遵循返回的状态。";
  }
  if (pending.nextKind === "run_loom_tool") {
    return [
      "最近的 Loom MCP 结果为 auto_runnable。",
      "立即执行 next.kind=run_loom_tool。",
      pending.requestRef ? `使用 requestRef=${pending.requestRef}。` : null,
      "检查请求，仅读取返回的 readGroups，调用返回的 Loom MCP 工具，然后在报告进度之前重试返回的 retryTool。",
    ].filter(Boolean).join("\n");
  }
  return [
    "最近的 Loom MCP 结果为 auto_runnable。",
    `立即执行 next.kind=${pending.nextKind || "未知"}。`,
    pending.requestRef ? `使用 requestRef=${pending.requestRef}。` : null,
    "通过 Loom MCP 读取工具读取已声明的请求字段组，仅写入返回的目标，并在报告完成之前通过返回的 MCP 提交工具进行提交。",
  ].filter(Boolean).join(" ");
}
