# 部署 Bootstrap 参考

当部署诊断提到缺失数据表、待执行迁移、schema 设置、Prisma、Django、Rails、Laravel、Flyway、Liquibase 或 `loom.deployBootstrap` 时，使用本参考文档。

## 检测

Bootstrap 任务是记录在 `DeploymentSpec.bootstrap.tasks` 中的建议性诊断。
此列表记录了 Loom 部署 bootstrap 扫描器当前输出的任务类型，并非通用迁移目录。

检测到的任务类型：

- Prisma：当 `prisma/schema.prisma` 存在或 package 脚本中包含 Prisma 迁移命令时触发。命令根据检测到的包管理器选择：`npx prisma migrate deploy`、`pnpm exec prisma migrate deploy`、`yarn prisma migrate deploy` 或 `bunx prisma migrate deploy`。
- Django：当检测到 `manage.py` 时触发。命令：`python manage.py migrate --noinput`。
- Rails：当检测到 `db/migrate` 时触发。命令：`bundle exec rails db:migrate`。
- Laravel：当检测到 `database/migrations` 时触发。命令：`php artisan migrate --force`。
- Flyway：当检测到 `flyway.conf`、`flyway.toml` 或 `src/main/resources/db/migration` 时触发。Java 项目可能使用 Maven 或 Gradle Flyway 插件命令；否则命令为 `flyway migrate`。
- Liquibase：当检测到 `liquibase.properties`、`liquibase.yml`、`liquibase.yaml` 或 `src/main/resources/db/changelog` 时触发。Java 项目可能使用 Maven 或 Gradle Liquibase 插件命令；否则命令为 `liquibase update`。

Agent 边界：

- 不要发明 `DeploymentSpec.bootstrap.tasks` 中未出现的 bootstrap 命令。
- 如果仓库使用此处未列出的迁移系统，视为当前扫描器不支持，除非 Loom 已为其输出任务。
- 修复可以在 MCP 代码中改进扫描器支持，但部署执行只能运行已声明的任务。

## 执行契约

- `loom.deployBootstrap` 携带 `confirm: false` 时预览检测到的任务并返回用户门控。不得执行命令。
- `loom.deployBootstrap` 携带 `confirm: true` 时只能执行 `DeploymentSpec.bootstrap.tasks` 中的任务。
- MCP 工具在活跃 Compose 主应用服务内通过 `docker compose exec -T <service> sh -lc <command>` 执行已确认的任务。
- 部署必须已经在运行。如果 Compose 服务未运行，在重试 bootstrap 之前使用 `loom.deployUp`。
- 当检测到多个任务时，如果用户批准了特定的迁移系统，则传递 `kind`。
- 在第一个失败的 bootstrap 命令后停止。报告任务类型、命令、Compose 路径、服务 id、退出码以及工具返回的 stdout/stderr 尾部。

## Agent 边界

- 不要在本地 shell 中手动运行迁移命令。
- 不要为了使 bootstrap 命令运行而编辑生成的 Compose 或 Dockerfile 资产。
- 不要发明额外的 bootstrap 任务。只使用 Loom 声明的任务。
- 如果工具报告服务未运行，按照推荐的 Loom 部署操作继续，而不是修补文件。

## 安全

- 将迁移视为针对本地 Compose 依赖服务的有状态操作。
- 不要自动运行破坏性的 reset/seed/drop 命令。
- 不要读取或注入真实的 `.env` 值。使用 Compose 中已有的生成本地依赖环境变量。
- 如果 bootstrap 需要凭证或私有网络访问，请向用户请求安全的本地配置，而不是发明值。
