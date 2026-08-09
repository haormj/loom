# Flutter 应用实现

在仓库的 Flutter/Dart 版本、目标平台、主题/设计系统、状态/导航选择、API 契约和平台配置内实现已接受的前端体验。不要因为外部示例偏好另一个技术栈就替换已建立的库。

## 应用边界

保持 `main()` 聚焦于所需的 binding/配置初始化和根 app/provider scope。将产品工作流放在功能屏幕/状态/用例中，将基础设施放在 repository/service/adapter 之后。

Widget 渲染状态并发出意图。它们不应在 `build` 内构造 API client、持久化 store、权限 adapter 或 repository。将领域/业务修改排除在可复用视觉 widget 之外。

对真正的本地临时行为选择本地 `StatefulWidget` 状态；仅对跨 widget/路由/业务生命周期使用所选的共享状态库。Riverpod 和 Bloc 是备选方案，除非已接受架构有意混用有界用途。

## 异步工作流状态

在所属界面/控件处表示加载、空、就绪、验证、业务阻止、提交中、成功、禁用、离线、过期和意外失败状态。不要将所有错误折叠为一个 `Text('Error')` 或将失败视为空数据。

维护与持久化/服务端状态分开的可编辑草稿。保存时，保留有效输入，映射字段/全局错误，阻止重复，协调返回的标识/版本/状态，并使重试/恢复显式。

在 await 之后使用 mounted/context 安全：

```dart
Future<void> submit() async {
  setState(() => submitting = true);
  final result = await repository.save(draft);
  if (!mounted) return;
  setState(() => submitting = false);
  result.fold(showFailure, applySavedRecord);
}
```

在共享状态代码中，优先使用状态/监听器驱动的副作用而非跨异步边界直接保留 `BuildContext`。

## 标识、不可变性与重建

按仓库模式使用不可变状态/模型。替换列表/map/model 而非原地修改被监听的状态。

对筛选、排序、可分页、动画、可重排或可编辑的集合使用稳定的领域键。键保留元素标识；它们不能修复以数组索引为键的状态模型。

在值为静态处使用 `const` 构造函数/子项并保持 `build()` 纯净。不要在每次构建期间创建 controller、future、stream、focus node、provider 或 client。

## 平台与响应式行为

一致地遵循所选的 Material/Cupertino/自定义设计系统。为任务所属的目标平台适配导航、安全区域、键盘/inset、指针/hover、文本缩放、方向、桌面窗口宽度和 Web URL 行为。

平台分支应能力驱动且可测试；不要在 widget 中散布 `Platform.is...` 或将 `dart:io` 导入 Web 兼容路径。将权限和原生 plugin 放在 adapter 之后，配以 denied/permanently-denied/unavailable 状态。

基于内容和仓库规则使用响应式约束/断点。密集业务 UI 需要可用的窄屏组合，而非缩小的桌面表格。

## API、存储与安全

为已接受的 API 路径、载荷、状态、auth、分页和错误使用类型化 repository/service。保持浏览器/移动端 base URL 和运行时绑定与部署/平台网络一致；不要将 emulator/localhost 地址硬编码到产品代码中。

按数据敏感性/生命周期选择安全存储、偏好设置、数据库或缓存。客户端存储在受损设备/浏览器面前不是秘密。永远不要嵌入服务凭据或将隐藏 UI/路由视为授权。

将连接性作为信号处理，而非请求会工作的证明。仅在已接受时保留离线/过期策略和待写入行为。

## 本地化、主题与内容

使用仓库的本地化生成和区域感知的日期/数字/货币。不要拼接无法翻译的句子片段或硬编码一个区域设置的格式。

使用 theme extension/令牌/组件而非一次性颜色、间距、文本样式和圆角卡片。尊重文本缩放、对比度、减少动画和长内容。

产品 UI 不得暴露运行时命令、框架备注、交付进度、验证指令、堆栈说明或调试错误。

## Verification

- 为变更的 Dart 和生成代码运行聚焦的分析/测试。
- 练习任务所属的异步状态、草稿/保存/回读、重复阻止和恢复。
- 在筛选/排序/分页/刷新/导航后验证稳定的行/操作标识。
- 在变更时测试目标平台键盘/安全区域/权限/存储行为。
- 为变更界面验证响应式、文本缩放、本地化、长内容和语义行为。
- 当 plugin/平台配置变更时构建/运行相关目标。

## 交付证据

标识 Flutter app/状态/平台/API 决策和证明它的 widget/provider/bloc/平台断言。分析成功或一张截图不能证明生命周期、状态协调、平台配置、可访问性或运行时 API 绑定。

## 不安全默认行为

- 在无已选择技术栈和状态所有权的情况下引入 Riverpod 或 Bloc。
- 在 `build()` 中构造 service/controller/future。
- 原地修改被监听状态或以索引为键的行。
- 跨异步操作保留 `BuildContext` 而无生命周期安全。
- 源代码中硬编码设备/emulator URL 和密钥。
- 将平台行为简化为视觉差异而无输入/权限/生命周期处理。
