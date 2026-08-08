use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use contracts::{
    DeploymentBootstrapDiagnostics, DeploymentBootstrapTask, PackageManager, RuntimeKind,
};
use delivery_core::{
    LoomMcpActionResult, LoomMcpBlockedResult, LoomMcpDoneResult, LoomMcpUserGateResult,
};
use serde_json::json;
use state::paths::from_project_relative;

use crate::{code_evidence::DeploymentCodeProbe, prepare::read_spec, DeployBootstrapInput};

const BOOTSTRAP_MAX_DEPTH: usize = 8;
const BOOTSTRAP_FILE_LIMIT: usize = 300;
const BOOTSTRAP_IGNORED_DIRECTORIES: &[&str] = &[
    ".git",
    ".loom",
    "node_modules",
    "vendor",
    ".venv",
    "venv",
    "__pycache__",
    "target",
    "build",
    "dist",
    "coverage",
    "tmp",
    "log",
];

pub(crate) fn analyze_deployment_bootstrap(
    project_root: &Path,
    stack: &DeploymentCodeProbe,
) -> DeploymentBootstrapDiagnostics {
    let mut tasks = Vec::new();

    if let Some(root) = find_prisma_root(project_root) {
        push_task(
            &mut tasks,
            project_root,
            &root,
            "prisma",
            package_manager_exec(stack.package_manager, "prisma migrate deploy"),
            "检测到 Prisma schema；数据库可能需要在应用提供服务前执行迁移。",
        );
    }

    if let Some(root) = find_directory_with_file(project_root, "manage.py") {
        push_task(
            &mut tasks,
            project_root,
            &root,
            "django",
            "python manage.py migrate --noinput".to_string(),
            "检测到 Django manage.py；未执行的迁移可能在启动或首次请求时表现为缺表错误。",
        );
    }

    if let Some(root) = find_directory_with_path(project_root, "db/migrate") {
        push_task(
            &mut tasks,
            project_root,
            &root,
            "rails",
            "bundle exec rails db:migrate".to_string(),
            "检测到 Rails 迁移；未执行的迁移可能导致启动或请求失败。",
        );
    }

    if let Some(root) = find_directory_with_path(project_root, "database/migrations") {
        push_task(
            &mut tasks,
            project_root,
            &root,
            "laravel",
            "php artisan migrate --force".to_string(),
            "检测到 Laravel 迁移；未执行的迁移可能导致数据库/表错误。",
        );
    }

    if let Some(root) = find_flyway_root(project_root) {
        push_task(
            &mut tasks,
            project_root,
            &root,
            "flyway",
            flyway_command(stack),
            "检测到 Flyway 配置；schema 迁移可能需要在部署健康之前执行。",
        );
    }

    if let Some(root) = find_liquibase_root(project_root) {
        push_task(
            &mut tasks,
            project_root,
            &root,
            "liquibase",
            liquibase_command(stack),
            "检测到 Liquibase 配置；schema 迁移可能需要在部署健康之前执行。",
        );
    }

    let warnings = if tasks.is_empty() {
        vec![]
    } else {
        vec![
            "引导任务为诊断性质且需确认后执行；Loom 不会在 deploy prepare、deploy up 或 deploy run 期间自动运行迁移。"
                .to_string(),
        ]
    };

    DeploymentBootstrapDiagnostics { tasks, warnings }
}

pub fn deploy_bootstrap(input: DeployBootstrapInput) -> LoomMcpActionResult {
    let project_root_buf = PathBuf::from(&input.project_root);
    let project_root = project_root_buf.as_path();
    let spec = match read_spec(project_root) {
        Ok(spec) => spec,
        Err(error) => {
            return LoomMcpActionResult::Blocked(LoomMcpBlockedResult {
                project_root: input.project_root,
                blockers: vec![format!("部署未准备就绪，无法读取引导任务：{error}。")],
                recommended_tool: Some("loom.deployPrepare".to_string()),
                details: None,
            })
        }
    };
    let tasks = spec
        .bootstrap
        .tasks
        .iter()
        .filter(|task| {
            input
                .kind
                .as_ref()
                .map(|kind| task.kind == *kind)
                .unwrap_or(true)
        })
        .cloned()
        .collect::<Vec<_>>();

    if tasks.is_empty() {
        return LoomMcpActionResult::Done(LoomMcpDoneResult {
            project_root: input.project_root,
            summary: "部署引导没有匹配的任务可运行。".to_string(),
            details: Some(json!({
                "executed": [],
                "skipped": [],
                "kind": input.kind,
                "warnings": spec.bootstrap.warnings
            })),
            warnings: spec.bootstrap.warnings,
        });
    }

    if !input.confirm {
        return LoomMcpActionResult::UserGate(LoomMcpUserGateResult::new(
            input.project_root,
            "部署引导可能运行数据库迁移或种子命令。执行前请确认。",
            vec!["confirm".to_string()],
            None,
            None,
            None,
            Some(json!({
                "tool": "loom.deployBootstrap",
                "confirmRequired": true,
                "kind": input.kind,
                "tasks": tasks,
                "warnings": spec.bootstrap.warnings
            })),
        ));
    }

    let compose_file = match from_project_relative(project_root, &spec.files.compose_path) {
        Ok(path) => path,
        Err(error) => {
            return LoomMcpActionResult::Blocked(LoomMcpBlockedResult {
                project_root: input.project_root,
                blockers: vec![format!(
                    "部署引导无法解析 Compose 文件 {}：{error}。",
                    spec.files.compose_path
                )],
                recommended_tool: Some("loom.deployPrepare".to_string()),
                details: Some(json!({ "composePath": spec.files.compose_path })),
            });
        }
    };
    let service_id = spec.source_model.primary_service_id.clone();
    if let Some(blocked) = ensure_compose_service_running(
        &input.project_root,
        project_root,
        &compose_file,
        &spec.files.compose_path,
        &service_id,
    ) {
        return blocked;
    }

    let mut executed = Vec::new();
    for task in tasks {
        let output = compose_exec_command(&compose_file, &service_id, &task.command)
            .current_dir(project_root)
            .output();
        match output {
            Ok(output) => {
                executed.push(json!({
                    "kind": task.kind,
                    "command": task.command,
                    "composePath": spec.files.compose_path,
                    "serviceId": service_id,
                    "exitCode": output.status.code(),
                    "status": if output.status.success() { "passed" } else { "failed" },
                    "stdoutTail": tail_lines(&String::from_utf8_lossy(&output.stdout), 40),
                    "stderrTail": tail_lines(&String::from_utf8_lossy(&output.stderr), 40)
                }));
                if !output.status.success() {
                    return LoomMcpActionResult::Blocked(LoomMcpBlockedResult {
                        project_root: input.project_root,
                        blockers: vec![format!("部署引导任务 {} 失败。", task.kind)],
                        recommended_tool: Some("loom.deployInspect".to_string()),
                        details: Some(json!({ "executed": executed })),
                    });
                }
            }
            Err(error) => {
                executed.push(json!({
                    "kind": task.kind,
                    "command": task.command,
                    "composePath": spec.files.compose_path,
                    "serviceId": service_id,
                    "status": "failed_to_start",
                    "error": error.to_string()
                }));
                return LoomMcpActionResult::Blocked(LoomMcpBlockedResult {
                    project_root: input.project_root,
                    blockers: vec![format!("部署引导任务 {} 无法启动：{error}。", task.kind)],
                    recommended_tool: Some("loom.deployInspect".to_string()),
                    details: Some(json!({ "executed": executed })),
                });
            }
        }
    }

    LoomMcpActionResult::Done(LoomMcpDoneResult {
        project_root: input.project_root,
        summary: "部署引导已执行确认的任务。".to_string(),
        details: Some(json!({ "executed": executed })),
        warnings: spec.bootstrap.warnings,
    })
}

fn ensure_compose_service_running(
    project_root_display: &str,
    project_root: &Path,
    compose_file: &Path,
    compose_path: &str,
    service_id: &str,
) -> Option<LoomMcpActionResult> {
    let output = Command::new("docker")
        .args(["compose", "-f"])
        .arg(compose_file)
        .args(["ps", "--status", "running", "--services"])
        .arg(service_id)
        .current_dir(project_root)
        .output();
    match output {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let running = stdout.lines().any(|line| line.trim() == service_id);
            if running {
                return None;
            }
            Some(LoomMcpActionResult::Blocked(LoomMcpBlockedResult {
                project_root: project_root_display.to_string(),
                blockers: vec![format!("部署引导需要运行中的 Compose 服务 {service_id}。")],
                recommended_tool: Some("loom.deployUp".to_string()),
                details: Some(json!({
                    "composePath": compose_path,
                    "serviceId": service_id,
                    "status": "not_running",
                    "stdoutTail": tail_lines(&stdout, 40),
                    "stderrTail": tail_lines(&String::from_utf8_lossy(&output.stderr), 40)
                })),
            }))
        }
        Ok(output) => Some(LoomMcpActionResult::Blocked(LoomMcpBlockedResult {
            project_root: project_root_display.to_string(),
            blockers: vec![format!(
                "部署引导无法确认运行中的 Compose 服务 {service_id}。"
            )],
            recommended_tool: Some("loom.deployUp".to_string()),
            details: Some(json!({
                "composePath": compose_path,
                "serviceId": service_id,
                "exitCode": output.status.code(),
                "stdoutTail": tail_lines(&String::from_utf8_lossy(&output.stdout), 40),
                "stderrTail": tail_lines(&String::from_utf8_lossy(&output.stderr), 40)
            })),
        })),
        Err(error) => Some(LoomMcpActionResult::Blocked(LoomMcpBlockedResult {
            project_root: project_root_display.to_string(),
            blockers: vec![format!(
                "部署引导无法检查 Compose 服务 {service_id}：{error}。"
            )],
            recommended_tool: Some("loom.deployUp".to_string()),
            details: Some(json!({
                "composePath": compose_path,
                "serviceId": service_id,
                "error": error.to_string()
            })),
        })),
    }
}

fn compose_exec_command(compose_file: &Path, service_id: &str, command: &str) -> Command {
    let mut process = Command::new("docker");
    process
        .args(["compose", "-f"])
        .arg(compose_file)
        .args(["exec", "-T"])
        .arg(service_id)
        .args(["sh", "-lc"])
        .arg(command);
    process
}

fn find_prisma_root(root: &Path) -> Option<PathBuf> {
    find_directory_with_path(root, "prisma/schema.prisma")
        .or_else(|| find_package_script_directory(root, &["prisma migrate", "prisma db push"]))
}

fn find_flyway_root(root: &Path) -> Option<PathBuf> {
    find_directory_with_any_file(root, &["flyway.conf", "flyway.toml"])
        .or_else(|| find_directory_with_path(root, "src/main/resources/db/migration"))
}

fn find_liquibase_root(root: &Path) -> Option<PathBuf> {
    find_directory_with_any_file(
        root,
        &["liquibase.properties", "liquibase.yml", "liquibase.yaml"],
    )
    .or_else(|| find_directory_with_path(root, "src/main/resources/db/changelog"))
}

fn find_package_script_directory(root: &Path, needles: &[&str]) -> Option<PathBuf> {
    collect_files(root, BOOTSTRAP_FILE_LIMIT, |relative| {
        relative.ends_with("package.json")
    })
    .into_iter()
    .find_map(|package_path| {
        let text = fs::read_to_string(&package_path).ok()?;
        let scripts = serde_json::from_str::<serde_json::Value>(&text)
            .ok()
            .and_then(|value| value.get("scripts").cloned())
            .and_then(|value| value.as_object().cloned())?;
        let matched = scripts.values().any(|value| {
            value.as_str().is_some_and(|script| {
                let lower = script.to_ascii_lowercase();
                needles.iter().any(|needle| lower.contains(needle))
            })
        });
        matched.then(|| package_path.parent().unwrap_or(root).to_path_buf())
    })
}

fn find_directory_with_file(root: &Path, file_name: &str) -> Option<PathBuf> {
    collect_files(root, BOOTSTRAP_FILE_LIMIT, |relative| {
        Path::new(relative)
            .file_name()
            .and_then(|name| name.to_str())
            == Some(file_name)
    })
    .into_iter()
    .next()
    .and_then(|file| file.parent().map(Path::to_path_buf))
}

fn find_directory_with_any_file(root: &Path, file_names: &[&str]) -> Option<PathBuf> {
    let names = file_names.iter().copied().collect::<BTreeSet<_>>();
    collect_files(root, BOOTSTRAP_FILE_LIMIT, |relative| {
        Path::new(relative)
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| names.contains(name))
    })
    .into_iter()
    .next()
    .and_then(|file| file.parent().map(Path::to_path_buf))
}

fn find_directory_with_path(root: &Path, suffix: &str) -> Option<PathBuf> {
    if root.join(suffix).exists() {
        return Some(root.to_path_buf());
    }
    let normalized_suffix = normalize_relative_path(suffix);
    collect_files(root, BOOTSTRAP_FILE_LIMIT, |relative| {
        let normalized = normalize_relative_path(relative);
        normalized.ends_with(&normalized_suffix)
            || normalized.contains(&format!("{normalized_suffix}/"))
    })
    .into_iter()
    .next()
    .and_then(|path| directory_for_suffix(root, &path, &normalized_suffix))
}

fn directory_for_suffix(root: &Path, path: &Path, suffix: &str) -> Option<PathBuf> {
    let relative = normalize_relative_path(path.strip_prefix(root).ok()?.to_str()?);
    let suffix_index = if relative.ends_with(suffix) {
        relative.len().saturating_sub(suffix.len())
    } else {
        relative.find(&format!("{suffix}/"))?
    };
    let prefix = relative[..suffix_index].trim_end_matches('/');
    if prefix.is_empty() {
        Some(root.to_path_buf())
    } else {
        Some(root.join(prefix))
    }
}

fn collect_files(root: &Path, limit: usize, predicate: impl Fn(&str) -> bool) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_files_inner(root, root, 0, limit, &predicate, &mut files);
    files
}

fn collect_files_inner(
    root: &Path,
    current: &Path,
    depth: usize,
    limit: usize,
    predicate: &impl Fn(&str) -> bool,
    files: &mut Vec<PathBuf>,
) {
    if depth > BOOTSTRAP_MAX_DEPTH || files.len() >= limit {
        return;
    }
    let Ok(entries) = fs::read_dir(current) else {
        return;
    };
    for entry in entries.flatten() {
        if files.len() >= limit {
            break;
        }
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            let name = entry.file_name();
            if name
                .to_str()
                .is_some_and(|name| BOOTSTRAP_IGNORED_DIRECTORIES.contains(&name))
            {
                continue;
            }
            collect_files_inner(root, &path, depth + 1, limit, predicate, files);
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .ok()
            .and_then(|path| path.to_str())
            .map(normalize_relative_path)
            .unwrap_or_default();
        if predicate(&relative) {
            files.push(path);
        }
    }
}

fn push_task(
    tasks: &mut Vec<DeploymentBootstrapTask>,
    project_root: &Path,
    task_root: &Path,
    kind: &str,
    command: String,
    reason: &str,
) {
    let command = command_in_directory(project_root, task_root, &command);
    if tasks
        .iter()
        .any(|task| task.kind == kind && task.command == command)
    {
        return;
    }
    tasks.push(DeploymentBootstrapTask {
        kind: kind.to_string(),
        command,
        automatic: false,
        reason: reason.to_string(),
    });
}

fn command_in_directory(project_root: &Path, task_root: &Path, command: &str) -> String {
    let relative = task_root
        .strip_prefix(project_root)
        .ok()
        .and_then(|path| path.to_str())
        .map(normalize_relative_path)
        .unwrap_or_default();
    if relative.is_empty() || relative == "." {
        command.to_string()
    } else {
        format!("cd {} && {command}", shell_quote(&relative))
    }
}

fn package_manager_exec(package_manager: Option<PackageManager>, command: &str) -> String {
    match package_manager {
        Some(PackageManager::Pnpm) => format!("pnpm exec {command}"),
        Some(PackageManager::Yarn) => format!("yarn {command}"),
        Some(PackageManager::Bun) => format!("bunx {command}"),
        _ => format!("npx {command}"),
    }
}

fn flyway_command(stack: &DeploymentCodeProbe) -> String {
    if stack.kind == RuntimeKind::Java && stack.package_manager == Some(PackageManager::Maven) {
        return "mvn -DskipTests flyway:migrate".to_string();
    }
    if stack.kind == RuntimeKind::Java && stack.package_manager == Some(PackageManager::Gradle) {
        return "gradle flywayMigrate".to_string();
    }
    "flyway migrate".to_string()
}

fn liquibase_command(stack: &DeploymentCodeProbe) -> String {
    if stack.kind == RuntimeKind::Java && stack.package_manager == Some(PackageManager::Maven) {
        return "mvn -DskipTests liquibase:update".to_string();
    }
    if stack.kind == RuntimeKind::Java && stack.package_manager == Some(PackageManager::Gradle) {
        return "gradle liquibaseUpdate".to_string();
    }
    "liquibase update".to_string()
}

fn tail_lines(text: &str, limit: usize) -> Vec<String> {
    text.lines()
        .rev()
        .take(limit)
        .map(str::to_string)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

fn normalize_relative_path(value: &str) -> String {
    value.replace('\\', "/")
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
