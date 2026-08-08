# Vue 混合、Native Shell 与 PWA 交付

仅当已接受的技术栈包含 Quasar、Capacitor 或 PWA 能力且任务显式拥有移动/原生/离线/平台行为时应用此参考。不要为普通 Vue Web 任务添加混合基础设施。

## 选择运行时

标识交付物是响应式 Web、可安装 PWA、Quasar SPA/PWA/Capacitor、普通 Vue 加 Capacitor、Electron/BEX 还是多个显式模式。每种模式有不同的配置、资源、路由、存储、权限、更新和部署行为。

保留仓库框架/模式。不要仅为移动外观布局引入 Quasar 或 Capacitor，不要假设 PWA service worker 满足原生需求。

在可行处保持共享产品组件独立于 shell API；通过类型化适配器/composable 暴露原生/PWA 能力。

## Quasar 边界

为应用级 client/plugin 使用 boot 文件，保持其服务端/客户端/模式守卫显式。仅注册产品所需的框架 plugin/组件；全面导入增加包并隐藏依赖。

尊重 Quasar 布局/页面容器所有权、屏幕工具、暗/主题令牌、对话框/通知/加载生命周期和可访问标签。全局加载/通知必须在每个结果上关闭，不得替代上下文错误。

构建模式、router 模式/base、公共路径、图标集、extras 和环境配置必须匹配部署/原生打包。

## Capacitor 配置

将 app ID/name、`webDir`、服务端设置、scheme、plugin、权限、平台项目、签名和同步视为影响发布的。永远不要在生产中发布开发 LAN URL、明文覆盖或实时重载服务端配置。

Web 构建/配置/plugin 变更后，使用仓库同步/复制/原生构建流程。TypeScript 导入不能证明 iOS/Android 上的 plugin 安装。

用运行时/能力检查保护 plugin 使用，并建模已授予、已拒绝、已阻止、不可用、已中断和原生错误状态。在支持 Web 模式处提供浏览器回退或清晰的不可用状态。

移除 app/plugin 监听器，在拥有时处理前台/后台、进程重建和恢复的导航/状态。

## PWA Manifest 与安装

保持 manifest ID/scope/start URL/display/orientation/theme/icons 与 router base 和部署路径对齐。仅在浏览器暴露且产品有有意义触发器时使用安装提示。

建模安装可用/已忽略/已安装、更新可用/稍后应用、离线/已重新连接和不支持状态，不阻止普通浏览器使用。

不要从 PWA 安装声称 iOS/Android 原生对等；平台能力和更新生命周期不同。

## Service Worker 缓存

按资源/资源选择策略：预缓存版本化应用壳、cache-first 不可变资源、network-first 有界公共数据，对敏感变更/认证响应使用 network-only，除非显式离线架构另有说明。

永远不要在共享键下缓存登录/令牌响应、个性化可变 API 或错误响应。包含方法、URL/查询、标识范围、新鲜度、大小/计数/过期和失效维度。

版本化/迁移离线队列并定义幂等性、冲突、重试/退避、排序、墓碑和用户/账户清理。后台同步不是服务端并发策略的替代。

协调 service-worker 激活/更新，使旧页面不会静默地与不兼容的资源/API schema 配对。提供受控的重载/应用路径。

## 移动 UI 与可访问性

为密度、安全区域、触摸目标、键盘、视口单位、standalone 模式、减少动画和离线反馈使用 UIX 响应式/移动规则。不要仅按 user-agent 分叉业务工作流。

原生对话框、通知、触觉、分享、相机、地理定位、文件和推送集成需要权限/能力/隐私感知行为和平台证据。

## Verification

- 运行每个受影响的 Quasar/PWA/Capacitor 构建模式和原生同步/配置边界。
- 验证生产配置排除开发服务端 URL/明文允许并解析正确的 `webDir`/base/scope/资源。
- 练习不支持、权限拒绝/阻止、plugin 失败、后台/前台和监听器清理。
- 测试 service-worker 安装/更新/离线/重新连接/缓存过期并确保 auth/变更/个性化数据未被不安全缓存。
- 在可用平台上验证受影响的原生行为并准确记录不可用平台风险。

## 交付证据

命名运行时模式、平台适配器/plugin、manifest/缓存/更新策略和实际 Web/原生证据。响应式浏览器页面或成功 Web 构建不能证明原生 plugin 安装、service-worker 安全、离线冲突行为或平台对等。

## 不安全默认行为

- 因为任务提及移动布局而添加 Quasar/Capacitor/PWA。
- Capacitor 生产配置保留 LAN URL 或明文模式。
- 仅在包导入后认为原生 plugin 已安装。
- 对认证或可变 API 数据应用 cache-first 策略。
- 无幂等性/冲突/账户清理的离线队列。
- Service-worker 更新强制不受控的工作流丢失。
- 从浏览器/PWA 证据声称原生对等。
