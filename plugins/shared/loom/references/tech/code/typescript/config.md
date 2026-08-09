# TypeScript 配置质量

## When To Use

- 仅当任务变更 `tsconfig` 文件、包构建设置、模块解析、声明、项目引用、别名、框架集成或编译器严格性时才加载。
- 不要将编辑配置作为绕过应用类型错误的捷径；修复源码或将配置变更缩小到拥有的包。
- 在变更编译器选项之前检查 `package.json`、包管理器脚本、bundler、测试运行器和现有配置继承。

## Decision Rules

- 保留 `strict`、`strictNullChecks`、`noImplicitAny`、`noUncheckedIndexedAccess`、`exactOptionalPropertyTypes` 和相关安全标志。构建修复不得静默削弱它们。
- 将模块解析与运行时匹配：bundler 应用使用面向 bundler 的解析，Node 库和 CLI 使用 `NodeNext` 兼容设置。
- 仅在 TypeScript、bundler、测试运行器、lint 和运行时都解析相同的映射时才添加路径别名。一个缩短的导入不足以作为理由。
- 仅对发出单独产物的真实包或层边界使用项目引用；对齐 `composite`、`rootDir`、`outDir` 和声明。
- 为库、SDK、共享包和插件 API 生成声明。仅应用包不需要它们，除非仓库消费它们。
- 保持 `include` 和 `exclude` 诚实。永远不要排除源码、测试或生成的契约来隐藏诊断。
- 保留仓库的 `skipLibCheck` 策略，永远不要用它来隐藏本地源码或本地声明失败。

## Implementation Focus

- 将公共编译器规则放在现有基础配置中，将环境特定的 `lib`、`jsx`、`types` 和 emit 选项放在包配置中。
- 如果启用了更严格的标志，在同一任务中修复其受影响的源码，或将标志范围限定到目标包。
- 对 `isolatedModules`，避免命名空间密集或不安全的单文件转译模式，并验证实际框架构建。
- 仅在仓库有可复现的缓存位置和干净构建路径时才使用增量编译或构建元数据。

## Failure Modes

- 不要在不检查 bundler、测试运行器和运行时的情况下变更 `module`、`moduleResolution`、JSX 或别名。
- 不要通过从共享配置中排除测试或源文件来使本地包通过。
- 不要在仓库将生成的声明或构建输出视为派生产物时提交它们。

## Verification Focus

- 用 `tsc -p` 或 `tsc -b` 运行确切变更的配置，然后当别名、JSX、模块解析或发出输出可能影响运行时时运行框架构建。
- 如果声明输出变更，检查生成的公共路径并运行干净的包构建。
- 确认没有变更的配置静默地从类型检查中移除文件。

## Evidence Focus

- 记录配置决策：严格性、模块解析、别名的所有权、项目引用、声明输出、框架分层或文件范围。
