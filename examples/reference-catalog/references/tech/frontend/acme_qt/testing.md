# AcmeQt 测试实现质量

本文件是 Acme 企业参考,指导 agent 如何在 Loom 任务中为 AcmeQt 应用编写测试。

## When To Use

- 任务新增或变更了 AcmeQt Widget、视图或服务逻辑,需要编写测试。
- 任务属于测试增量(TaskKind::VerificationIncrement)。
- 优先使用仓库约定。仅当现有项目已支持时才引入 AcmeQt 测试模式。

## Implementation Focus

- 使用 `pytest` + `pytest-qt` 作为测试框架,通过 `qtbot` fixture 操作 Widget:
  ```python
  def test_view_switch(qtbot):
      window = AcmeMainWindow()
      qtbot.addWidget(window)
      window.view_manager.navigate("settings")
      assert window.view_manager.current_view_id == "settings"
  ```
- Widget 测试使用 `qtbot.addWidget(widget)` 注册,确保测试结束后正确清理。
  不要手动创建和销毁 Widget(可能导致 Qt 警告或资源泄漏)。
- 信号测试使用 `qtbot.waitSignal(signal, timeout=5000)` 等待信号发射,
  不要用 `time.sleep` 轮询。超时设为 5 秒(企业标准)。
- 异步操作测试使用 `qtbot.waitUntil(condition, timeout=5000)` 等待条件满足。
- 服务层测试使用 mock 隔离外部依赖(网络、数据库、文件系统),不要在单元测试中
  发起真实网络请求或数据库连接。
- 使用 `AcmeTestApp` fixture 创建完整应用上下文(包含 `Application`、`ViewManager`、
  `AcmeSignalHub`),不要在测试中手动拼装应用实例:
  ```python
  def test_app_startup(acme_test_app):
      app = acme_test_app
      assert app.is_running()
      assert app.main_window is not None
  ```
- UI 交互测试(点击、输入)使用 `qtbot.mouseClick`/`qtbot.keyClick`,
  不要模拟底层 Qt 事件。
- 测试覆盖目标:新增业务逻辑至少 80% 行覆盖率,异常路径 100% 覆盖。

## Verification Focus

- 运行 `uv run pytest` 证明所有测试通过。
- 运行 `uv run pytest --cov=src` 验证覆盖率达标(新增逻辑 ≥80%,异常路径 100%)。
- 验证测试中无真实网络请求或数据库连接(使用 mock 隔离)。
- 验证 Widget 测试使用 `qtbot.addWidget` 注册,无 Qt 资源泄漏警告。

## Evidence Focus

- 在证据总结中,说明使用 `pytest` + `pytest-qt`,Widget 通过 `qtbot` 操作。
- 说明信号测试使用 `waitSignal`,异步测试使用 `waitUntil`,无 `time.sleep` 轮询。
- 说明服务层测试使用 mock 隔离外部依赖,覆盖率达标。
- 说明使用 `AcmeTestApp` fixture 创建应用上下文,不手动拼装实例。
