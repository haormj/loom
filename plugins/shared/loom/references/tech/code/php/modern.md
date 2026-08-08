# 现代 PHP 特性质量

## When To Use

- 任务有意采用或变更 PHP 语言特性如严格类型、enum、readonly class、属性、一等可调用、`match`、`never`、Fiber 或类型化属性。
- 仅为拥有语言版本或特性决策的任务加载此参考。核心 PHP 指导仍是普通应用代码的基线。

## Implementation Focus

- 在使用特性之前确认仓库的 PHP 版本、Composer 平台约束、运行时镜像和受支持的静态分析版本。不要仅从本地解释器推断支持。
- 当仓库使用时一致地为新 PHP 文件添加 `declare(strict_types=1);`，并在调用严格 API 之前验证跨越弱类型边界的数据。
- 对具有稳定存储或传输表示的有限值使用 backed enum。定义未知值行为并在契约要求 enum 值时避免序列化 enum 标签。
- 对不可变 DTO/值对象使用 readonly class 或 property。在未验证 hydration、变更和序列化行为之前不要将 Doctrine/Eloquent 实体或代理管理的框架对象标记为 readonly。
- 仅在活动框架读取属性的地方使用属性。将路由、验证、序列化和 DI 元数据保留在一个权威表示中。
- 当一等可调用和 `match` 能使分派或穷尽分支更清晰时使用它们。在外部边界保持异常和默认/未知情况显式。
- 仅对真正不能返回的函数使用 `never`，如类型化终止器或异常边界。不要用它隐藏不完整的结果路径。
- 将 Fiber 视为低级原语。为调度、I/O、取消和生命周期加载异步参考；此参考单独不使代码并发。

## Verification Focus

- 为变更模块运行仓库配置的 PHP 测试和静态分析命令，加上选中 PHP 版本的最窄运行时检查。
- 在涉及时验证 enum 持久化/序列化、readonly hydration、属性发现、可调用分派、穷尽 `match` 和未知输入行为。
- 检查 Composer 自动加载和受支持的运行时/容器路径，而非仅用较新的本地 PHP 二进制文件进行语法解析。

## Evidence Focus

- 在证据总结中，说明特性决策、受支持的 PHP/Composer 约束、集成边界和已测试行为。

## Failure Modes

- 不要将 PHP 8.3 语法复制到 Composer/运行时约束较旧的项目中。
- 不要将 readonly、属性、enum 或 Fiber 用作装饰；每个都必须解决拥有的契约或生命周期问题。
- 不要将 PHPStan/Psalm 抑制、`mixed` 或通过的语法检查视为运行时兼容性的证明。
