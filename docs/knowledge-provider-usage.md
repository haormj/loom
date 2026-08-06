# OpenViking 知识库接入使用指南

## 概述

loom 知识模块支持接入外部 OpenViking 上下文数据库作为知识源。接入后，`knowledgeSearch`、`knowledgeInspectChunk` 等 MCP 工具会自动适配，无需额外配置工具。

## 前置条件

1. 已部署 OpenViking 服务器（默认端口 1933）
2. 已知 OpenViking 的 API Key（如需认证）
3. loom 已安装并可用

## 配置步骤

### 第一步：创建 providers.yaml 配置文件

在 `~/.loom/knowledge/` 目录下创建 `providers.yaml` 文件：

```bash
mkdir -p ~/.loom/knowledge
```

文件内容示例：

```yaml
sources:
  - name: confluence-kb
    endpoint: http://confluence-openviking.internal:1933
    apiKeyEnv: CONFLUENCE_OV_KEY
    account: acme
    user: alice
    targetUri: viking://resources/confluence/
    timeoutSecs: 10

  - name: internal-wiki
    endpoint: http://wiki-openviking.internal:1933
    apiKeyEnv: WIKI_OV_KEY
    account: acme
    targetUri: viking://resources/wiki/
    timeoutSecs: 15
```

### 第二步：设置 API Key 环境变量

`apiKeyEnv` 字段指定环境变量名称，loom 运行时从中读取实际的 API Key：

```bash
# 在 shell 配置文件（~/.bashrc / ~/.zshrc）中添加
export CONFLUENCE_OV_KEY="your-openviking-api-key-here"
export WIKI_OV_KEY="another-api-key"
```

> **注意**：API Key 只存环境变量，不写入配置文件，避免泄露。

### 配置字段说明

| 字段 | 必填 | 默认值 | 说明 |
|------|------|--------|------|
| `name` | 是 | — | 知识源名称（2-80 字符，仅含字母、数字、`.`、`_`、`-`） |
| `endpoint` | 是 | — | OpenViking 服务地址（含协议和端口） |
| `apiKeyEnv` | 否 | 无 | API Key 所在环境变量名 |
| `account` | 否 | 无 | 多租户账户名，映射为 `X-OpenViking-Account` 头 |
| `user` | 否 | 无 | 用户标识，映射为 `X-OpenViking-User` 头 |
| `targetUri` | 否 | `viking://resources/` | 搜索目标 URI，指定 OpenViking 中的资源范围 |
| `timeoutSecs` | 否 | `10` | HTTP 请求超时秒数 |
| `enabled` | 否 | `true` | 是否启用 |

## 使用方式

配置完成后，现有的 MCP 工具自动支持 OpenViking 知识源，无需额外操作。

### 搜索知识

```json
{
  "tool": "loom.knowledgeSearch",
  "arguments": {
    "projectRoot": "/path/to/project",
    "naturalLanguageQuery": "如何配置用户认证",
    "sourceNames": ["confluence-kb"]
  }
}
```

不指定 `sourceNames` 时会搜索所有已启用的知识源（本地 + OpenViking）。

### 查看知识块详情

```json
{
  "tool": "loom.knowledgeInspectChunk",
  "arguments": {
    "projectRoot": "/path/to/project",
    "sourceName": "confluence-kb",
    "buildId": "openviking",
    "chunkId": "viking://resources/confluence/auth-guide.md"
  }
}
```

> OpenViking 知识源的 `buildId` 固定为 `"openviking"`，`chunkId` 为 OpenViking 返回的 viking:// URI。

### 列出所有知识源

```json
{
  "tool": "loom.knowledgeList",
  "arguments": {
    "projectRoot": "/path/to/project"
  }
}
```

返回结果中同时包含本地知识源和 OpenViking 知识源。

### 查看知识源状态

```json
{
  "tool": "loom.knowledgeStatus",
  "arguments": {
    "projectRoot": "/path/to/project",
    "name": "confluence-kb"
  }
}
```

### 启用/禁用知识源

```json
{
  "tool": "loom.knowledgeDisable",
  "arguments": {
    "projectRoot": "/path/to/project",
    "name": "confluence-kb"
  }
}
```

### 移除知识源

```json
{
  "tool": "loom.knowledgeRemove",
  "arguments": {
    "projectRoot": "/path/to/project",
    "name": "confluence-kb"
  }
}
```

> 移除 OpenViking 知识源只会从 registry 中删除记录，不会影响 OpenViking 服务器上的数据。

## 不支持的操作

以下操作对 OpenViking 知识源不可用，会返回错误：

| 操作 | 原因 |
|------|------|
| `knowledgeBuild` | OpenViking 数据由外部维护，无需本地构建 |
| `knowledgeResume` | 同上 |
| `knowledgeSemanticSubmitFile` | 同上 |
| `knowledgeAdd` | OpenViking 知识源通过 providers.yaml 配置，不支持动态添加路径 |
| `knowledgeUpdate` | 同上 |

如需修改 OpenViking 知识源配置，请直接编辑 `~/.loom/knowledge/providers.yaml` 文件。

## 故障隔离

- 单个 OpenViking 知识源故障（网络超时、HTTP 错误）不会影响其他知识源的搜索
- 错误信息会输出到 stderr，搜索结果中不包含故障源的结果
- 可通过 `knowledgeStatus` 检查知识源状态

## 本地知识源与 OpenViking 知识源共存

loom 支持同时使用本地知识源和 OpenViking 知识源。搜索时所有已启用的知识源都会参与：

- **本地知识源**：通过 `knowledgeBuild` 构建后，使用 BM25 + 语义匹配搜索
- **OpenViking 知识源**：通过 `providers.yaml` 配置后，远程调用 OpenViking API 搜索

两类知识源的搜索结果会合并后统一排序返回。

## 配置示例：企业多知识库

```yaml
sources:
  # Confluence 知识库
  - name: confluence
    endpoint: http://openviking.internal:1933
    apiKeyEnv: OPENVIKING_API_KEY
    account: company-a
    user: dev-team
    targetUri: viking://resources/confluence/
    timeoutSecs: 10

  # 内部 Wiki
  - name: internal-wiki
    endpoint: http://openviking.internal:1933
    apiKeyEnv: OPENVIKING_API_KEY
    account: company-a
    targetUri: viking://resources/wiki/
    timeoutSecs: 10

  # 技能库
  - name: skills-db
    endpoint: http://openviking.internal:1933
    apiKeyEnv: OPENVIKING_API_KEY
    account: company-a
    targetUri: viking://memories/skills/
    timeoutSecs: 5
```

## FAQ

### Q: providers.yaml 修改后需要重启吗？

A: 不需要。`providers.yaml` 在每次 `knowledgeList`、`knowledgeSearch` 等操作读取 registry 时动态加载。

### Q: 如何调试 OpenViking 连接问题？

A: 可以用 `curl` 手动测试 OpenViking API：

```bash
curl -X POST http://openviking.internal:1933/api/v1/search/find \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $OPENVIKING_API_KEY" \
  -H "X-OpenViking-Account: company-a" \
  -d '{"query": "test", "target_uri": "viking://resources/", "limit": 5}'
```

### Q: 本地 registry.json 中已有同名知识源会怎样？

A: `providers.yaml` 中的同名知识源会更新 registry.json 中该知识源的 provider 配置，不会创建重复记录。建议本地知识源和 OpenViking 知识源使用不同的名称。
