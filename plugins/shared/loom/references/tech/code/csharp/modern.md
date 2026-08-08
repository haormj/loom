# C# 语言版本特性

## When To Use

仅当任务显式拥有 C# 语言版本 API 或变更 `LangVersion`/目标框架行为时才使用此参考。不要仅因为 SDK 能编译较新语法就现代化稳定代码。

## Implementation Focus

### Compatibility Boundary

确认 SDK、目标框架、`LangVersion`、编译器/分析器、运行时/BCL、源码生成器、测试/构建代理、消费者和部署镜像支持。C# 语法和 .NET 运行时/库特性有各自的兼容性。

除非需要显式版本否则优先使用项目的 SDK/目标默认值。语言升级包括项目/CI/工具/消费者变更和回退/迁移证据。

没有已接受的预览策略和固定 toolchain 就不要使用预览特性。

### Records, Required Members, And Primary Constructors

当相等/解构/不可变性适合时使用 record/值语义。在选择 record 之前审查数组/集合、可变属性、EF/代理/序列化器行为和继承的相等性。

`required` 传达对象初始化义务但不验证反序列化/外部值。保持构造函数/工厂/验证器负责运行时不变式。

主构造函数参数是参数，不自动成为字段/属性。避免含糊地捕获可变/可销毁依赖或使 API 兼容性不太清晰。

### Pattern Matching

当属性/列表/关系/类型模式和 switch 表达式能使封闭决策逻辑穷尽且可读时使用它们。

处理 null、守卫、重叠情况、求值顺序和未来枚举/派生类型。不要在模式/守卫中隐藏副作用或昂贵调用。

对于领域状态，优先使用类型化封闭模型和穷尽匹配而非字符串和静默接受未知情况的默认分支。

### Collection Expressions And Spreads

当目标类型和分配语义清晰时使用集合表达式。了解目标是数组/列表/span/不可变/接口以及展开是枚举一次还是分配/拷贝。

在重载解析、构建器行为、延迟枚举或所有权变得不太明显的地方不要使用它们。在旧编译器上为公共示例/消费者保留仓库风格。

### Ref Structs, Spans, And Scoped Lifetimes

仅在已验证的同步生命周期边界内使用 `Span<T>`、`ReadOnlySpan<T>`、`ref struct`、`scoped`、`ref` 返回和栈分配。

类引用值不能跨越 await/yield、普通装箱/接口/堆捕获或比后端存储活得更久。避免返回指向栈/本地/池缓冲区的 span 或在没有显式所有者生命周期的情况下使用 `Memory<T>`。

保持 unsafe/ref 特性隔离并配以边界/生命周期测试和性能证据。

### Generic Math And Static Abstract Members

为具有清晰约束和受支持目标框架的真实数值/泛型消费者使用泛型数学/静态抽象接口成员。

定义溢出、检查上下文、转换、浮点 NaN/无穷和语义操作期望。避免为一个数值类型创建泛型层。

### Interpolated Strings And Raw Literals

原始字符串改善嵌入文本可读性但不使 SQL/HTML/JSON/shell 内容安全。对不可信数据使用参数化/编码/序列化而非插值。

自定义插值字符串处理器是高级性能/API 特性，需要调用者语义、条件求值和基准证明。

### Source Generation, AOT, And Trimming

当目标运行时、反射/修剪/启动/构建需求证明且仓库工具支持生成输出时，使用源码生成的 JSON/regex/日志/DI 或自定义生成器。

生成器需要确定性增量输入、诊断、命名空间/碰撞策略、分析器包行为和消费者/构建测试。生成源码不是隐藏业务逻辑的地方。

Native AOT/修剪变更需要反射/动态加载/序列化/插件兼容性和发布时证据，而非仅普通 `dotnet build`。

## Verification Focus

- 用确切的 SDK/TFM/LangVersion 和受影响的消费者/工具矩阵构建。
- 在使用时测试 record 相等性、required 运行时验证、模式穷尽性、集合分配/重载和 span/ref 生命周期行为。
- 在拥有时运行发布/修剪/AOT 和源码生成器消费者测试。
- 为多目标项目和旧消费者验证回退/兼容性。
- 对任何性能动机的语言特性与清晰代码对比测量。

## Evidence Focus

说明特性、SDK/TFM/语言支持、运行时/生命周期/序列化语义、兼容性边界和聚焦行为/发布证明。本地编译的新语法不是生产兼容性证据。

## Unsafe Defaults

- 没有声明 toolchain 所有权就引入 C# 12/预览语法。
- 尽管可变标识或不兼容的序列化器/ORM 语义仍选择 record。
- `required` 被视为运行时输入验证。
- Span/ref struct 逃逸后端生命周期或异步边界。
- 仅从普通构建声称源码生成/AOT 完成。
- 原始/插值字符串用作注入保护。
