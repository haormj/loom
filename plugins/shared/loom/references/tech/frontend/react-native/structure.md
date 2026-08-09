# React Native 项目结构与配置

仅当任务拥有应用配置、框架迁移、可复用移动架构或前端体验脚手架时更改项目结构。保留仓库已建立的 Expo/裸原生和功能/模块边界。

## 发现现有形态

在添加目录之前检查包/工作区根、应用入口、`app/` 路由或导航器设置、`src/`/功能文件夹、平台项目、Expo 配置、Metro/Babel 配置、TypeScript/JavaScript 配置、测试、资源和生成原生文件。

标识 `ios/` 和 `android/` 是源拥有的裸项目、由 Expo prebuild 生成还是有意缺失。当仓库期望 config plugin 和可重复 prebuild 时不要编辑生成的原生输出。

尊重 monorepo 包边界和 Metro watch/resolver 配置。TypeScript 解析的模块仍可能在 Metro、Jest、CocoaPods、Gradle 或 EAS/原生构建中失败。

## 路由与功能放置

在 Expo Router 中，路由文件拥有路由组合、参数接收和布局元数据。将可复用屏幕、功能组件、数据 hook、service、验证器和 store 移出路由文件夹，除非它们真正是路由本地的。

在 React Navigation 中，一旦层级变得非平凡，将导航器声明与可复用屏幕实现分开。避免中心导航器直接导入每个功能的基础设施。

在发明全局 `utils`、`helpers` 或 `common` 桶之前按已建立的功能/领域边界组织。共享 UI 原语不得导入产品领域 service；功能组件可以用领域行为组合原语。

将 API、auth、分析、存储、通知、权限和原生 adapter 放在显式类型化模块之后。Service 不应拥有屏幕导航或隐藏的 UI 状态转换。

## 应用配置

将 bundle/package 标识符、应用名称/slug、scheme、associated/universal link、权限、plugin、方向、平板支持、图标、启动资源、运行时/版本策略、更新通道和构建属性视为影响发布的配置。

通过仓库的配置机制合并环境特定设置。不要在源配置中硬编码开发主机、凭据、签名值或 provider ID。

添加 config plugin 或原生依赖时，确认兼容的 Expo SDK/React Native 版本、plugin 排序、生成变更、所需权限以及是否需要 development-client/原生重建。

## 工具配置

保持 Metro、Babel、TypeScript、Jest、Reanimated 和路径别名对齐。如果引入别名，更新消费它的每个 resolver 而非使编辑器/typecheck 成功成为唯一证明。

保留所需的 plugin 排序，如仓库的 Reanimated/worklet 约定。不要在不检查已安装包的情况下复制版本特定的 plugin 规则。

使用仓库语言策略。不要作为附带结构变更将 JavaScript 项目转为严格 TypeScript；迁移需要显式所有权和完整工具覆盖。

## 资源与字体

将资源放在 Metro/Expo/原生配置可以解析的位置。确认精确路径/大小写、支持格式、尺寸、缩放变体、许可证、字体加载和回退行为。

不要引用生成的、仅本地的或被忽略的资源路径。应用图标和启动资源需要平台特定的形状/尺寸/背景检查，不仅是应用内渲染。

## 依赖边界

优先使用仓库中已批准的依赖。对于新包，验证维护的平台支持、Expo 兼容性、原生设置、许可证、包/二进制影响和替换/回退行为。

避免路由、功能、组件、store 和 service 之间的循环导入。当 barrel export 掩盖循环或将仅原生模块拉入共享代码时保持它们窄。

## 迁移与移动

以可构建切片移动文件并同时更新路由发现、导入、测试、mock、Metro/Jest 别名、原生注册和资源引用。不要在切换后留下并行的旧/新模块所有权。

除非已接受变更显式迁移，否则保留路由名称、深链接、存储键、分析 ID 和公共包导出。

## Verification

- 运行聚焦的 TypeScript/JavaScript 检查、Metro bundle/启动、测试和受影响的平台构建/配置验证。
- 当 SDK、plugin 或原生模块变更时使用仓库 Expo doctor/prebuild 或原生依赖检查。
- 证明别名在 Metro 和测试中解析，不仅在编辑器/typechecker 中。
- 验证路由发现、配置 scheme/权限、引用的资源/字体和生成原生期望。
- 检查依赖循环并确保生成的原生输出可从源配置重现。

## 交付证据

命名源拥有的应用根、路由与功能边界、配置/依赖变更、生成/原生所有权和证明所有 resolver/构建路径一致的命令。仅整洁的文件夹树不能建立运行时或发布正确性。

## 不安全默认行为

- 将新文件夹分类法强加于已建立项目。
- 可复用实现留在 Expo 路由文件内。
- 当 prebuild 拥有它们时编辑生成的 `ios/`/`android/` 文件。
- 别名仅在 TypeScript 中配置。
- 在无 plugin/重建/平台兼容性检查的情况下添加原生依赖。
- 从示例复制的应用标识符、scheme、权限或环境主机。
