# Flutter 项目与功能结构

仅当任务拥有应用设置、前端架构、依赖/配置/资源/代码生成或框架迁移时应用结构指导。普通 widget 任务应保留现有放置而不加载完整的项目布局参考。

## 保留仓库形态

在创建目录之前标识项目是功能优先、层优先、包基础还是混合。扩展所选的所有权边界；不要将参考树复制覆盖已建立的应用。

功能优先结构仅在那些层有意义存在时分离展示/状态/领域/数据：

```text
lib/
  app/                 # bootstrap, router, theme, global composition
  core/                # stable cross-feature platform/infrastructure primitives
  features/
    orders/
      presentation/    # screens and feature widgets
      state/           # selected providers/blocs
      domain/          # accepted entities/use cases/ports
      data/            # DTOs, repositories, adapters
```

不要创建空洞的仪式层或将功能特定规则放在 `core`、`shared` 或 `utils` 中。共享 widget/工具必须真正可复用，不得导入功能 repository/状态。

## 引导与环境

保持 `main.dart` 精小且确定：初始化所需的框架/plugin/配置，安装根 provider/路由/主题/本地化，然后运行应用。仅当产品/运行时契约选择时分离 flavor/入口点。

将浏览器/移动端配置视为公开的。将 API 绑定/环境值保留在仓库的运行时/构建机制中并验证它们；永远不要嵌入服务凭据。

可能失败的初始化需要加载/失败/恢复行为或清晰的启动失败。不要在没有已接受生命周期的情况下在每次应用启动时执行数据库迁移、无界网络调用或种子业务数据。

## 依赖与 pubspec

在添加包之前，检查现有能力、目标平台支持、SDK 约束、维护、传递/原生需求、许可证和包/构建影响。不要因为模板使用就添加 Riverpod、Bloc、GoRouter、Dio、Freezed 或存储库。

保持依赖版本与所选 Flutter/Dart SDK 和锁文件策略兼容。避免将宽泛的 `dependency_overrides` 作为常规解决方案。

准确声明字体/资源，包括大小写敏感的路径和目录语义。验证功能使用的每个目标。

## 生成代码

使用仓库的 `build_runner`、Riverpod 生成器、Freezed、JSON 序列化、本地化或资产生成命令和提交输出策略。源注解和生成输出必须一致。

永远不要手动编辑生成文件。注意模型/provider 变更后的过期生成代码、重复 part 名称、冲突 builder 和缺失输出。

生成的持久化/JSON model 是传输/存储表示；在已接受架构分离处保持领域/UI 语义独立。

## 平台边界

将平台 plugin 放在窄 adapter 之后，保持 Android/iOS/Web/桌面配置同步：权限、manifest/plist、URL scheme、entitlement、最低版本、签名能力和 plugin 初始化。

使用为每个所选目标编译的条件导入/实现。避免在 Web 代码中导入 `dart:io` 或在功能 widget 中散布平台检查。

按产品需求而非 plugin 默认值添加平台权限文案和 denied/restricted 行为。

## 路由、主题、本地化与资源

在支持处保持一个路由组合所有者和功能拥有的路由片段。屏幕不得创建竞争的路由实例。

集中主题/颜色/文本/间距/组件 extension 和本地化 delegate/生成字符串。功能代码消费语义令牌/字符串而非硬编码视觉或语言值。

资源和本地化键需要确定性命名、所有权、移除和测试/构建验证以防止过期的包内容。

## 依赖方向

通过导入/包强制已接受的方向并避免功能循环。数据 adapter 可实现领域/应用端口；当那些层分离时领域代码不应依赖 Flutter widget、BuildContext、状态库或平台 plugin。

状态/导航应依赖功能操作，而非让 repository 依赖 UI。不要使用全局 service locator 擦除项目边界。

## Verification

- 在 `pubspec.yaml`、SDK、lint 或生成器变更后运行依赖解析/分析。
- 在平台 plugin/配置/条件导入变更后构建/测试所选目标。
- 重新生成输出并验证无过期/手动编辑的生成文件。
- 从新路径验证资源/字体/本地化/主题/路由组合。
- 检查导入/包依赖的循环和功能泄漏。
- 确认功能代码可在不启动不相关原生 service/路由的情况下测试。

## 交付证据

标识结构/依赖/配置/生成/平台决策和证明它的解析/分析/构建/导入断言。目录树或成功的编辑器导入不能证明目标构建、plugin 设置、生成器新鲜度或依赖方向。

## 不安全默认行为

- 将参考文件夹树强加于已建立项目。
- 空洞的 clean-architecture 层和功能逻辑放在 `core/shared/utils` 中。
- 从示例添加包而无技术栈/平台/许可证检查。
- 生成文件手动编辑或留过期。
- 平台权限/配置仅为一个所选目标变更。
- 使用全局 service locator 绕过依赖边界。
