# Vue 类型契约

当已接受的 Vue 技术栈使用 TypeScript 且任务拥有 SFC props/emits/model/slot、composable/store 契约、template ref、注入、指令、plugin 或模块扩充时应用此参考。不要强制 JavaScript 项目迁移。

## SFC Compiler 契约

保持 `vue`、`@vue/compiler-sfc`、语言工具、`vue-tsc`、TypeScript、打包器插件和测试转换兼容。宏/类型行为跨 Vue 版本不同。

使用仓库的 `<script setup lang="ts">` 或约定式 `defineComponent` 风格。不要仅为使用外部代码片段而混合风格。

运行 `vue-tsc`，因为仅 `tsc` 不能完全检查 template/SFC 契约。在使用处保留项目引用和生成声明工作流。

## Props、Emits、Model 与 Slot

按仓库和运行时验证需求通过类型或运行时声明定义 props。保持可选与可空精确，对可变值安全使用 `withDefaults`。

为 emit 使用命名元组/调用签名和稳定的领域载荷类型。避免在命令边界使用 `any`、宽泛的 `Record<string, unknown>` 或未检查的强制转换。

仅当已安装的 Vue/compiler 支持时使用 `defineModel`。类型化多个命名 model/修饰符并保留无效中间表单值。

当 slot 的公共 prop 重要时用支持的 SFC 宏/工具类型化；否则保持 slot 用法具体并通过 `vue-tsc` 验证消费者。

## 响应式类型

显式类型化可空/可替换 ref。在可能时让 `reactive` 推断内聚对象状态，除非公共泛型契约需要，否则不应用 `UnwrapNestedRefs` 风格复杂性。

仅当调用者真正传递/响应那些形式时在 composable 契约中使用 `Ref`、`ComputedRef`、`MaybeRefOrGetter` 或本地等价物。在边界规范化，在不允许修改时返回只读 ref。

避免在运行时验证之前声称 template ref、注入依赖、API 结果或路由参数存在的类型断言。

## Template Ref 与组件

Template ref 初始为 null 且可返回 null。按元素类型化 DOM ref，通过公共暴露 API（`defineExpose`/组件实例类型化）类型化组件 ref，而非深入内部。

对于重复 ref、动态组件和泛型组件，遵循已安装的 Vue 语言工具支持，在类型无法证明具体组件处保留运行时标识检查。

当一个类型化命令或 DOM ref 足够时不要暴露整个组件实例。

## Provide、指令与 Plugin

使用 `InjectionKey<T>` 并显式处理缺失的必需 provider。提供默认对象可能掩盖安装/配置缺陷。

按元素/绑定/值/参数/修饰符行为类型化自定义指令，在指令生命周期 hook 中清理资源。

在正确的 Vue/Nuxt 模块中用模块扩充声明 app 全局属性和 plugin 注入。在类型中反映服务端/客户端可用性和可选安装。

## Router 与外部数据

通过仓库 router 工具类型化路由名称/参数，但仍验证运行时 URL 值。静态类型不保护直接外部导航。

在运行时解析 API/存储/原生载荷并映射到受信任的内部/视图类型。在可用时生成的 client 或 schema 优于手工维护的重复接口。

在防止错误目标或不完整状态处理处保留品牌 ID、可辨识联合、精确状态/错误变体和版本字段。

## 泛型组件与 Composable

当一个真正可复用的集合/字段/选择器契约跨类型工作且保留推断时使用泛型。避免在每次模板使用时都需要强制转换的泛型抽象。

约束键/回调并显式携带稳定标识。泛型列表仍不能对可变行使用索引标识。

## Verification

- 公共类型变更后运行仓库 `vue-tsc`/SFC 构建加聚焦消费者。
- 在支持处为可复用泛型、slot、注入、plugin 或 model 契约包含编译 fixture/测试。
- 练习静态类型无法验证的路由/API/存储/原生值的运行时解析。
- 通过实际挂载/卸载/条件行为验证可空 ref 和可选 provider。
- 当库边界变更时确保生成声明和包导出保持可消费。

## 交付证据

命名 SFC/公共类型边界、运行时验证边界和消费者/构建证明。用强制转换移除类型错误或编译一个组件不能建立 template、plugin、生成声明或外部数据安全。

## 不安全默认行为

- 强制将 TypeScript 迁移引入已接受的 JavaScript Vue 项目。
- `tsc` 成功用作唯一的 SFC/template 证明。
- 在 props、emit、params、inject 或 API 边界使用 `any`/强制转换。
- 将 template/组件 ref 视为始终已初始化。
- 在无已安装 compiler 支持的情况下使用宏 API。
- 在推断差和广泛强制转换的情况下引入泛型组件。
