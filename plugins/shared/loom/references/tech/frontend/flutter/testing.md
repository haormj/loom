# Flutter 测试

仅对拥有测试的任务使用 Flutter 框架测试。为变更边界选择最小证明。当单独分配时，浏览器/设备集成参考拥有真正的端到端运行时、平台和渲染多视口证据。

## 证明边界

| 声明 | 适用证明 |
|---|---|
| 纯验证器/映射器/用例 | Dart unit test |
| Riverpod notifier/provider | `ProviderContainer` test |
| Bloc/Cubit 转换 | bloc test/unit test |
| Widget 渲染/交互 | `testWidgets` |
| 路由栈/重定向 | widget/router test |
| Plugin/platform channel adapter | adapter test plus selected platform integration |
| 完整应用工作流/真实 service | integration/device/browser task |

不要为每个纯规则启动整个应用，也不要声称 widget 测试能证明原生权限对话框、深链接 OS 配置、Web 托管回退或部署 API 绑定。

## Widget Harness

构建聚焦的测试应用，配以所需主题、本地化、MediaQuery、路由和 provider/bloc override。避免使不相关依赖成为每个测试一部分的巨大生产应用 fixture。

通过 finder、语义、文本字段、点击、滚动、键盘操作和可见输出交互。不要修改私有 State 字段来制造结果。

使用 `pump` 进行受控帧，仅在动画/定时器实际稳定时使用 `pumpAndSettle`。重复定时器、无限动画、轮询或未解决的 stream 可能使 `pumpAndSettle` 隐藏/挂起；显式 pump 预期持续时间/事件。

## 状态与依赖测试

用确定性 fake override repository、时钟、存储、权限、网络/连接性和平台端口。断言目标 ID、载荷、调用、持久状态映射和禁止副作用的缺失。

Riverpod 测试应创建/释放隔离容器并验证 notifier 状态/生命周期/失效。Bloc 测试应断言精确状态序列/并发并关闭实例。

不要 mock 被声称的 notifier/bloc/widget 行为。Mock 的状态流可用于 widget 渲染但不是状态实现的证明。

## 表单、列表与语义

覆盖任务所属的加载、空、就绪、验证、业务冲突、权限、离线/不可用、提交中、成功、禁用和破坏性确认状态。

对于列表，在排序/筛选/分页/刷新后测试稳定目标标识并在拥有处验证滚动/分页去重。对于表单，验证草稿保留、字段/全局错误、焦点、重复提交和服务端回读协调。

使用语义测试标签、角色/操作、切换/禁用状态、错误宣告、命中目标和自定义控件。在布局变更时练习文本缩放、窄/宽约束、长/本地化内容。

## 导航与平台

路由测试应证明初始位置、参数、push/replace/back、shell/标签栈、重定向、无效/未找到/禁止和监听器去重。平台深链接/冷启动需要集成证据。

为单元/widget 测试抽象 method channel/plugin，当 manifest/plist/权限/原生行为是声称内容时添加所选目标集成测试。不要用一个平台 fake 作为所有目标设置有效的证明。

## Golden 与集成测试

仅在仓库工具维护字体、设备像素比、主题、本地化和更新/审查工作流且视觉回归风险证明时使用 golden。Golden 不证明交互/可访问性。

集成测试需要确定性种子/账户/数据、对可观察状态的显式等待而非休眠以及清理。将环境/运行时失败与代码失败分开。

## 生成模型与序列化

当 model 变更时测试有效/缺失/null/未知/向后兼容的 JSON/存储数据和枚举/日期/小数/版本映射。测试前重新生成，永远不要修补生成输出来满足测试。

## 验证与清理

首先运行变更的测试文件，然后仅在共享类型、路由、生成代码或平台配置变更时运行聚焦的 `flutter analyze`/所属包测试/构建。在无仓库策略时不要施加通用覆盖率阈值。

释放容器/bloc/controller/fake，恢复平台 dispatcher/override，清除定时器并验证未处理异常。任意休眠和测试顺序依赖是缺陷。

## 交付证据

记录边界、场景、命令和有意义的 widget/状态/路由/平台断言。通过计数、覆盖率、私有状态或一个 golden 不能证明生命周期、导航、平台设置、响应式语义或真实集成。

## 不安全默认行为

- 仅当已接受任务拥有 Flutter 测试创建、测试修改或测试专用验证时才加载此参考。
- 为每个纯规则使用整个应用 widget 测试。
- 私有 State 修改或 mock 状态声称是实现证明。
- 在不理解待处理工作的情况下使用 `pumpAndSettle`/任意休眠。
- Golden 用于交互或可访问性声明。
- 一个平台 fake 声称为原生配置证据。
- 从外部指导复制的通用覆盖率要求。
