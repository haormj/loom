# API 运维语义

本引用用于影响重复调用、重试、缓存、速率限制、请求追踪或条件更新的 API 行为。这些规则并非对每个端点强制；仅在当前阶段、仓库约定或已接受 API 契约需要时应用。本文件拥有运维策略；`errors.md` 拥有错误类别、代码、字段和安全客户端消息。

## 幂等性

| 操作 | 默认期望 | 何时添加显式策略 |
|---|---|---|
| `GET`、`HEAD`、`OPTIONS` | 安全且幂等。 | 缓存验证器或条件读取重要时。 |
| `PUT`、`DELETE` | 幂等终态。 | 并发更新或重复删除需要定义的状态行为时。 |
| `PATCH` | 通常不幂等，除非专门设计。 | 部分更新可被客户端或 worker 重试时。 |
| `POST` | 默认不幂等。 | 支付、提交、工作流转换、导入或重复点击场景可能产生重复副作用时。 |

对于重试敏感的 `POST` 操作，在已接受 API 契约中记录幂等策略：

```json
{
  "idempotencyPolicy": {
    "required": true,
    "keyHeader": "Idempotency-Key",
    "dedupeScope": "actor_and_operation",
    "duplicateBehavior": "return_original_result"
  }
}
```

不要为简单的一次性内部操作添加幂等存储或头，除非重复提交是当前阶段的真实风险。

## 缓存和条件请求

对于读 API，缓存是可选的，应遵循已有仓库或产品需求。

在以下情况使用 `cachePolicy`：

- 读取载荷昂贵或频繁重复
- UI 可安全复用缓存数据
- 仓库已使用 `ETag`、`Last-Modified` 或 cache-control 头

当更新需要乐观并发时使用条件请求：

```json
{
  "cachePolicy": {
    "etag": true,
    "lastModified": false,
    "cacheControl": "private, max-age=60"
  },
  "conditionalRequestPolicy": {
    "ifMatchRequiredForUpdates": true,
    "staleUpdateStatus": 412
  }
}
```

不要为易变的工作流状态编造缓存验证器，除非陈旧读取是可接受且已描述的。

## 速率限制

仅对公开的、类登录的、搜索密集的、导入/批量导向的或其他滥用敏感的 API 声明 `rateLimitPolicy`。

```json
{
  "rateLimitPolicy": {
    "applies": true,
    "status": 429,
    "headers": ["Retry-After"],
    "clientMessage": "Too many requests. Try again later."
  }
}
```

对于内部工具，当当前部署或认证仅限本地时可以推迟速率限制；仅在本阶段滥用或意外负载可能时才记录风险。

## 重试和可用性响应

对于重试可能成功的依赖或运行时中断，使用重试指导：

- `408`、`429`、`502`、`503`、`504` 可能可重试，取决于上下文。
- `400`、`401`、`403`、`404`、`409`、`422` 通常不可重试，除非用户或数据变更。
- 当服务端能给出有意义的延迟时包含 `Retry-After`。

当依赖暂时不可用时，优先使用清晰的 `503` 或领域特定的阻断错误，而非通用 `500`。

## 请求追踪

对于有后台工作、外部依赖或重要业务副作用的 API，定义请求 ID 策略：

```json
{
  "requestIdPolicy": {
    "header": "X-Request-ID",
    "includedInErrorBody": true,
    "logCorrelationRequired": true
  }
}
```

已接受的 `request_id` 运维策略是应用可观测性引用的结构化所有权信号。它不要求记录每个端点，也不将请求追踪或日志保留移入部署。

实现证据应引用证明所选运维策略的测试、运行时探针、日志或源文件。不要仅在描述中声称运维语义。
