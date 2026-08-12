# AcmeQt Widget 实现质量

本文件是 Acme 企业参考,指导 agent 如何在 Loom 任务中创建和组织 AcmeQt Widget。

## When To Use

- 任务变更了 AcmeQt Widget 的创建、布局、样式或交互逻辑。
- 任务新增了 UI 组件(表单、列表、对话框、工具栏)。
- 优先使用仓库约定。仅当现有项目已支持时才引入 AcmeQt Widget 模式。

## Implementation Focus

- 自定义 Widget 继承 `AcmeWidget` 或 `AcmeFrame`,不要直接继承 `QWidget`。
  `AcmeWidget` 提供了统一的样式注入、事件追踪和可访问性支持。
- 使用 AcmeQt 的布局管理器(`AcmeVBoxLayout`/`AcmeHBoxLayout`/`AcmeGridLayout`),
  不要手动设置 Widget 几何位置。布局管理器处理 DPI 缩放和窗口调整。
- 通过 `AcmeWidget.set_style_id(style_id)` 应用样式,不要在代码中拼接 qss 字符串。
  样式 ID 在全局样式表中注册,确保主题切换时一致更新。
- 表单 Widget 使用 `AcmeForm` + `AcmeFormField`,字段定义与验证规则声明式表达,
  不要在提交处理中手动逐字段校验。
- 对长列表使用 `AcmeListView` + `AcmeListModel`,不要用 `QListWidget` 逐项添加。
  `AcmeListModel` 支持虚拟化、批量更新和数据绑定。
- 对话框使用 `AcmeDialog` 工厂方法(`AcmeDialog.confirm()`/`AcmeDialog.input()`),
  保持与企业 UI 标准一致,不要直接实例化 `QDialog`。

## Verification Focus

- 运行 `uv run pytest` 证明变更可运行且测试通过。
- 验证 Widget 在不同 DPI 设置下布局正确(不重叠、不截断)。
- 如果新增了表单,验证必填字段校验和错误提示行为。
- 如果新增了列表,验证大量数据下的滚动性能(无卡顿)。

## Evidence Focus

- 在证据总结中,说明 Widget 继承 `AcmeWidget`/`AcmeFrame`,布局使用 AcmeQt 管理器。
- 说明样式通过 `set_style_id` 应用,而非硬编码 qss。
- 说明对话框通过 `AcmeDialog` 工厂方法创建,而非直接实例化 `QDialog`。
