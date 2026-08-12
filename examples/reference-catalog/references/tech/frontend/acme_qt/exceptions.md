# AcmeQt 异常处理实现质量

本文件是 Acme 企业参考,指导 agent 如何在 Loom 任务中处理 AcmeQt 应用的异常和错误恢复。

## When To Use

- 任务变更了 AcmeQt 应用的异常处理、错误恢复或用户错误提示逻辑。
- 任务新增了可能失败的操作(网络请求、文件 IO、数据库访问)。
- 优先使用仓库约定。仅当现有项目已支持时才引入 AcmeQt 异常处理模式。

## Implementation Focus

- 使用 AcmeQt 的异常层次结构,不要直接抛出 `Exception` 或 `RuntimeError`:
  - `AcmeUiError` — UI 层错误(视图不存在、Widget 状态无效)
  - `AcmeServiceError` — 服务层错误(网络超时、数据解析失败)
  - `AcmeConfigError` — 配置错误(缺失配置项、格式错误)
  - `AcmeDataError` — 数据错误(验证失败、数据不一致)
- UI 层异常通过 `AcmeMainWindow.handle_error(error)` 统一处理,不要在 Widget 中
  直接弹 `QMessageBox`。`handle_error` 根据异常类型选择提示方式(对话框/状态栏/通知)。
- 服务层异常在到达 UI 前转换为 `AcmeUiError`,附带用户可理解的消息。
  不要将 `AcmeServiceError` 的技术细节(堆栈、SQL)直接展示给用户。
- 用户可见错误消息使用企业标准格式:`{操作描述}失败:{原因}。{建议操作}。`
  例如:`数据导入失败:网络连接超时。请检查网络后重试。`
- 可恢复异常使用重试模式,通过 `AcmeRetry` 装饰器声明式表达:
  ```python
  from acme_qt.retry import AcmeRetry

  @AcmeRetry(max_attempts=3, delay=1.0, backoff=2.0)
  def fetch_data(url: str) -> dict: ...
  ```
- 不可恢复异常记录 ERROR 日志(通过 `AcmeLogger`)并展示用户友好提示。
  不要静默吞掉异常(`except: pass` 绝对禁止)。
- 在 `Application.shutdown()` 中注册全局异常钩子,捕获未处理异常并记录日志,
  避免应用崩溃时无日志。

## Verification Focus

- 运行 `uv run pytest` 证明变更可运行且测试通过。
- 为每个异常路径添加测试,包括至少一个"服务层异常 → UI 层转换"的测试。
- 验证用户可见错误消息符合企业标准格式(操作描述 + 原因 + 建议操作)。
- 验证可恢复异常的重试行为(重试次数、退避间隔)。
- 验证不可恢复异常被记录为 ERROR 日志,不静默吞掉。

## Evidence Focus

- 在证据总结中,说明使用 AcmeQt 异常层次结构,UI 层通过 `handle_error` 统一处理。
- 说明服务层异常转换为 `AcmeUiError`,用户消息符合企业标准格式。
- 说明可恢复异常使用 `AcmeRetry` 装饰器,不可恢复异常记录 ERROR 日志。
- 说明无静默异常吞掉(`except: pass` 禁止)。
