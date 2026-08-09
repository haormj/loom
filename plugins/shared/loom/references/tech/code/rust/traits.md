# Rust Trait 质量

## When To Use

- 任务变更了 trait、泛型约束、关联类型、trait 对象、derive 宏、转换 trait、扩展 trait、标记 trait 或基于 trait 的依赖边界。
- 当 trait 设计影响公共 API 清晰度、分派、测试接缝或编译时保证时使用此参考。
- 如果具体类型或简单函数足够，不要添加 trait 层。

## Implementation Focus

- 保持 trait 小且聚焦于能力。避免镜像一个具体类型完整方法集的宽泛 trait。
- 当每个实现有一个自然的相关类型时使用关联类型；当调用者可能在一个实现中组合多个具体类型时使用泛型参数。
- 对热的泛型代码优先使用静态分派，对异构集合、插件边界或运行时灵活性使用 trait 对象。当影响 API 形态时使此选择显式。
- 确保打算作为对象的 trait 是对象安全的。不要为需要 `dyn Trait` 的 trait 添加泛型方法或按值的 `self` 方法。
- 当语义匹配时使用标准 trait（`From`、`TryFrom`、`AsRef`、`Borrow`、`Iterator`、`Display`、`Debug`、`Error`、`Serialize`）；不要发明平行的转换/显示 API。
- 当行为纯粹是结构性的时派生标准 trait。仅在语义与逐字段行为不同时手动实现。
- 当外部实现会违反不变式或使未来演化不安全时使用密封 trait。
- 保持扩展 trait 的范围和命名以避免令人惊讶的方法污染。不要为一个本地调用点添加扩展 trait。
- 仅对真正的编译时保证使用标记 trait，并为 unsafe 标记 trait 记录安全/不变式要求。
- 在跨 crate 边界设计 impl 时注意 coherence/orphan 规则；不要用过于宽泛的 blanket impl 将 crate 逼入 API 角落。

## Decision Rules

- 为真正的接缝使用小型能力 trait。镜像一个具体类型或组合不相关职责的 trait 会创造间接性而无替代性。
- 当每个实现有一个自然的相关类型时选择关联类型；当调用者需要组合多个具体类型时使用泛型参数。当影响对象安全或推断时记录选择。
- 对热的、封闭的泛型代码优先使用静态分派，对异构集合或运行时插件边界使用 `dyn Trait`。在暴露 trait 对象之前验证对象安全。
- 当 `From`/`TryFrom`、`AsRef`、`Borrow`、`Iterator`、`Display`、`Error` 和仓库标准的 derive trait 的语义匹配时使用它们。不要发明平行的转换或格式化契约。
- 当外部实现可能违反不变式或使未来演化不安全时密封 trait。保持扩展 trait 命名精准以避免令人惊讶的方法污染。
- 在 crate 边界尊重 coherence 和 orphan 规则；避免使下游实现不可能的宽泛 blanket 实现。

## Verification Focus

- 构建通过预期边界使用 trait 的示例或测试：泛型静态分派、trait 对象、转换或扩展方法。
- 当 trait 旨在抽象多个实现时，测试至少两个有意义的实现者。
- 使用 `dyn Trait` 时确认对象安全。
- 对于公共 trait，包含文档或示例展示必需的不变式和预期实现者行为。

## Evidence Focus

- 在证据总结中，说明 trait 决策：关联类型、泛型约束、trait 对象、derive/手动 impl、转换 trait、密封 trait、扩展 trait、标记 trait 或 coherence 边界。
