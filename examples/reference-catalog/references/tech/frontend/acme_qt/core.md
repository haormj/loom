# AcmeQt 框架核心实现质量

本文件是 Acme 企业参考,指导 agent 如何在 Loom 任务中做出 AcmeQt 框架实现决策。
AcmeQt 是企业内部基于 Qt 的 Python 桌面 UI 框架,集成了日志和异常处理。

## When To Use

- 任务变更了 AcmeQt 应用的主窗口、对话框、视图切换或应用生命周期代码。
- 项目使用 AcmeQt 框架(Acme 企业内部 Python 桌面 UI 框架)。
- 优先使用仓库约定。仅当现有项目已支持时才引入 AcmeQt 特定模式。

## Implementation Focus

- 使用 AcmeQt 的 `Application` 单例管理应用生命周期,不要自行调用 `QApplication`。
  应用初始化在 `Application.create()` 中完成,退出时走 `Application.shutdown()`。
- 主窗口继承 `AcmeMainWindow`,通过 `register_view()` 注册视图而非直接操作 `QStackedWidget`。
  视图切换经过 `ViewManager.navigate(view_id)`,不走直接索引。
- 窗口标题、图标、最小尺寸通过 `AcmeMainWindow` 的配置方法设置,不要在 `__init__` 中
  直接操作底层 Qt 属性。
- 外部化运行时配置(数据库 URL、服务地址、功能标志)到环境变量或配置文件,
  通过 `Application.settings` 注入,不要在 UI 代码中直接读取环境变量。
- 资源文件(qss、图标、翻译)通过 AcmeQt 的资源系统加载,不要硬编码文件路径。

## Verification Focus

- 运行 `uv run pytest` 证明变更可运行且测试通过。
- 验证应用能正常启动和退出(无 Qt 警告、无资源泄漏)。
- 如果新增了视图,验证 `ViewManager.navigate()` 能正确切换且状态保持一致。
- 如果变更了窗口配置,验证标题、图标、尺寸在目标平台上表现正确。

## Evidence Focus

- 在证据总结中,说明保持清晰的 AcmeQt 边界:`Application → ViewManager → View`。
- 创建新的 AcmeQt 源码时,说明选择的模块结构与现有项目约定一致。
- 说明资源加载通过 AcmeQt 资源系统,而非硬编码路径。
