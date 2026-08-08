# 部署提供者参考

当扩展或修复 loom deploy 提供者选择时，使用本参考文档。

## 当前策略

loom v1 仅使用 Dockerfile 和 Docker Compose。它不调用 Railpack、Buildpacks、Nixpacks 或其他外部构建器。

提供者顺序：

1. 复用根级 Compose 文件而不覆盖它们。
2. 复用根级 Dockerfile 并仅生成本地 Compose 包装。
3. 为已知或未知技术栈生成确定性 Dockerfile/Compose 文件。
4. 当构建、启动、日志或健康验证失败时返回有界的 MCP 修复操作。

`loom.deployRun` 是常规 agent 使用的首选高级 MCP 工具。它组合了 prepare、build/start、validate、status 和修复操作报告，而不隐藏提供者选择或切换构建器。

`.loom/deployment/specs/local.json` 中的 `providerCandidates` 应描述已选择、可用或跳过的 Compose/Dockerfile 提供者，以及验证/构建它们的命令。

## 提供者策略

提供者策略通过 `DeployToolInput.providerPolicy` 为策略选择提供显式用户控制：

- `provider: "compose-existing"`：要求根级 Compose 文件。
- `provider: "dockerfile-existing"`：要求根级 Dockerfile 并仅生成 Compose 包装。
- `provider: "generated"`：生成 Loom Dockerfile/Compose 资产而非选择现有提供者。
- `forceGenerate: true`：强制生成 Dockerfile/Compose 资产并跳过现有用户资产。
- `reuseExisting: false`：禁用现有 Dockerfile/Compose 复用，同时保持正常模板生成。

如果显式选择的现有提供者没有匹配文件，返回 `INVALID_ARGUMENT` 并附明确原因。不要静默回退到另一个提供者。

提供者候选应解释策略跳过，以便修复/检查输出能告知提供者是不可用还是被有意绕过。

## 回退策略

当用户未强制提供者时，首先尝试现有资产。如果未强制的现有 Compose 或 Dockerfile 提供者因受保护资产形态甚至无法 build/start，Loom 可以回退到生成的提供者而非要求用户编辑其文件。此回退必须：

- 保持用户拥有的 Compose 文件或 Dockerfile 不变
- 将 `providerPolicy` 切换为 generated 并禁用现有复用以进行重试
- 在新的 DeploymentSpec 中记录选定的生成提供者
- 将受保护的用户资产排除在 `editableFiles` 之外

当用户显式选择了 `compose-existing` 或 `dockerfile-existing` 时，不允许回退。返回解释现有资产不匹配和可用用户选择的修复/阻塞项。

## 提供者规则

- 现有 Compose 受保护，在 `loom.deployPrepare` 期间从不覆盖。
- 现有 Dockerfile 受保护，使用生成的 Compose 包装复用。
- 生成的文件位于 `.loom/deployment/specs/generated/` 下。
- 未知项目仍然接收确定性占位 Dockerfile，以便编码 agent 可以检查、修复或解释阻塞项。
- 生成 fallback 是一种提供者策略，而非修复循环。修复应在已知选定提供者后修复生成资产或应用/runtime 问题。

## 护栏

- 当用户显式强制提供者时，失败后不要自动切换提供者。
- 除非产品明确将其添加为未来提供者家族，否则不要引入外部构建器。
- 未经用户明确批准，不要覆盖现有 `Dockerfile` 或 Compose 文件。
- 失败应为编码 agent 产生清晰的 MCP 修复操作，而非隐藏重试策略链。
