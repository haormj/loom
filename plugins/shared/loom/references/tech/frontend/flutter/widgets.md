# Flutter Widget 与业务界面

围绕任务所属的产品界面、可见状态和用户意图设计 widget。可复用 widget 接收类型化值/回调；功能屏幕可以协调所选状态/repository 边界，不在布局代码中隐藏业务逻辑。

## Widget 契约

当没有值就无法继续渲染时使用必需构造函数参数，为可选行为使用显式默认值。保持回调面向事件并包含稳定的目标标识。

```dart
class OrderRow extends StatelessWidget {
  const OrderRow({
    required this.order,
    required this.onInspect,
    this.disabled = false,
    super.key,
  });

  final OrderRowViewModel order;
  final ValueChanged<String> onInspect;
  final bool disabled;

  @override
  Widget build(BuildContext context) => ListTile(
    key: ValueKey(order.id),
    title: Text(order.number),
    onTap: disabled ? null : () => onInspect(order.id),
  );
}
```

不要将 repository、路由对象或可变 API 实体传递到展示型 widget 中。避免具有许多布尔值编码不相关产品变体的通用组件。

## 构建纯度与生命周期

保持 `build()` 确定性且廉价。在所属 State 生命周期中创建/释放文本/编辑/滚动/动画/焦点 controller 和订阅，或安全使用已建立的 hook。

对静态构造函数/子项使用 `const`，但当状态所有权/重建范围是真正问题时不要仅为 const 而改动代码。永远不要在构建期间直接启动网络调用、定时器或状态写入。

仅为其预期的生命周期语义使用 `didUpdateWidget`、`didChangeDependencies`、post-frame 回调和键。Post-frame 回调不得成为无尽的 setState 循环。

## 布局与响应式

用约束（`LayoutBuilder`、`MediaQuery`、仓库断点）而非假设设备尺寸来组合。在内容适配时保持固定控件/工具栏稳定。

有意识地使用 `Expanded`/`Flexible`/滚动视图；避免无界约束、嵌套同轴可滚动和对大型动态列表使用 shrinkWrap。对于密集列表/详情工作流，定义移动卡片/抽屉/路由和平板/桌面分屏行为。

尊重安全区域、键盘 inset、文本缩放、长/本地化标签、方向、指针/hover 和桌面/Web 宽度。控件和文本不得重叠或不连贯地裁剪。

## 集合与 Sliver

对大型/未知集合使用 `ListView.builder`、`GridView.builder`、`SliverList`、分页或仓库抽象。在重排/筛选/刷新后保留稳定键和显示的操作标识。

当一个协调的滚动面需要 app bar、头、网格和列表时使用 sliver。避免仅为重现静态 mockup 而嵌套独立可滚动。

空/错误/加载状态属于集合区域内部，同时保留周围页面上下文和操作。

## 表单、对话框与操作

在稳定生命周期处拥有 `Form` 键/controller。区分客户端字段验证和后端字段/全局错误。保留草稿，聚焦第一个有意义的错误，防止重复提交，并协调保存的响应。

使用所选的 Material/Cupertino/设计系统对话框、菜单、sheet、picker 和按钮，配以语义标签和焦点/键盘行为。破坏性确认必须显示受影响对象和后果；仅 snack bar 不是确认。

当语义按钮/列表控件适合时不要使用可点击的 `Container`/`GestureDetector`。自定义交互需要 `Semantics`、焦点、键盘快捷方式/操作、命中尺寸、按下/禁用反馈和屏幕阅读器行为。

## 主题与视觉系统

使用 `Theme.of`、`ColorScheme`、文本主题、theme extension 和仓库令牌/组件。避免在功能 widget 内部重复字面颜色、间距、圆角和排版。

图像/图标需要有界尺寸、fit、加载/错误行为、语义和资源声明。动画应传达状态并尊重减少动画/平台行为；避免掩盖重复工作的装饰性动画。

## Verification

- 通过可见文本/语义测试渲染，通过点击/键盘/输入测试回调。
- 覆盖 widget 所属的加载、空、就绪、验证、禁用、提交中、冲突、权限、离线和确认状态。
- 在排序/筛选/分页/刷新后验证动态行标识/操作目标。
- 测试焦点、语义、命中目标、错误关联、对话框/sheet 关闭和焦点返回。
- 练习代表性的窄/宽/文本缩放/长内容约束。
- 仅在仓库维护 golden 测试且视觉回归是实际风险时使用 golden 测试。

## 交付证据

命名 widget 契约/布局/状态和证明它的 widget/语义/视口断言。构造函数单元测试、静态树转储或单个无约束 golden 不能证明交互标识、生命周期、可访问性或响应式可用性。

## 不安全默认行为

- 在可复用视觉 widget 内部访问 repository/路由/状态库。
- 在 `build()` 中创建 future/controller/provider。
- 大型急切子列表或 shrinkWrap 用于消除约束问题。
- 以索引为键或标识重要处无键的动态行。
- 原始手势容器替换语义控件。
- 绕过所选主题/令牌的一次性视觉字面值。
