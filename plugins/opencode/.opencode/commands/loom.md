---
description: 通过 MCP 路由 Loom 交付、知识和部署命令。
argument-hint: "<request> | plan <request> | continue | knowledge [subcommand] | deploy [subcommand] | status"
---

# loom

你正在执行 `/loom $ARGUMENTS`。

在进行任何其他工作之前，先为当前项目目录调用对应的 Loom MCP 工具。

- `status` -> `loom.status`
- `continue`、`resume`、`proceed`、`next` 或空 -> `loom.continue`
- `knowledge ...` -> 对应的 `loom.knowledge*` 工具
- `deploy` -> `loom.deployRun`
- `deploy ...` -> 对应的 `loom.deploy*` 工具
- `plan <request>` -> `loom.plan`，传入 `<request>`
- 任何其他请求文本 -> `loom.plan`，传入完整请求文本

工具返回后，遵循 `LoomMcpActionResult.state`：将 `auto_runnable` 视为必需的继续检查点并立即继续；在 `stopAllowed=false` 时不要报告进度、标记本地计划完成、发送最终答复或停止。如果 shell、补丁、测试或嵌套 MCP 调用失败，检查确切的失败原因并在同一轮次中重试最小的纠正步骤。当 MCP 通过包装器调用时，解析嵌套的结构化结果及其 `state`；包装器的状态不是 Loom 工作流状态。对于带有 `preResponseContract` 的 `user_gate`，在发出任何用户可见的响应之前按顺序执行其步骤：调用 `loom.inspectRequest`，仅通过 `loom.readFieldGroup` 读取 `requestReadPlan.groups` 中 `whenToRead` 在可见响应之前适用的字段组，并在形成选项或确认之前运行每个 Brainstorm `knowledge_context_plan` 步骤。安排在用户确认之后的字段组在确认/提交调用之前仍然是必需的。不要仅凭 `prompt` 回答、跳到通用选项，或调用 `/loom continue` 来绕过约定。阶段延续的 Brainstorm 网关是一个主动澄清轮次，而不是可选的 `/loom continue`；不要停在进度回顾处或说"是否要继续"。完成读取和知识调用，然后提出可见的当前块问题并等待用户的回答。对于没有 `preResponseContract` 的网关，展示返回的提示并等待被接受的响应。对于 `repairable_error`，`stopAllowed=false`：调用 `loom.inspectRequest`，读取 `requestReadPlan.groups` 中的每个必需修复字段组，然后仅修复返回的目标，遵循返回的 `agentInstruction`，并调用返回的重新提交工具；仅在 `done`、`blocked` 或 `failed` 时停止。

## 结果纪律

在 `state=auto_runnable` 或 `stopAllowed=false` 时不要停在回顾处。在最近的 Loom 结果为 auto-runnable 时不要标记本地计划完成、发送最终答复或询问是否继续。任务执行仅在请求的结果产物已写入且其 MCP 提交工具成功后才算完成。

对于 `active_operation`，仅调用结果指定的观察工具。对于 `repairable_error`，`stopAllowed=false`：先检查返回的请求并读取每个必需的修复字段组，然后仅修复返回的文件或目标 ID，遵循 `agentInstruction`，并调用返回的重新提交工具。

## 请求读取

当结果包含 `requestRef` 时，使用 `loom.inspectRequest` 和 `loom.readFieldGroup`。`requestReadPlan.groups` 是唯一的读取约定。不要搜索 `.loom`，不要使用临时 JSON 选择器，也不要从旧产物推断请求结构或提交参数。

仅读取当前操作所需的字段组。不要请求单个字段路径；`loom.readFieldGroup` 是请求读取 API。

## 编写与提交

仅将产物写入返回的 `writeTargets`。仅通过返回的 MCP 提交工具使用 `{ projectRoot, requestRef, writtenTargetIds? }` 进行提交。

对于 `GenerateKnowledgeSemanticsNext`，仅通过 `loom.knowledgeInspectChunk` 读取分块正文，填充提供的结果模板，并使用 `loom.knowledgeSemanticSubmitFile` 提交。逐包继续，直到结果发布、被阻止或进入用户网关。

对于 `ExecuteTaskNext`，检查 `next.requestRef`，读取已声明的字段组，仅实现返回的任务请求，遵守编辑边界，编写 TaskResult，并在报告进度为完成之前提交它。

对于 `RunLoomToolNext`，检查 requestRef，仅读取返回的 readGroups，调用返回的 Loom MCP 工具，然后在报告进度之前重试返回的 retryTool。

对于 `DeployRepairAssetsNext`，仅编辑返回的部署资产文件并通过返回的部署工具重试。对于部署执行修复，仅编辑允许的应用程序/运行时文件并通过返回的修复提交工具提交。

## 引用加载

当前 MCP 请求/结果仍然是权威。可选引用安装在 `../references/loom/` 下。默认不加载任何引用；仅当当前操作选择了引用配置文件时才加载引用。

协议：
- 读取当前请求字段组后，仅从该请求选择的配置文件中选择引用。
- 仅从当前 MCP 请求/结果中的 `referenceLoadPlan` 数组读取引用文件。
- 将任何选定的字段组字段视为范围和证据的语义标签，而不是路径映射。
- 如果引用文件未被 MCP 约定选定且当前操作不需要，则保持未读。
- 在质量自检中，报告所选加载计划中确切的 `referencePlanFilesChecked` 路径；不要粘贴引用正文或模板内容。

引用配置文件：
- 每个 `referenceLoadPlan` 条目包含 `refId`、`path` 和 `reason`。相对于此命令/插件位置解析 `path`（相对于 `../references/loom/`），而不是相对于项目工作区。
- 仅加载当前操作列出的路径。不要从字段组名称派生路径、扫描引用目录或加载外部语言/API/架构/UI 技能。
- 将令牌模板路径视为项目文件的合并基线，而不是要复制到 Loom 产物中的文本。

引用纪律：
- 不要加载未选定的引用来弥补实现规划的不足；仅当选定的 `referenceLoadPlan` 不足以完成任务时才请求 Loom 修复约定。
- 在没有选定加载计划的 TaskPlan、Execution、Review 和 Repair 请求中，使用提供的质量引用、需求、证据和审查信号，而不读取原始引用。
- 不要将技术引用文本粘贴到 Architecture、TaskPlan、TaskResult、ReviewResult、源文件或面向用户的 UI 中。使用引用来产生具体的决策、接口约定、NFR、风险和证据。

交付规划、设计、审查、修复和交接规则由当前 MCP 请求/结果提供。不要加载单独的交付引用文件。

## 边界

不要将字段级约定、知识语义模板、Brainstorm 块约定、部署栈规则、架构章节约定或 TaskResult 约定复制到此命令中。它们属于当前 MCP 请求/结果。

保持用户可见输出简洁。除非用户明确要求检查，否则不要粘贴生成的 JSON 产物、完整请求负载、完整结果文件或大型日志。
