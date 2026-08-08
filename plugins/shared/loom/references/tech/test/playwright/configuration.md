# Playwright 项目配置

配置应使分配的检查在本地和 CI 中可复现，同时保留仓库现有的脚本、服务端、包管理器和测试布局。

## 创建前先适配

首先检查所选项目 runner 事实：

- 包根和包管理器；
- 依赖和已解析版本；
- 现有 Playwright 配置；
- 测试根和脚本；
- 应用启动/预览命令；
- monorepo 工作区过滤器；
- 当前 CI 产物约定。

扩展现有配置。不要用通用完整矩阵替换它、重命名已建立项目或仅为匹配示例而移动测试根。

## 最小新配置

对于新浏览器套件，从一个 Chromium 项目和 profile 所需的视口开始。仅在产品/浏览器支持需求要求时添加 Firefox、WebKit、设备、区域设置或配色方案。

```typescript
import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  fullyParallel: true,
  forbidOnly: Boolean(process.env.CI),
  retries: process.env.CI ? 2 : 0,
  reporter: process.env.CI
    ? [['line'], ['html', { open: 'never' }]]
    : [['list'], ['html', { open: 'never' }]],
  outputDir: 'test-results',
  use: {
    baseURL: process.env.E2E_BASE_URL ?? 'http://127.0.0.1:4173',
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
  },
  projects: [
    {
      name: 'desktop-primary',
      use: { ...devices['Desktop Chrome'] },
    },
    {
      name: 'mobile-primary',
      use: { ...devices['Pixel 7'] },
    },
  ],
  webServer: process.env.E2E_EXTERNAL_SERVER
    ? undefined
    : {
        command: 'npm run preview -- --host 127.0.0.1',
        url: 'http://127.0.0.1:4173',
        reuseExistingServer: !process.env.CI,
        timeout: 120_000,
      },
});
```

当未分配响应式覆盖时移除 `mobile-primary`。用项目事实替换命令和端口；示例不是默认值。

## 视口映射

将 profile 视口引用映射到命名项目或逐测试 `test.use()` 值：

| Profile 引用 | 配置意图 |
| --- | --- |
| `desktop_primary` | 支持的桌面浏览器和生产工作面宽度 |
| `mobile_primary` | 支持的窄/触摸视口，无 hover 依赖 |

保持视口名称稳定，使验证证据标识运行了什么。不要模拟品牌设备，除非设备特定行为重要；视口和输入模式通常足以用于响应式 Web 检查。

## 服务端与 Base URL

- 当构建输出是检查的一部分时，优先使用仓库的类生产预览/启动命令而非开发服务端。
- 为 Playwright 可以可靠启动和停止的单个项目拥有进程使用 `webServer`。
- 对于组合的前端/后端系统，使用项目的编排命令或在 fixture 中启动受管进程；不要在一个 shell 字符串中隐藏多个不受管前台服务端。
- 除非需要外部访问，否则将测试服务端绑定到环回地址。
- 从环境/配置派生 base URL，避免硬编码被占用端口。
- 就绪 URL 必须证明应用已为分配的检查足够就绪，而非仅进程打开了套接字。

## 共享浏览器运行时

项目拥有 `@playwright/test`、配置、脚本和锁文件。MCP 从已安装依赖或锁文件解析精确项目版本，仅准备所选浏览器，并将匹配的执行环境附加到闭环请求。

- 主机缓存按 OS/CPU 隔离，以精确 Playwright 版本加浏览器集为键。永远不要自己构造或持久化缓存路径。
- `partial` 运行时矩阵可运行：选择匹配当前项目 runner 的环境，仅将绑定到不可用目标的检查标记为 blocked。不要将不相关的工作区版本视为全局浏览器中断。
- 对于主机后端，将返回的 `browserEnvironment` 应用于项目本地 runner。
- 当主机启动失败时，Loom 可能返回带有精确版本镜像、命令前缀、挂载路径和浏览器环境的受管容器后端。运行该描述符；不要发明另一个镜像或在项目中运行 `playwright install --with-deps`。
- 当 `projectRunner` 不存在且闭环被授权建立 Playwright 时，在运行时提供的精确 `resolvedVersion` 处添加首个项目拥有的 `@playwright/test` 依赖，创建最小配置/测试根，并更新项目锁文件。运行时准备后不要选择不同的范围或最新版本。
- 如果预览/API 在主机上运行，仅用返回的 `hostGateway` 替换环回主机名并保留发现的端口。在现有容器网络中运行的 service 需要该网络的受支持地址。
- 保持项目 runner 版本与准备的浏览器修订对齐；当执行请求标识过期的包事实时恢复项目依赖。
- 不要提交共享缓存路径、下载的浏览器二进制文件或机器特定的绝对路径。
- 不要将 Loom 的共享 runner `node_modules` 复制到仓库中。共享 runner 是准备/doctor 资产；项目测试仍使用项目拥有的包和配置。

## 产物与报告器

- 保持 `outputDir`、HTML 报告、JUnit 和 blob 报告路径可预测且被忽略，除非仓库有意存储基线。
- 在首次重试时使用 trace 或失败时保留；始终开启的 trace 昂贵且可能暴露敏感数据。
- 按项目策略在失败时保留截图/视频。
- CI 上传应在测试失败时也运行并使用有界保留。
- 不要将凭据、授权头或完整的机密载荷输出到报告中。

## 超时与重试

- 保持操作/断言超时接近预期 UI 延迟。
- 给服务端启动单独的超时；不要因为编译慢而膨胀所有断言。
- 本地重试通常应为零，使不稳定性可见。
- CI 重试可捕获诊断，但重试成功仍是可见证据，重复重试需要修复。
- 不要全局将 worker 设置为 1，除非共享基础设施确实无法隔离状态。

## 认证项目

当多个套件共享角色特定存储状态时使用 setup 项目和依赖。将状态文件保留在忽略的输出中并使角色名称显式。认证本身的测试不得依赖预认证状态。

通过 setup 项目刷新过期状态，将凭据保留在环境/密钥存储中，不要让管理员状态泄漏到低权限项目中。

## Monorepo

- 将配置和测试放在拥有浏览器应用的包根处，除非仓库有中心 E2E 工作区。
- 通过包管理器的工作区/过滤语法调用。
- 从该包根解析 Web 服务端 cwd、构建输出、环境文件和产物路径。
- 不要假设仓库根包含前端 manifest。

## 配置验证

运行完整检查之前，验证 Playwright 可以列出所选测试/项目，且配置解析指向预期的 base URL、测试根和输出目录。发现零测试的配置不是通过。

还要验证所选包脚本退出、产物路径可写/被忽略，且失败的检查保留配置的诊断产物而非在自动化中打开交互式报告。
