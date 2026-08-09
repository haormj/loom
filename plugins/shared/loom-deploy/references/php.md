# PHP 部署参考

当实现或修复 PHP 家族项目的 loom deploy 支持时，使用本参考文档。

## 扫描器信号

- `composer.json` 标识 PHP 项目，应在 `package.json` 之前检查，因为 Laravel 项目通常包含前端资产。
- `laravel/framework` 或 `artisan` 信号标识 Laravel。
- `symfony/framework-bundle` 或 `symfony/runtime` 信号标识 Symfony。
- `slim/slim` 信号标识 Slim。
- `composer.lock` 是 Composer lockfile。
- `composer.json` `require.php` 应在可能时指导 PHP 次版本。默认 PHP 版本为 8.3。

## 模板规则

- v1 使用单容器本地预览模板。
- 使用 `php:<minor>-cli` 加 Composer 进行确定性本地预览。
- 安装 web 应用所需的常见扩展：`pdo`、`pdo_mysql`、`pdo_pgsql` 和 `zip`。
- 在源文件之前复制 `composer.json` 和 `composer.lock`，然后运行 Composer install。
- 对于 Laravel，创建 `storage` 和 `bootstrap/cache`，然后运行 `php artisan package:discover --ansi || true`。
- Laravel 本地预览使用 `php artisan serve --host=0.0.0.0 --port=${PORT:-8000}`。
- 对于通用 PHP，使用内置服务器和 `public/index.php`。

## 依赖服务

- 从 `pdo_mysql`、`mysqli`、`mysql` 或 Laravel 数据库配置检测 MySQL/MariaDB。
- 从 `pdo_pgsql`、`pgsql` 或 postgres 连接字符串检测 Postgres。
- 从 `predis`、`phpredis` 或 Redis 连接字符串检测 Redis。
- 从 Composer 包名和 env/config 信号检测 RabbitMQ、Elasticsearch、MongoDB 和 S3-compatible 服务。

## 修复说明

- 如果 Composer install 因缺失 PHP 扩展失败，在编辑应用代码之前更新生成的 Dockerfile 扩展安装块。
- 如果 Laravel 启动但返回 500，检查日志中缺失的 `APP_KEY`、`storage` 写权限、数据库迁移失败或缺失 env 值。
- 默认不要将真实 `.env` 文件复制到生成的镜像中；使用 `.env.example` 推断所需变量。
- 对于生产级 PHP 部署，未来的提供者可能使用 Nginx + PHP-FPM，但 v1 Dockerfile 模板有意设计为本地预览路径。

## 扫描器信号到部署事实

在生成文件之前，将 PHP 扫描器证据转换为部署事实：

- `composer.json` 路径成为服务根、manifest ref 和 Composer install 事实。
- `composer.lock` 成为 lockfile ref。
- Laravel `artisan`、Symfony runtime/config、Slim 路由设置或 `public/index.php` 决定框架/runtime 事实。
- `composer.json` `require.php` 在可能时选择 PHP 镜像版本。
- `public/` document root、`artisan serve` 和框架 router 信号成为 preview/start 命令事实。
- `.env.example`、配置文件、DB 驱动、queue/cache 包和存储路径成为环境/依赖事实。
- Laravel/Symfony 内的前端资产 `package.json` 不覆盖 PHP 应用角色。

## 生成的资产预期

生成的 PHP 资产应显示：

- 当 lockfile/manifest 允许层缓存时，在源复制之前进行 Composer install。
- 根据依赖事实安装所需的 PHP 扩展，而非硬编码的数据库猜测。
- Laravel 本地预览包含生成的 `APP_KEY`、可写的 `storage` 和 `bootstrap/cache` 以及容器安全的端口。
- 通用 PHP/Slim/Symfony preview 服务于检测到的 public document root。
- 依赖 URL/config 指向 Compose 服务 DNS 名和生成的本地凭证。
- 真实 `.env` 值从不复制到镜像中。

## 修复边界

在以下情况下修复生成的 PHP 部署资产：

- Composer install 因生成扩展包不完整而失败。
- Runtime 命令服务了错误的 document root 或绑定了错误的主机/端口。
- Laravel/Symfony 本地密钥/cache/storage 默认值在生成的 env 或目录中缺失。
- 依赖配置在容器内使用了 localhost。

在部署资产修复期间不要编辑 PHP 应用配置、迁移、Composer 依赖或真实 env 文件，除非 MCP 操作路由到执行修复。
