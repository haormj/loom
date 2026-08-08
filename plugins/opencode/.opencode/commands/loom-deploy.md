---
description: 通过 MCP 路由 Loom 部署命令。
argument-hint: "[prepare|up|status|inspect|validate|logs|bootstrap|down|repair]"
---

# loom-deploy

为当前项目目录调用对应的 Loom MCP 部署工具。

- 空 -> `loom.deployRun`
- `prepare` -> `loom.deployPrepare`
- `up` -> `loom.deployUp`
- `status` -> `loom.deployStatus`
- `inspect` -> `loom.deployInspect`
- `validate` -> `loom.deployValidate`
- `logs` -> `loom.deployLogs`
- `bootstrap` -> `loom.deployBootstrap`
- `down` -> `loom.deployDown`
- `repair` -> `loom.deployRepair`

遵循返回的操作结果。对于带有 `preResponseContract` 的 `user_gate`，在任何可见响应之前执行约定：调用 `loom.inspectRequest`，然后仅对必需的 `requestReadPlan.groups` 调用 `loom.readFieldGroup`，并展示返回的部署决策。不要在该 MCP 结果之外编造部署资产、拓扑、修复范围、预览 URL 或端口。

在 `active_operation` 期间，仅调用结果指定的观察工具，遵守 `observationPolicy`，遵守 `forbiddenActions`，并在 `finalResponsePolicy` 禁止时不要报告完成。

对于 `DeployRepairAssetsNext`，仅编辑返回的已生成部署资产文件或返回的 `modelRepairRef`，并通过返回的 `retryTool` 重试；切勿直接编辑生成的 source-model/topology/facts 快照。不要使用 `loom.deployRun` 作为修复重试。对于部署执行修复，仅编辑允许的应用程序/运行时文件并通过返回的修复提交工具提交。

当部署修复或部署执行修复结果包含 `requestRef` 时，使用 `loom.inspectRequest` 和 `loom.readFieldGroup`。`requestReadPlan.groups` 是唯一的读取约定。仅读取当前修复操作所需的字段组。

## 引用加载

当前 MCP 部署结果仍然是权威。可选部署引用安装在 `../references/loom-deploy/` 下；默认不加载。仅当 MCP 结果提供确切的引用加载计划时才加载部署引用。

协议：
- 在部署 MCP 结果之后，查找 `next.deployReferenceProfile.referenceLoadPlan` 或 `details.deployReferenceProfile.referenceLoadPlan`。
- 每个 `referenceLoadPlan` 条目包含 `refId`、`path` 和 `reason`。相对于 `../references/loom-deploy/` 解析 `path`。
- 仅读取列出的路径。不要从 `refId`、栈名称、失败文本或 `../references/loom-deploy` 目录推断额外文件。
- 将引用用作已生成或已修复部署文件的实现指南；不要将引用正文粘贴到部署产物、修复结果或最终聊天输出中。
- 如果当前部署操作没有 `deployReferenceProfile`，则保持部署引用未读。

不要将部署栈规则、修复约定、运行时家族规则或 TaskResult 约定复制到此命令中。它们属于当前 MCP 部署结果或修复请求。
