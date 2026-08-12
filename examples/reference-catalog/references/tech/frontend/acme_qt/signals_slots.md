# AcmeQt 信号与槽实现质量

本文件是 Acme 企业参考,指导 agent 如何在 Loom 任务中正确使用 AcmeQt 的信号/槽机制。

## When To Use

- 任务变更了 AcmeQt 信号/槽连接、事件传递或跨组件通信逻辑。
- 任务新增了自定义信号或事件处理逻辑。
- 优先使用仓库约定。仅当现有项目已支持时才引入 AcmeQt 信号/槽模式。

## Implementation Focus

- 使用 AcmeQt 的 `AcmeSignal` 类型声明自定义信号,不要直接使用 `pyqtSignal`。
  `AcmeSignal` 提供了类型检查、线程安全和日志追踪。
- 信号连接使用 `AcmeSignal.connect(slot, connection_type)` 显式指定连接类型。
  默认使用 `Qt.QueuedConnection` 用于跨线程通信,`Qt.DirectConnection` 用于同线程。
  不要依赖 Qt 的自动推断。
- 跨线程信号必须携带不可变数据(str/int/float/tuple/dataclass),不要传递可变对象
  或 Qt 对象引用。可变对象在跨线程传递前序列化为 dataclass 或 JSON。
- 使用 `AcmeSignalHub` 管理全局信号(如应用级事件总线),不要在 Widget 之间
  直接连接信号。`AcmeSignalHub` 提供了命名空间隔离和生命周期管理。
- 信号断开使用 `AcmeSignal.disconnect(slot)`,在 Widget 销毁前显式断开所有连接。
  不要依赖 Qt 的自动断开机制(在复杂场景下可能泄漏)。
- 避免信号循环连接(A → B → A)。使用 `AcmeSignalHub` 的单向消息流设计,
  或在槽函数中用 `_updating` 标志防止递归。

## Verification Focus

- 运行 `uv run pytest` 证明变更可运行且测试通过。
- 如果信号涉及跨线程通信,验证数据完整性(无可变对象传递、无竞态)。
- 如果新增了 `AcmeSignalHub` 通道,验证信号在 Widget 销毁后不泄漏(无幽灵回调)。
- 如果变更了连接类型,验证目标线程的事件循环正确处理排队连接。

## Evidence Focus

- 在证据总结中,说明使用 `AcmeSignal` 而非 `pyqtSignal`,连接类型显式指定。
- 说明跨线程信号携带不可变数据,通过 `AcmeSignalHub` 管理全局信号。
- 说明 Widget 销毁前显式断开信号连接,无信号循环。
