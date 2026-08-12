# AcmeQt 日志实现质量

本文件是 Acme 企业参考,指导 agent 如何在 Loom 任务中使用 AcmeQt 集成的日志框架。

## When To Use

- 任务变更了 AcmeQt 应用的日志记录、日志配置或日志格式。
- 任务新增了需要日志的功能模块。
- 优先使用仓库约定。仅当现有项目已支持时才引入 AcmeQt 日志模式。

## Implementation Focus

- 使用 AcmeQt 的 `AcmeLogger` 获取日志器,不要直接使用 `logging.getLogger`。
  `AcmeLogger` 自动关联 AcmeQt 的日志总线,将 UI 事件日志与应用日志统一管理。
  ```python
  from acme_qt.logging import AcmeLogger
  logger = AcmeLogger(__name__)
  ```
- 日志级别遵循企业约定:
  - `DEBUG` — 开发调试信息(默认不输出到生产)
  - `INFO` — 关键业务事件(视图切换、用户操作、应用启动/退出)
  - `WARNING` — 可恢复的异常(网络超时重试、配置降级)
  - `ERROR` — 不可恢复的异常(数据库连接失败、关键服务不可用)
- 结构化日志使用 `logger.info("event", extra={"view_id": ..., "user_action": ...})`
  格式,不要用 f-string 拼接日志消息。结构化字段便于日志聚合和查询。
- 敏感数据脱敏:日志中不记录用户密码、token、完整身份证号。使用 `AcmeLogger` 的
  `redact()` 辅助函数处理敏感字段。
- UI 操作日志通过 `AcmeWidget.log_action(action_name, **context)` 记录,
  不要在 Widget 中直接调用 `logger.info`。`log_action` 自动附带 Widget ID 和时间戳。
- 异步任务的日志使用 `AcmeLogger` 的 `task_context` 上下文管理器,自动注入任务 ID:
  ```python
  with logger.task_context("import_data"):
      logger.info("start")
      ...
      logger.info("done")
  ```

## Verification Focus

- 运行 `uv run pytest` 证明变更可运行且测试通过。
- 验证日志输出为结构化格式(可被 `json.loads` 解析)。
- 验证敏感数据在日志中已脱敏(无明文密码/token)。
- 如果新增了 UI 操作日志,验证 `log_action` 自动附带 Widget ID 和时间戳。

## Evidence Focus

- 在证据总结中,说明使用 `AcmeLogger` 而非标准 `logging.getLogger`。
- 说明日志级别遵循企业约定,日志为结构化格式。
- 说明敏感数据已脱敏,UI 操作通过 `log_action` 记录。
