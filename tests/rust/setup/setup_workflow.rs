use setup::{
    archive_package_layout, install, package_file_names, prepare_browser_runtime, purge,
    release_artifact_file_names, write_package_layout, AgentKind, BrowserRuntimePrepareOptions,
    ReleaseManifest, SetupEnvironment, SetupError, TargetPlatform, VERSION,
};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const CODE_REFERENCE_FILES: &[&str] = &[
    "observability",
    "cpp/build",
    "cpp/concurrency",
    "cpp/core",
    "cpp/modern",
    "cpp/performance",
    "cpp/templates",
    "cpp/testing",
    "csharp/blazor",
    "csharp/core",
    "csharp/modern",
    "csharp/performance",
    "csharp/persistence",
    "csharp/testing",
    "go/concurrency",
    "go/core",
    "go/generics",
    "go/interfaces",
    "go/structure",
    "go/testing",
    "java/core",
    "java/persistence",
    "java/reactive",
    "java/security",
    "java/spring",
    "java/testing",
    "javascript/async",
    "javascript/browser",
    "javascript/core",
    "javascript/modules",
    "javascript/node",
    "javascript/testing",
    "kotlin/compose",
    "kotlin/core",
    "kotlin/coroutines",
    "kotlin/dsl",
    "kotlin/ktor",
    "kotlin/multiplatform",
    "kotlin/testing",
    "php/async",
    "php/core",
    "php/modern",
    "php/laravel",
    "php/symfony",
    "php/testing",
    "python/async",
    "python/core",
    "python/packaging",
    "python/testing",
    "python/typing",
    "rust/async",
    "rust/core",
    "rust/errors",
    "rust/ownership",
    "rust/testing",
    "rust/traits",
    "sql/dialects",
    "sql/optimization",
    "sql/queries",
    "sql/schema",
    "sql/windows",
    "sql/mysql/schema",
    "sql/mysql/queries",
    "sql/mysql/transactions",
    "sql/postgresql/schema",
    "sql/postgresql/queries",
    "sql/postgresql/transactions",
    "swift/concurrency",
    "swift/core",
    "swift/memory",
    "swift/protocols",
    "swift/swiftui",
    "swift/testing",
    "typescript/config",
    "typescript/core",
    "typescript/guards",
    "typescript/patterns",
    "typescript/testing",
    "typescript/types",
];

const BACKEND_REFERENCE_FILES: &[&str] = &[
    "aspnetcore/architecture",
    "aspnetcore/data",
    "aspnetcore/logging",
    "aspnetcore/minimal",
    "aspnetcore/runtime",
    "aspnetcore/security",
    "aspnetcore/testing",
    "django/models",
    "django/logging",
    "django/security",
    "django/serializers",
    "django/testing",
    "django/views",
    "fastapi/data",
    "fastapi/logging",
    "fastapi/migration",
    "fastapi/routing",
    "fastapi/schemas",
    "fastapi/security",
    "fastapi/testing",
    "nestjs/controllers",
    "nestjs/dtos",
    "nestjs/logging",
    "nestjs/migration",
    "nestjs/security",
    "nestjs/services",
    "nestjs/testing",
    "springboot/async",
    "springboot/cache",
    "springboot/cloud",
    "springboot/data",
    "springboot/integration",
    "springboot/logging",
    "springboot/observability",
    "springboot/resilience",
    "springboot/runtime",
    "springboot/security",
    "springboot/testing",
    "springboot/web",
    "springboot/mybatis-plus/index",
    "springboot/mybatis-plus/configuration",
    "springboot/mybatis-plus/mapping",
    "springboot/mybatis-plus/crud",
    "springboot/mybatis-plus/wrappers",
    "springboot/mybatis-plus/plugins",
    "springboot/mybatis-plus/security",
    "springboot/mybatis-plus/extensions",
];

const FRONTEND_REFERENCE_FILES: &[&str] = &[
    "angular/components",
    "angular/core",
    "angular/ngrx",
    "angular/routing",
    "angular/rxjs",
    "angular/testing",
    "flutter/bloc",
    "flutter/core",
    "flutter/navigation",
    "flutter/performance",
    "flutter/riverpod",
    "flutter/structure",
    "flutter/testing",
    "flutter/widgets",
    "nextjs/actions",
    "nextjs/app-router",
    "nextjs/core",
    "nextjs/data",
    "nextjs/runtime",
    "nextjs/server-components",
    "nextjs/testing",
    "react/core",
    "react/hooks",
    "react/migration",
    "react/performance",
    "react/react19",
    "react/server-components",
    "react/state",
    "react/testing",
    "react-native/core",
    "react-native/lists",
    "react-native/navigation",
    "react-native/platform",
    "react-native/storage",
    "react-native/structure",
    "react-native/testing",
    "vue/build",
    "vue/components",
    "vue/core",
    "vue/mobile",
    "vue/nuxt",
    "vue/state",
    "vue/testing",
    "vue/typescript",
];

const REVIEW_REFERENCE_FILES: &[&str] = &[
    "core",
    "defect-patterns",
    "finding-quality",
    "spec-compliance",
    "test-evidence",
];

const PLAYWRIGHT_REFERENCE_FILES: &[&str] = &[
    "accessibility",
    "configuration",
    "core",
    "fixtures",
    "locators",
    "network",
    "reliability",
    "visual",
];

#[cfg(unix)]
#[test]
fn browser_runtime_prepare_reuses_valid_cache_and_rebuilds_checksum_drift() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = Fixture::new("browser-runtime-cache");
    let fake_npm = fixture.root.join("fake-npm.sh");
    fs::write(
        &fake_npm,
        r#"#!/bin/sh
set -eu
spec=""
for arg in "$@"; do spec="$arg"; done
version="${spec##*@}"
mkdir -p node_modules/@playwright/test node_modules/.bin
printf '{"name":"@playwright/test","version":"%s"}\n' "$version" > node_modules/@playwright/test/package.json
cat > node_modules/.bin/playwright <<'EOF'
#!/bin/sh
set -eu
mkdir -p "$PLAYWRIGHT_BROWSERS_PATH/chromium-test"
printf ready > "$PLAYWRIGHT_BROWSERS_PATH/chromium-test/marker"
EOF
chmod +x node_modules/.bin/playwright
printf '{"lockfileVersion":3,"packages":{}}\n' > package-lock.json
"#,
    )
    .unwrap();
    let mut permissions = fs::metadata(&fake_npm).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&fake_npm, permissions).unwrap();
    let fake_node = fixture.root.join("fake-node.sh");
    fs::write(&fake_node, "#!/bin/sh\nexit 0\n").unwrap();
    let mut permissions = fs::metadata(&fake_node).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&fake_node, permissions).unwrap();
    let fake_container = fixture.root.join("fake-docker.sh");
    fs::write(&fake_container, "#!/bin/sh\nexit 0\n").unwrap();
    let mut permissions = fs::metadata(&fake_container).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&fake_container, permissions).unwrap();
    let env = fixture.env();
    let options = BrowserRuntimePrepareOptions {
        requested_versions: vec!["1.55.0".to_string()],
        npm_program: Some(fake_npm),
        node_program: Some(fake_node),
        container_program: Some(fake_container),
        ..BrowserRuntimePrepareOptions::default()
    };

    let first = prepare_browser_runtime(&env, &options).unwrap();
    assert_eq!(first.status, "ready");
    assert_eq!(first.platform, setup_platform_key());
    assert_eq!(first.runtimes.len(), 1);
    assert!(!first.runtimes[0].reused);
    assert_eq!(first.runtimes[0].platform, first.platform);
    assert_eq!(first.runtimes[0].browsers, vec!["chromium"]);
    assert!(Path::new(&first.runtimes[0].runner_path).is_file());
    assert!(first.runtimes[0]
        .doctor_checks
        .iter()
        .all(|check| check.status == "passed"));

    let second = prepare_browser_runtime(&env, &options).unwrap();
    assert!(second.runtimes[0].reused);

    let fake_node = options.node_program.as_ref().unwrap();
    fs::write(
        fake_node,
        "#!/bin/sh\necho 'Host system is missing dependencies' >&2\nexit 1\n",
    )
    .unwrap();
    let container_fallback = prepare_browser_runtime(&env, &options).unwrap();
    assert_eq!(container_fallback.status, "ready");
    assert!(container_fallback.runtimes[0].reused);
    assert_eq!(container_fallback.runtimes[0].backend, "managed_container");
    assert!(container_fallback.runtimes[0]
        .doctor_checks
        .iter()
        .any(|check| { check.failure_code.as_deref() == Some("missing_system_dependencies") }));
    assert!(container_fallback.runtimes[0].managed_container.is_some());
    fs::write(fake_node, "#!/bin/sh\nexit 0\n").unwrap();

    let runtime_root = Path::new(&second.runtimes[0].manifest_path)
        .parent()
        .unwrap();
    fs::write(runtime_root.join("package-lock.json"), "corrupt").unwrap();
    let repaired = prepare_browser_runtime(&env, &options).unwrap();
    assert!(!repaired.runtimes[0].reused);
    assert!(repaired.runtimes[0]
        .doctor_checks
        .iter()
        .all(|check| check.status == "passed"));
}

fn setup_platform_key() -> String {
    let os = match std::env::consts::OS {
        "macos" => "darwin",
        "windows" => "windows",
        value => value,
    };
    let arch = match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "x86_64" => "x64",
        value => value,
    };
    format!("{os}-{arch}")
}

#[test]
fn install_projects_shared_references_to_agent_read_paths() {
    let fixture = Fixture::new("install_shared_references");
    fixture.write_package();
    let env = fixture.env();
    write_file(
        &env.opencode_config_path(),
        "{\n  \"$schema\": \"https://opencode.ai/config.json\",\n  \"mcp\": {\n    \"existing-server\": { \"type\": \"local\", \"command\": [\"existing\"], },\n  },\n}\n",
    );

    let report = install(&env, &AgentKind::all()).unwrap();

    assert!(env
        .opencode_home
        .join("references/loom/uix/core.md")
        .exists());
    assert!(env
        .opencode_home
        .join("references/loom/uix/templates/tokens.css.tpl")
        .exists());
    assert!(env
        .opencode_home
        .join("references/loom/uix/templates/tokens.tailwind.tpl")
        .exists());
    assert!(env
        .opencode_home
        .join("references/loom/tech/arch/core.md")
        .exists());
    assert!(env
        .opencode_home
        .join("references/loom/tech/arch/nfr.md")
        .exists());
    assert!(env
        .opencode_home
        .join("references/loom/tech/arch/adr.md")
        .exists());
    assert!(env
        .opencode_home
        .join("references/loom/tech/api/core.md")
        .exists());
    assert!(env
        .opencode_home
        .join("references/loom/tech/api/contract.md")
        .exists());
    for file in REVIEW_REFERENCE_FILES {
        assert!(env
            .opencode_home
            .join(format!("references/loom/tech/review/{file}.md"))
            .exists());
    }
    for file in PLAYWRIGHT_REFERENCE_FILES {
        assert!(env
            .opencode_home
            .join(format!("references/loom/tech/test/playwright/{file}.md"))
            .exists());
    }
    assert!(env
        .opencode_home
        .join("references/loom/tech/code/common.md")
        .exists());
    for file in CODE_REFERENCE_FILES {
        assert!(env
            .opencode_home
            .join(format!("references/loom/tech/code/{file}.md"))
            .exists());
    }
    for file in BACKEND_REFERENCE_FILES {
        assert!(env
            .opencode_home
            .join(format!("references/loom/tech/backend/{file}.md"))
            .exists());
    }
    for file in FRONTEND_REFERENCE_FILES {
        assert!(env
            .opencode_home
            .join(format!("references/loom/tech/frontend/{file}.md"))
            .exists());
    }
    assert!(!env.opencode_home.join("references/loom/delivery").exists());
    assert!(env
        .opencode_home
        .join("references/loom-deploy/compose.md")
        .exists());
    assert!(env
        .opencode_home
        .join("references/loom-deploy/matrix.md")
        .exists());
    assert!(env
        .opencode_home
        .join("references/loom-deploy/source-model.md")
        .exists());
    assert!(env
        .opencode_home
        .join("references/loom-deploy/topology.md")
        .exists());
    assert!(env
        .opencode_home
        .join("references/loom/.loom-mcp-install.json")
        .exists());
    assert!(env
        .opencode_home
        .join("references/loom-deploy/.loom-mcp-install.json")
        .exists());
    assert!(!env
        .agent_mcp_registration_path(AgentKind::Opencode)
        .exists());

    let opencode_config = read_json(&env.opencode_config_path());
    assert_eq!(
        opencode_config["mcp"]["existing-server"]["command"][0].as_str(),
        Some("existing")
    );
    assert_eq!(
        opencode_config["mcp"]["loom"]["command"][0].as_str(),
        Some(path_string_for_test(&env.runtime_current().join("bin/loom-mcp-server")).as_str())
    );
    assert_eq!(
        opencode_config["mcp"]["loom"]["environment"]["LOOM_HOST"].as_str(),
        Some("opencode")
    );
    for check_name in ["opencode.mcpRegistration"] {
        assert_eq!(
            report
                .checks
                .iter()
                .find(|check| check.name == check_name)
                .unwrap()
                .status,
            "passed"
        );
    }
}

#[test]
fn purge_removes_user_runtime_but_preserves_project_state() {
    let fixture = Fixture::new("purge_preserves_project");
    fixture.write_package();
    let env = fixture.env();
    install(&env, &AgentKind::all()).unwrap();
    let project_state = fixture.root.join("project/.loom/status.json");
    fs::create_dir_all(project_state.parent().unwrap()).unwrap();
    fs::write(&project_state, "{}").unwrap();
    let opencode_user_config = env.opencode_home.join("settings.json");
    fs::create_dir_all(opencode_user_config.parent().unwrap()).unwrap();
    fs::write(&opencode_user_config, "{\"theme\":\"user\"}").unwrap();

    let report = purge(&env).unwrap();

    assert_eq!(report.command, "purge");
    assert!(!env.runtime_root().exists());
    assert!(project_state.exists());
    assert!(opencode_user_config.exists());
}

#[test]
fn release_package_names_cover_supported_platforms() {
    let names = package_file_names(VERSION);
    assert_eq!(
        names,
        vec![
            format!("loom-{VERSION}-darwin-arm64.tar.gz"),
            format!("loom-{VERSION}-darwin-x64.tar.gz"),
            format!("loom-{VERSION}-linux-x64.tar.gz"),
            format!("loom-{VERSION}-linux-arm64.tar.gz"),
            format!("loom-{VERSION}-windows-x64.zip"),
        ]
    );
    let artifacts = release_artifact_file_names(VERSION);
    assert_eq!(
        artifacts,
        vec![
            format!("loom-{VERSION}-darwin-arm64.tar.gz"),
            format!("loom-{VERSION}-darwin-arm64.tar.gz.sha256"),
            format!("loom-{VERSION}-darwin-x64.tar.gz"),
            format!("loom-{VERSION}-darwin-x64.tar.gz.sha256"),
            format!("loom-{VERSION}-linux-x64.tar.gz"),
            format!("loom-{VERSION}-linux-x64.tar.gz.sha256"),
            format!("loom-{VERSION}-linux-arm64.tar.gz"),
            format!("loom-{VERSION}-linux-arm64.tar.gz.sha256"),
            format!("loom-{VERSION}-windows-x64.zip"),
            format!("loom-{VERSION}-windows-x64.zip.sha256"),
        ]
    );
}

#[test]
fn archive_package_layout_writes_windows_zip_artifact() {
    let fixture = Fixture::new("archive_package");
    fixture.write_package();
    let output_dir = fixture.root.join("release");
    let archive = archive_package_layout(
        &fixture.package_root,
        &output_dir,
        TargetPlatform::WindowsX64,
    )
    .unwrap();
    assert_eq!(
        archive.file_name().unwrap().to_string_lossy(),
        format!("loom-{VERSION}-windows-x64.zip")
    );
    assert!(archive.exists());
    let checksum = archive.with_file_name(format!(
        "{}.sha256",
        archive.file_name().unwrap().to_string_lossy()
    ));
    assert!(checksum.exists());
    let checksum_text = fs::read_to_string(checksum).unwrap();
    assert!(checksum_text.contains(&sha256(&archive)));
    assert!(checksum_text.contains(&format!("loom-{VERSION}-windows-x64.zip")));
}

#[test]
fn install_sh_release_plan_resolves_platform_assets_and_checksums() {
    let repo = repo_root();
    let script = repo.join("install.sh");
    let mac_output = Command::new("sh")
        .arg(&script)
        .args(["--agent", "opencode", "--version", "9.8.7", "--print-plan"])
        .env("LOOM_INSTALL_TEST_OS", "Darwin")
        .env("LOOM_INSTALL_TEST_ARCH", "arm64")
        .output()
        .unwrap();
    assert!(
        mac_output.status.success(),
        "{}",
        String::from_utf8_lossy(&mac_output.stderr)
    );
    let mac_plan: serde_json::Value = serde_json::from_slice(&mac_output.stdout).unwrap();
    assert_eq!(mac_plan["agent"], "opencode");
    assert_eq!(mac_plan["platform"], "darwin-arm64");
    assert_eq!(mac_plan["package"], "loom-9.8.7-darwin-arm64.tar.gz");
    assert_eq!(
        mac_plan["packageUrl"],
        "https://github.com/valkor-ai/loom/releases/download/v9.8.7/loom-9.8.7-darwin-arm64.tar.gz"
    );
    assert_eq!(
        mac_plan["checksumUrl"],
        "https://github.com/valkor-ai/loom/releases/download/v9.8.7/loom-9.8.7-darwin-arm64.tar.gz.sha256"
    );
    assert_eq!(mac_plan["archiveChecksumRequired"], true);

    let linux_output = Command::new("sh")
        .arg(&script)
        .args([
            "--agent",
            "all",
            "--version",
            "9.8.7",
            "--base-url",
            "https://mirror.example/loom",
            "--print-plan",
        ])
        .env("LOOM_INSTALL_TEST_OS", "linux")
        .env("LOOM_INSTALL_TEST_ARCH", "x86_64")
        .output()
        .unwrap();
    assert!(
        linux_output.status.success(),
        "{}",
        String::from_utf8_lossy(&linux_output.stderr)
    );
    let linux_plan: serde_json::Value = serde_json::from_slice(&linux_output.stdout).unwrap();
    assert_eq!(linux_plan["agent"], "all");
    assert_eq!(linux_plan["platform"], "linux-x64");
    assert_eq!(
        linux_plan["packageUrl"],
        "https://mirror.example/loom/loom-9.8.7-linux-x64.tar.gz"
    );
    assert_eq!(
        linux_plan["checksumUrl"],
        "https://mirror.example/loom/loom-9.8.7-linux-x64.tar.gz.sha256"
    );
}

#[test]
fn install_ps1_release_contract_uses_windows_zip_checksum_and_doctor() {
    let script = fs::read_to_string(repo_root().join("install.ps1")).unwrap();
    assert!(script.contains("loom-$Version-$platform.zip"));
    assert!(script.contains("$packageUrl.sha256"));
    assert!(script.contains("Get-FileHash -Algorithm SHA256"));
    assert!(script.contains("windows-x64"));
    assert!(script.contains("install --agent $Agent --package-root"));
    assert!(script.contains("doctor --agent $Agent --package-root"));
}

#[test]
fn release_workflow_uploads_installers_packages_and_checksums() {
    let workflow = fs::read_to_string(repo_root().join(".github/workflows/release.yml")).unwrap();
    assert!(workflow.contains("tags:"));
    assert!(workflow.contains("- \"v*\""));
    assert!(workflow.contains("contents: write"));
    for platform in [
        "darwin-arm64",
        "darwin-x64",
        "linux-x64",
        "linux-arm64",
        "windows-x64",
    ] {
        assert!(workflow.contains(platform), "missing platform {platform}");
    }
    assert!(workflow.contains("loom-setup.exe"));
    assert!(workflow.contains("install.sh"));
    assert!(workflow.contains("install.ps1"));
    assert!(workflow.contains("install.sh.sha256"));
    assert!(workflow.contains("install.ps1.sha256"));
    assert!(workflow.contains(".tar.gz.sha256"));
    assert!(workflow.contains(".zip.sha256"));
    assert!(workflow.contains("softprops/action-gh-release@v3"));
    assert!(workflow.contains("make_latest: true"));
}

#[test]
fn archive_package_layout_rejects_legacy_typescript_runtime_entries() {
    let fixture = Fixture::new("archive_rejects_legacy_runtime");
    fixture.write_package();
    write_file(
        &fixture.package_root.join("src/ts/cli.ts"),
        "console.log('legacy cli');\n",
    );
    fixture.write_checksums();

    let error = archive_package_layout(
        &fixture.package_root,
        &fixture.root.join("release"),
        TargetPlatform::DarwinArm64,
    )
    .unwrap_err();
    match error {
        SetupError::InvalidArgument(message) => {
            assert!(message.contains("发布包不得包含"));
            assert!(message.contains("src/ts/cli.ts"));
        }
        other => panic!("expected InvalidArgument, got {other:?}"),
    }
}

#[test]
fn archive_package_layout_rejects_legacy_cli_launcher_entries() {
    let fixture = Fixture::new("archive_rejects_legacy_launcher");
    fixture.write_package();
    write_file(&fixture.package_root.join("bin/loom-cli"), "#!/bin/sh\n");
    fixture.write_checksums();

    let error = archive_package_layout(
        &fixture.package_root,
        &fixture.root.join("release"),
        TargetPlatform::LinuxX64,
    )
    .unwrap_err();
    match error {
        SetupError::InvalidArgument(message) => {
            assert!(message.contains("发布包不得包含"));
            assert!(message.contains("bin/loom-cli"));
        }
        other => panic!("expected InvalidArgument, got {other:?}"),
    }
}

#[test]
fn package_layout_copies_current_runtime_and_plugin_sources() {
    let fixture = Fixture::new("package_layout_sources");
    let binary_dir = fixture.root.join("built-bin");
    write_file(&binary_dir.join("loom-mcp-server"), "#!/bin/sh\n");
    write_file(&binary_dir.join("loom-setup"), "#!/bin/sh\n");
    std::env::set_var("LOOM_SETUP_BINARY_DIR", &binary_dir);

    let package = write_package_layout(
        &fixture.root.join("package-out"),
        TargetPlatform::DarwinArm64,
    );
    std::env::remove_var("LOOM_SETUP_BINARY_DIR");
    let package = package.unwrap();
    assert!(package.join("bin/loom-mcp-server").is_file());
    assert!(package.join("bin/loom-setup").is_file());
    assert!(package.join("python/algorithms/worker.py").is_file());
    assert!(package.join("python/runtime/README").is_file());
    assert!(package
        .join("plugins/opencode/.opencode/plugins/loom.js")
        .is_file());
    assert!(package
        .join("plugins/shared/loom/references/uix/core.md")
        .is_file());
    assert!(package
        .join("plugins/shared/loom-deploy/references/compose.md")
        .is_file());
    assert!(package
        .join("plugins/shared/loom-deploy/references/matrix.md")
        .is_file());
    assert!(package
        .join("plugins/shared/loom-deploy/references/source-model.md")
        .is_file());
    assert!(package
        .join("plugins/shared/loom-deploy/references/topology.md")
        .is_file());
    assert!(package.join("checksums.txt").is_file());
}

#[test]
fn plugin_templates_do_not_expose_legacy_protocol_terms() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let plugin_root = repo.join("plugins");
    let files = [
        "opencode/.opencode/commands/loom.md",
        "opencode/.opencode/commands/loom-deploy.md",
        "opencode/.opencode/plugins/loom.js",
    ];
    let forbidden = [
        "loom-cli",
        "LOOM_AGENT_PROFILE",
        "LOOM_COMPACT_OUTPUT",
        "commandInvocation",
        "submitCommand.argv",
        "CLI envelope",
    ];
    for file in files {
        let path = plugin_root.join(file);
        let content = fs::read_to_string(&path).unwrap();
        for term in forbidden {
            assert!(
                !content.contains(term),
                "{} must not contain legacy term {term}",
                path.display()
            );
        }
    }
}

#[test]
fn deploy_plugin_templates_obey_active_operation_policy_fields() {
    let repo = repo_root();
    let plugin_root = repo.join("plugins");
    for file in ["opencode/.opencode/commands/loom-deploy.md"] {
        let content = fs::read_to_string(plugin_root.join(file)).unwrap();
        for required in [
            "observationPolicy",
            "forbiddenActions",
            "finalResponsePolicy",
        ] {
            assert!(
                content.contains(required),
                "{file} must require deploy active_operation policy field {required}"
            );
        }
    }
}

#[test]
fn agent_templates_obey_user_gate_pre_response_contract() {
    let repo = repo_root();
    let plugin_root = repo.join("plugins");
    for file in [
        "opencode/.opencode/commands/loom.md",
        "opencode/.opencode/commands/loom-deploy.md",
    ] {
        let content = fs::read_to_string(plugin_root.join(file)).unwrap();
        assert!(
            content.contains("preResponseContract"),
            "{file} must consume the structured user-gate pre-response contract"
        );
        assert!(
            content.contains("requestReadPlan.groups"),
            "{file} must keep requestReadPlan.groups as the only read contract"
        );
        assert!(
            content.contains("loom.inspectRequest") && content.contains("loom.readFieldGroup"),
            "{file} must require inspect-then-group-read before a gated response"
        );
    }
}

#[test]
fn opencode_commands_expose_mcp_result_discipline() {
    let repo = repo_root();
    let command_root = repo.join("plugins/opencode/.opencode/commands");
    let loom = fs::read_to_string(command_root.join("loom.md")).unwrap();
    let deploy = fs::read_to_string(command_root.join("loom-deploy.md")).unwrap();

    for required in [
        "loom.inspectRequest",
        "loom.readFieldGroup",
        "requestReadPlan.groups",
        "GenerateKnowledgeSemanticsNext",
        "loom.knowledgeInspectChunk",
        "ExecuteTaskNext",
        "RunLoomToolNext",
        "retryTool",
        "DeployRepairAssetsNext",
        "不要将字段级约定",
    ] {
        assert!(
            loom.contains(required),
            "opencode loom.md missing {required}"
        );
    }
    assert!(
        !loom.contains("loom.readRequestFields"),
        "opencode loom.md must not expose readRequestFields"
    );

    for required in [
        "引用配置文件：",
        "引用纪律：",
        "referenceLoadPlan",
        "解析 `path`",
        "仅加载当前操作列出的路径",
        "不要从字段组名称派生路径",
        "扫描引用目录",
        "外部语言/API/架构/UI 技能",
        "不要粘贴引用正文或模板内容",
        "交付规划、设计、审查、修复和交接规则由当前 MCP 请求/结果提供",
        "不要加载单独的交付引用文件",
    ] {
        assert!(
            loom.contains(required),
            "opencode loom.md missing optional reference guidance {required}"
        );
    }
    for forbidden in [
        "MCP-selected references:",
        "../references/loom/uix/core.md",
        "`groups.core`",
        "../references/loom/uix/anti-patterns.md",
        "../references/loom/uix/templates/tokens.css.tpl",
        "Focus references are contract-selected group/items",
        "skill_reference_by_group",
    ] {
        assert!(
            !loom.contains(forbidden),
            "opencode loom.md must not retain hard-coded reference map fragment {forbidden}"
        );
    }

    for required in [
        "active_operation",
        "DeployRepairAssetsNext",
        "部署执行修复",
        "loom.inspectRequest",
        "loom.readFieldGroup",
        "deployReferenceProfile",
        "../references/loom-deploy/",
        "referenceLoadPlan",
        "requestReadPlan.groups",
        "不要将部署栈规则",
    ] {
        assert!(
            deploy.contains(required),
            "opencode loom-deploy.md missing {required}"
        );
    }
}

#[test]
fn agent_templates_expose_reference_loading_protocol() {
    let repo = repo_root();
    let plugin_root = repo.join("plugins");
    let files = ["opencode/.opencode/commands/loom.md"];

    for file in files {
        let content = fs::read_to_string(plugin_root.join(file)).unwrap();
        assert!(
            !content.contains("## Optional References"),
            "{file} must use Reference Loading instead of Optional References"
        );
        assert!(
            !content.contains("UIX references:"),
            "{file} must not keep the old broad UIX references section"
        );
        for required in [
            "## 引用加载",
            "协议：",
            "引用纪律：",
            "读取当前请求字段组后",
            "referenceLoadPlan",
            "引用配置文件：",
            "不要从字段组名称派生路径",
            "扫描引用目录",
            "如果引用文件未被 MCP 约定选定",
            "不要粘贴引用正文或模板内容",
        ] {
            assert!(
                content.contains(required),
                "{file} missing reference loading protocol fragment {required}"
            );
        }
        for forbidden in [
            "MCP-selected references:",
            "../references/loom/uix/core.md",
            "`groups.core`",
            "`groups.scenarios`",
            "`groups.tokens`",
            "`groups.stacks`",
            "`groups.templates`",
            "skill_reference_by_group",
        ] {
            assert!(
                !content.contains(forbidden),
                "{file} must not retain hard-coded reference map fragment {forbidden}"
            );
        }
    }

    for file in ["opencode/.opencode/commands/loom-deploy.md"] {
        let content = fs::read_to_string(plugin_root.join(file)).unwrap();
        assert!(
            !content.contains("## Optional References"),
            "{file} must use Reference Loading instead of Optional References"
        );
        for required in [
            "## 引用加载",
            "协议：",
            "deployReferenceProfile.referenceLoadPlan",
            "每个 `referenceLoadPlan` 条目包含 `refId`、`path` 和 `reason`",
            "推断额外文件",
            "不要将引用正文粘贴",
            "如果当前部署操作没有 `deployReferenceProfile`",
        ] {
            assert!(
                content.contains(required),
                "{file} missing deploy reference loading protocol fragment {required}"
            );
        }
        for forbidden in [
            "deployReferenceProfile.referenceIds",
            "MCP-selected deploy references:",
            "`deploy.providers` ->",
            "`deploy.matrix` ->",
            "`deploy.stacks.java`",
        ] {
            assert!(
                !content.contains(forbidden),
                "{file} must not retain deploy reference id map fragment {forbidden}"
            );
        }
        assert!(
            !content.contains("external-references"),
            "{file} must not expose maintainer research as a deploy reference"
        );
    }
}

#[test]
fn loom_code_references_are_operational_and_load_plan_driven() {
    let repo = repo_root();
    let code_root = repo.join("plugins/shared/loom/references/tech/code");
    let backend_root = repo.join("plugins/shared/loom/references/tech/backend");
    let frontend_root = repo.join("plugins/shared/loom/references/tech/frontend");
    let common = fs::read_to_string(code_root.join("common.md")).unwrap();
    for required in [
        "在 Loom 中的定位",
        "仓库适配",
        "交付规则",
        "验证规则",
        "证据规则",
        "常见反模式",
    ] {
        assert!(
            common.contains(required),
            "common code reference missing shared section {required}"
        );
    }
    let mut files = Vec::new();
    collect_markdown_files(&code_root, &mut files);
    assert!(
        files.len() >= 71,
        "expected language reference coverage for all supported code profiles"
    );

    for path in files {
        if path.file_name().is_some_and(|name| name == "common.md") {
            continue;
        }
        let content = fs::read_to_string(&path).unwrap();
        let line_count = content.lines().count();
        let code_profile = path
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|name| name.to_str());
        let minimum_lines = if matches!(code_profile, Some("cpp" | "csharp" | "go")) {
            65
        } else {
            25
        };
        assert!(
            line_count >= minimum_lines,
            "{} is too thin to act as a topic code reference: {line_count} lines",
            path.display()
        );
        for required in [
            "When To Use",
            "Implementation Focus",
            "Verification Focus",
            "Evidence Focus",
        ] {
            assert!(
                content.contains(required),
                "{} missing code reference section or boundary {required}",
                path.display()
            );
        }
        if matches!(code_profile, Some("cpp" | "csharp" | "go")) {
            assert!(
                content.contains("## Unsafe Defaults"),
                "{} missing enhanced code-profile unsafe-default guidance",
                path.display()
            );
        }
        for forbidden in [
            "referenceLoadPlan",
            "readFieldGroup",
            "requestReadPlan",
            "techReferenceProfile",
            "skill_reference_by_group",
            "Load this file only when `techReferenceProfile.groups",
            "Load only the listed group/items",
            "Source Coverage",
            "Repository Adaptation",
            "Delivery Patterns",
            "## Evidence\n",
            "## Anti-Patterns",
        ] {
            assert!(
                !content.contains(forbidden),
                "{} retained old group-to-path reference loading fragment {forbidden}",
                path.display()
            );
        }
    }
    let java_core = fs::read_to_string(code_root.join("java/core.md")).unwrap();
    assert!(java_core.contains("app.<project_slug>"));
    assert!(java_core.contains("app.generated"));
    assert!(java_core.contains("com.example"));
    let java_spring = fs::read_to_string(code_root.join("java/spring.md")).unwrap();
    assert!(java_spring.contains("组件扫描"));
    assert!(java_spring.contains("app.<project_slug>"));
    assert!(java_spring.contains("@Qualifier"));
    let cpp_core = fs::read_to_string(code_root.join("cpp/core.md")).unwrap();
    assert!(cpp_core.contains("std::string_view"));
    assert!(cpp_core.contains("唯一定义规则"));
    let cpp_modern = fs::read_to_string(code_root.join("cpp/modern.md")).unwrap();
    assert!(cpp_modern.contains("特性测试宏"));
    assert!(cpp_modern.contains("std::expected"));
    let cpp_templates = fs::read_to_string(code_root.join("cpp/templates.md")).unwrap();
    assert!(cpp_templates.contains("引用折叠"));
    assert!(cpp_templates.contains("显式实例化"));
    let cpp_performance = fs::read_to_string(code_root.join("cpp/performance.md")).unwrap();
    assert!(cpp_performance.contains("反优化"));
    assert!(cpp_performance.contains("ISA 特定"));
    let cpp_concurrency = fs::read_to_string(code_root.join("cpp/concurrency.md")).unwrap();
    assert!(cpp_concurrency.contains("std::jthread"));
    assert!(cpp_concurrency.contains("happens-before"));
    let cpp_build = fs::read_to_string(code_root.join("cpp/build.md")).unwrap();
    assert!(cpp_build.contains("生成器表达式"));
    assert!(cpp_build.contains("target_compile_features"));
    let cpp_testing = fs::read_to_string(code_root.join("cpp/testing.md")).unwrap();
    assert!(cpp_testing.contains("ASan"));
    assert!(cpp_testing.contains("模糊测试目标"));
    assert!(!code_root.join("csharp/aspnet.md").exists());
    let csharp_core = fs::read_to_string(code_root.join("csharp/core.md")).unwrap();
    assert!(csharp_core.contains("OperationCanceledException"));
    assert!(csharp_core.contains("IServiceProvider"));
    let csharp_modern = fs::read_to_string(code_root.join("csharp/modern.md")).unwrap();
    assert!(csharp_modern.contains("LangVersion"));
    assert!(csharp_modern.contains("Native AOT"));
    let csharp_persistence = fs::read_to_string(code_root.join("csharp/persistence.md")).unwrap();
    assert!(csharp_persistence.contains("IDbContextFactory"));
    assert!(csharp_persistence.contains("DbUpdateConcurrencyException"));
    let csharp_blazor = fs::read_to_string(code_root.join("csharp/blazor.md")).unwrap();
    assert!(csharp_blazor.contains("JSDisconnectedException"));
    assert!(csharp_blazor.contains("持久组件状态"));
    let csharp_performance = fs::read_to_string(code_root.join("csharp/performance.md")).unwrap();
    assert!(csharp_performance.contains("BenchmarkDotNet"));
    assert!(csharp_performance.contains("ArrayPool"));
    let csharp_testing = fs::read_to_string(code_root.join("csharp/testing.md")).unwrap();
    assert!(csharp_testing.contains("HttpMessageHandler"));
    assert!(csharp_testing.contains("变异测试"));
    let go_core = fs::read_to_string(code_root.join("go/core.md")).unwrap();
    assert!(go_core.contains("errors.Join"));
    assert!(go_core.contains("rows.Err"));
    let go_concurrency = fs::read_to_string(code_root.join("go/concurrency.md")).unwrap();
    assert!(go_concurrency.contains("errgroup.WithContext"));
    assert!(go_concurrency.contains("SetLimit"));
    let go_interfaces = fs::read_to_string(code_root.join("go/interfaces.md")).unwrap();
    assert!(go_interfaces.contains("类型化 nil"));
    assert!(go_interfaces.contains("指向接口的指针"));
    let go_generics = fs::read_to_string(code_root.join("go/generics.md")).unwrap();
    assert!(go_generics.contains("~T"));
    assert!(go_generics.contains("方法不能独立引入"));
    let go_structure = fs::read_to_string(code_root.join("go/structure.md")).unwrap();
    assert!(go_structure.contains("go.work"));
    assert!(go_structure.contains("//go:build"));
    let go_testing = fs::read_to_string(code_root.join("go/testing.md")).unwrap();
    assert!(go_testing.contains("httptest.Server"));
    assert!(go_testing.contains("t.Setenv"));
    let spring_runtime = fs::read_to_string(backend_root.join("springboot/runtime.md")).unwrap();
    assert!(spring_runtime.contains("@ConfigurationProperties"));
    assert!(spring_runtime.contains("优雅关闭"));
    let spring_async = fs::read_to_string(backend_root.join("springboot/async.md")).unwrap();
    assert!(spring_async.contains("@Async"));
    assert!(spring_async.contains("同类自调用"));
    let spring_cache = fs::read_to_string(backend_root.join("springboot/cache.md")).unwrap();
    assert!(spring_cache.contains("@Cacheable"));
    assert!(spring_cache.contains("真值来源"));

    let mut backend_files = Vec::new();
    collect_markdown_files(&backend_root, &mut backend_files);
    assert!(
        backend_files.len() >= BACKEND_REFERENCE_FILES.len(),
        "expected backend framework reference coverage for selected framework profiles"
    );
    for path in backend_files {
        let content = fs::read_to_string(&path).unwrap();
        let line_count = content.lines().count();
        let backend_profile = path
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|name| name.to_str());
        let is_spring_boot = backend_profile == Some("springboot");
        let is_nestjs = backend_profile == Some("nestjs");
        let is_aspnet_core = backend_profile == Some("aspnetcore");
        let minimum_lines = if matches!(
            backend_profile,
            Some("springboot" | "fastapi" | "django" | "nestjs" | "aspnetcore")
        ) {
            65
        } else {
            25
        };
        assert!(
            line_count >= minimum_lines,
            "{} is too thin to act as a backend framework reference: {line_count} lines",
            path.display()
        );
        if is_spring_boot {
            for required in ["Verification Focus", "不安全默认"] {
                assert!(
                    content.contains(required),
                    "{} missing Spring Boot engineering section {required}",
                    path.display()
                );
            }
        } else if is_nestjs || is_aspnet_core {
            for required in ["验证", "## 交付证据", "不安全默认"] {
                assert!(
                    content.contains(required),
                    "{} missing enhanced backend engineering section {required}",
                    path.display()
                );
            }
        } else {
            for required in [
                "何时使用",
                "Implementation Focus",
                "Verification Focus",
                "Evidence Focus",
            ] {
                assert!(
                    content.contains(required),
                    "{} missing backend framework reference section or boundary {required}",
                    path.display()
                );
            }
        }
        for forbidden in [
            "referenceLoadPlan",
            "readFieldGroup",
            "requestReadPlan",
            "techReferenceProfile",
            "skill_reference_by_group",
            "Load this file only when `techReferenceProfile.groups",
            "Load only the listed group/items",
            "Source Coverage",
            "Repository Adaptation",
            "Delivery Patterns",
            "## Evidence\n",
            "## Anti-Patterns",
        ] {
            assert!(
                !content.contains(forbidden),
                "{} retained protocol or duplicated shared reference fragment {forbidden}",
                path.display()
            );
        }
    }
    let spring_web = fs::read_to_string(backend_root.join("springboot/web.md")).unwrap();
    assert!(spring_web.contains("真实 Spring Boot 基础包"));
    assert!(spring_web.contains("com.example"));
    let fastapi_routing = fs::read_to_string(backend_root.join("fastapi/routing.md")).unwrap();
    assert!(fastapi_routing.contains("APIRouter"));
    assert!(fastapi_routing.contains("BackgroundTasks"));
    let fastapi_schemas = fs::read_to_string(backend_root.join("fastapi/schemas.md")).unwrap();
    assert!(fastapi_schemas.contains("model_fields_set"));
    assert!(fastapi_schemas.contains("from_attributes"));
    let fastapi_data = fs::read_to_string(backend_root.join("fastapi/data.md")).unwrap();
    assert!(fastapi_data.contains("async_sessionmaker"));
    assert!(fastapi_data.contains("Alembic"));
    let fastapi_security = fs::read_to_string(backend_root.join("fastapi/security.md")).unwrap();
    assert!(fastapi_security.contains("OAuth2PasswordBearer"));
    assert!(fastapi_security.contains("issuer"));
    let fastapi_testing = fs::read_to_string(backend_root.join("fastapi/testing.md")).unwrap();
    assert!(fastapi_testing.contains("ASGITransport"));
    assert!(fastapi_testing.contains("dependency_overrides"));
    let fastapi_migration = fs::read_to_string(backend_root.join("fastapi/migration.md")).unwrap();
    assert!(fastapi_migration.contains("ViewSet"));
    assert!(fastapi_migration.contains("对齐"));
    let django_models = fs::read_to_string(backend_root.join("django/models.md")).unwrap();
    assert!(django_models.contains("select_related"));
    assert!(django_models.contains("apps.get_model"));
    let django_serializers =
        fs::read_to_string(backend_root.join("django/serializers.md")).unwrap();
    assert!(django_serializers.contains("SerializerMethodField"));
    assert!(django_serializers.contains("partial=True"));
    let django_views = fs::read_to_string(backend_root.join("django/views.md")).unwrap();
    assert!(django_views.contains("get_queryset"));
    assert!(django_views.contains("对象级检查"));
    let django_security = fs::read_to_string(backend_root.join("django/security.md")).unwrap();
    assert!(django_security.contains("SimpleJWT"));
    assert!(django_security.contains("CSRF"));
    let django_testing = fs::read_to_string(backend_root.join("django/testing.md")).unwrap();
    assert!(django_testing.contains("TransactionTestCase"));
    assert!(django_testing.contains("assertNumQueries"));
    let nest_controllers = fs::read_to_string(backend_root.join("nestjs/controllers.md")).unwrap();
    assert!(nest_controllers.contains("ParseUUIDPipe"));
    assert!(nest_controllers.contains("@Res()"));
    let nest_dtos = fs::read_to_string(backend_root.join("nestjs/dtos.md")).unwrap();
    assert!(nest_dtos.contains("PartialType"));
    assert!(nest_dtos.contains("bigint"));
    let nest_services = fs::read_to_string(backend_root.join("nestjs/services.md")).unwrap();
    assert!(nest_services.contains("useExisting"));
    assert!(nest_services.contains("forwardRef"));
    let nest_security = fs::read_to_string(backend_root.join("nestjs/security.md")).unwrap();
    assert!(nest_security.contains("getAllAndOverride"));
    assert!(nest_security.contains("APP_GUARD"));
    let nest_testing = fs::read_to_string(backend_root.join("nestjs/testing.md")).unwrap();
    assert!(nest_testing.contains("TestingModule"));
    assert!(nest_testing.contains("app.getHttpServer()"));
    let nest_migration = fs::read_to_string(backend_root.join("nestjs/migration.md")).unwrap();
    assert!(nest_migration.contains("对齐矩阵"));
    assert!(nest_migration.contains("路由归属"));
    let aspnet_architecture =
        fs::read_to_string(backend_root.join("aspnetcore/architecture.md")).unwrap();
    assert!(aspnet_architecture.contains("MediatR"));
    assert!(aspnet_architecture.contains("IServiceProvider"));
    let aspnet_minimal = fs::read_to_string(backend_root.join("aspnetcore/minimal.md")).unwrap();
    assert!(aspnet_minimal.contains("TypedResults"));
    assert!(aspnet_minimal.contains("WebApplicationFactory"));
    let aspnet_data = fs::read_to_string(backend_root.join("aspnetcore/data.md")).unwrap();
    assert!(aspnet_data.contains("IEntityTypeConfiguration"));
    assert!(aspnet_data.contains("DbUpdateConcurrencyException"));
    let aspnet_security = fs::read_to_string(backend_root.join("aspnetcore/security.md")).unwrap();
    assert!(aspnet_security.contains("IAuthorizationRequirement"));
    assert!(aspnet_security.contains("UseAuthentication"));
    let aspnet_runtime = fs::read_to_string(backend_root.join("aspnetcore/runtime.md")).unwrap();
    assert!(aspnet_runtime.contains("ValidateOnStart"));
    assert!(aspnet_runtime.contains("IHttpClientFactory"));
    assert!(aspnet_runtime.contains("HybridCache"));
    let aspnet_testing = fs::read_to_string(backend_root.join("aspnetcore/testing.md")).unwrap();
    assert!(aspnet_testing.contains("WebApplicationFactory<Program>"));
    assert!(aspnet_testing.contains("EF InMemory"));

    let mut frontend_files = Vec::new();
    collect_markdown_files(&frontend_root, &mut frontend_files);
    assert!(
        frontend_files.len() >= FRONTEND_REFERENCE_FILES.len(),
        "expected frontend framework reference coverage for selected framework profiles"
    );
    for path in frontend_files {
        let content = fs::read_to_string(&path).unwrap();
        let line_count = content.lines().count();
        let frontend_profile = path
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|name| name.to_str());
        let is_angular = frontend_profile == Some("angular");
        let is_flutter = frontend_profile == Some("flutter");
        let is_nextjs = frontend_profile == Some("nextjs");
        let is_react = frontend_profile == Some("react");
        let is_react_native = frontend_profile == Some("react-native");
        let is_vue = frontend_profile == Some("vue");
        let is_enhanced_frontend =
            is_angular || is_flutter || is_nextjs || is_react || is_react_native || is_vue;
        let minimum_lines = if is_enhanced_frontend { 65 } else { 25 };
        assert!(
            line_count >= minimum_lines,
            "{} is too thin to act as a frontend framework reference: {line_count} lines",
            path.display()
        );
        if is_enhanced_frontend {
            for required in ["验证", "## 交付证据", "不安全默认"] {
                assert!(
                    content.contains(required),
                    "{} missing enhanced frontend engineering section {required}",
                    path.display()
                );
            }
        } else {
            for required in [
                "When To Use",
                "Implementation Focus",
                "Verification Focus",
                "Evidence Focus",
            ] {
                assert!(
                    content.contains(required),
                    "{} missing frontend framework reference section {required}",
                    path.display()
                );
            }
        }
        for forbidden in [
            "referenceLoadPlan",
            "readFieldGroup",
            "requestReadPlan",
            "techReferenceProfile",
            "skill_reference_by_group",
            "Load this file only when `techReferenceProfile.groups",
            "Load only the listed group/items",
            "Source Coverage",
            "Repository Adaptation",
            "Delivery Patterns",
            "## Evidence\n",
            "## Anti-Patterns",
        ] {
            assert!(
                !content.contains(forbidden),
                "{} retained protocol or duplicated shared reference fragment {forbidden}",
                path.display()
            );
        }
    }
    let angular_core = fs::read_to_string(frontend_root.join("angular/core.md")).unwrap();
    assert!(angular_core.contains("ChangeDetectionStrategy.OnPush"));
    assert!(angular_core.contains("getRawValue()"));
    let angular_components =
        fs::read_to_string(frontend_root.join("angular/components.md")).unwrap();
    assert!(angular_components.contains("input.required"));
    assert!(angular_components.contains("DestroyRef"));
    let angular_routing = fs::read_to_string(frontend_root.join("angular/routing.md")).unwrap();
    assert!(angular_routing.contains("RouterTestingHarness"));
    assert!(angular_routing.contains("withComponentInputBinding"));
    let angular_rxjs = fs::read_to_string(frontend_root.join("angular/rxjs.md")).unwrap();
    assert!(angular_rxjs.contains("takeUntilDestroyed"));
    assert!(angular_rxjs.contains("shareReplay"));
    let angular_ngrx = fs::read_to_string(frontend_root.join("angular/ngrx.md")).unwrap();
    assert!(angular_ngrx.contains("createEntityAdapter"));
    assert!(angular_ngrx.contains("concatLatestFrom"));
    let angular_testing = fs::read_to_string(frontend_root.join("angular/testing.md")).unwrap();
    assert!(angular_testing.contains("provideHttpClientTesting"));
    assert!(angular_testing.contains("TestScheduler.run"));
    let flutter_core = fs::read_to_string(frontend_root.join("flutter/core.md")).unwrap();
    assert!(flutter_core.contains("if (!mounted)"));
    assert!(flutter_core.contains("Platform.is"));
    let flutter_widgets = fs::read_to_string(frontend_root.join("flutter/widgets.md")).unwrap();
    assert!(flutter_widgets.contains("ValueKey"));
    assert!(flutter_widgets.contains("SliverList"));
    let flutter_structure = fs::read_to_string(frontend_root.join("flutter/structure.md")).unwrap();
    assert!(flutter_structure.contains("build_runner"));
    assert!(flutter_structure.contains("条件导入"));
    let flutter_navigation =
        fs::read_to_string(frontend_root.join("flutter/navigation.md")).unwrap();
    assert!(flutter_navigation.contains("stateful shell"));
    assert!(flutter_navigation.contains("pathParameters"));
    let flutter_riverpod = fs::read_to_string(frontend_root.join("flutter/riverpod.md")).unwrap();
    assert!(flutter_riverpod.contains("AsyncNotifierProvider"));
    assert!(flutter_riverpod.contains("copyWithPrevious"));
    let flutter_bloc = fs::read_to_string(frontend_root.join("flutter/bloc.md")).unwrap();
    assert!(flutter_bloc.contains("BlocProvider.value"));
    assert!(flutter_bloc.contains("事件 transformer"));
    let flutter_performance =
        fs::read_to_string(frontend_root.join("flutter/performance.md")).unwrap();
    assert!(flutter_performance.contains("profile 模式"));
    assert!(flutter_performance.contains("RepaintBoundary"));
    let flutter_testing = fs::read_to_string(frontend_root.join("flutter/testing.md")).unwrap();
    assert!(flutter_testing.contains("ProviderContainer"));
    assert!(flutter_testing.contains("pumpAndSettle"));
    let next_core = fs::read_to_string(frontend_root.join("nextjs/core.md")).unwrap();
    assert!(next_core.contains("NEXT_PUBLIC_*"));
    assert!(next_core.contains("server-only"));
    let next_router = fs::read_to_string(frontend_root.join("nextjs/app-router.md")).unwrap();
    assert!(next_router.contains("intercepting 路由"));
    assert!(next_router.contains("notFound()"));
    let next_server_components =
        fs::read_to_string(frontend_root.join("nextjs/server-components.md")).unwrap();
    assert!(next_server_components.contains("可序列化交接"));
    assert!(next_server_components.contains("suppressHydrationWarning"));
    let next_actions = fs::read_to_string(frontend_root.join("nextjs/actions.md")).unwrap();
    assert!(next_actions.contains("useActionState"));
    assert!(next_actions.contains("revalidateTag"));
    let next_data = fs::read_to_string(frontend_root.join("nextjs/data.md")).unwrap();
    assert!(next_data.contains("React `cache()`"));
    assert!(next_data.contains("跨用户/租户"));
    let next_runtime = fs::read_to_string(frontend_root.join("nextjs/runtime.md")).unwrap();
    assert!(next_runtime.contains("output: 'standalone'"));
    assert!(next_runtime.contains("Edge"));
    let next_testing = fs::read_to_string(frontend_root.join("nextjs/testing.md")).unwrap();
    assert!(next_testing.contains("production `next build`"));
    assert!(next_testing.contains("Server Action"));
    let react_core = fs::read_to_string(frontend_root.join("react/core.md")).unwrap();
    assert!(react_core.contains("稳定的领域键"));
    assert!(react_core.contains("dangerouslySetInnerHTML"));
    let react_hooks = fs::read_to_string(frontend_root.join("react/hooks.md")).unwrap();
    assert!(react_hooks.contains("AbortController"));
    assert!(react_hooks.contains("useSyncExternalStore"));
    let react_state = fs::read_to_string(frontend_root.join("react/state.md")).unwrap();
    assert!(react_state.contains("TanStack Query"));
    assert!(react_state.contains("query key"));
    let react_migration = fs::read_to_string(frontend_root.join("react/migration.md")).unwrap();
    assert!(react_migration.contains("componentDidCatch"));
    assert!(react_migration.contains("证明等价的行为断言"));
    let react_performance = fs::read_to_string(frontend_root.join("react/performance.md")).unwrap();
    assert!(react_performance.contains("代表性工作负载"));
    assert!(react_performance.contains("虚拟化行"));
    let react_19 = fs::read_to_string(frontend_root.join("react/react19.md")).unwrap();
    assert!(react_19.contains("useActionState"));
    assert!(react_19.contains("稳定操作标识"));
    let react_server =
        fs::read_to_string(frontend_root.join("react/server-components.md")).unwrap();
    assert!(react_server.contains("可序列化交接"));
    assert!(react_server.contains("仅服务端"));
    let react_testing = fs::read_to_string(frontend_root.join("react/testing.md")).unwrap();
    assert!(react_testing.contains("userEvent"));
    assert!(react_testing.contains("Strict Mode"));
    let rn_core = fs::read_to_string(frontend_root.join("react-native/core.md")).unwrap();
    assert!(rn_core.contains("development-client 重建"));
    assert!(rn_core.contains("稳定目标标识"));
    let rn_structure = fs::read_to_string(frontend_root.join("react-native/structure.md")).unwrap();
    assert!(rn_structure.contains("生成的原生输出"));
    assert!(rn_structure.contains("Metro"));
    let rn_navigation =
        fs::read_to_string(frontend_root.join("react-native/navigation.md")).unwrap();
    assert!(rn_navigation.contains("单数/数组形式"));
    assert!(rn_navigation.contains("硬件返回"));
    let rn_platform = fs::read_to_string(frontend_root.join("react-native/platform.md")).unwrap();
    assert!(rn_platform.contains("Platform.select"));
    assert!(rn_platform.contains("永久拒绝"));
    let rn_lists = fs::read_to_string(frontend_root.join("react-native/lists.md")).unwrap();
    assert!(rn_lists.contains("getItemLayout"));
    assert!(rn_lists.contains("onEndReached"));
    let rn_storage = fs::read_to_string(frontend_root.join("react-native/storage.md")).unwrap();
    assert!(rn_storage.contains("schemaVersion"));
    assert!(rn_storage.contains("晚期 hydration"));
    let rn_testing = fs::read_to_string(frontend_root.join("react-native/testing.md")).unwrap();
    assert!(rn_testing.contains("React Native Testing Library"));
    assert!(rn_testing.contains("双平台覆盖"));
    let vue_core = fs::read_to_string(frontend_root.join("vue/core.md")).unwrap();
    assert!(vue_core.contains("watchEffect"));
    assert!(vue_core.contains("effectScope"));
    let vue_components = fs::read_to_string(frontend_root.join("vue/components.md")).unwrap();
    assert!(vue_components.contains("defineModel"));
    assert!(vue_components.contains("InjectionKey"));
    let vue_state = fs::read_to_string(frontend_root.join("vue/state.md")).unwrap();
    assert!(vue_state.contains("storeToRefs"));
    assert!(vue_state.contains("进程全局"));
    let vue_typescript = fs::read_to_string(frontend_root.join("vue/typescript.md")).unwrap();
    assert!(vue_typescript.contains("vue-tsc"));
    assert!(vue_typescript.contains("defineExpose"));
    let vue_nuxt = fs::read_to_string(frontend_root.join("vue/nuxt.md")).unwrap();
    assert!(vue_nuxt.contains("useAsyncData"));
    assert!(vue_nuxt.contains("runtimeConfig.public"));
    let vue_build = fs::read_to_string(frontend_root.join("vue/build.md")).unwrap();
    assert!(vue_build.contains("VITE_*"));
    assert!(vue_build.contains("手动分块"));
    let vue_mobile = fs::read_to_string(frontend_root.join("vue/mobile.md")).unwrap();
    assert!(vue_mobile.contains("Capacitor"));
    assert!(vue_mobile.contains("network-only"));
    let vue_testing = fs::read_to_string(frontend_root.join("vue/testing.md")).unwrap();
    assert!(vue_testing.contains("Vue Test Utils"));
    assert!(vue_testing.contains("测试 Pinia"));
}

#[test]
fn loom_review_references_are_operational_without_protocol_duplication() {
    let repo = repo_root();
    let review_root = repo.join("plugins/shared/loom/references/tech/review");
    let expected = [
        ("core.md", "评审姿态"),
        ("spec-compliance.md", "缺失需求检查"),
        ("defect-patterns.md", "功能正确性"),
        ("test-evidence.md", "强证据"),
        ("finding-quality.md", "发现内容"),
    ];

    for (file, required_section) in expected {
        let path = review_root.join(file);
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read review reference {}: {error}", path.display()));
        let line_count = content.lines().count();
        assert!(
            line_count >= 65,
            "{} is too thin to guide review decisions: {line_count} lines",
            path.display()
        );
        for required in ["使用本引用", required_section] {
            assert!(
                content.contains(required),
                "{} missing review reference section {required}",
                path.display()
            );
        }
        for forbidden in [
            "referenceLoadPlan",
            "readFieldGroup",
            "requestReadPlan",
            "techReferenceProfile",
            "skill_reference_by_group",
            "Full Review Report Template",
            "Code Review: [PR Title]",
            "Receiving Feedback",
            "Load this file only",
        ] {
            assert!(
                !content.contains(forbidden),
                "{} retained protocol, markdown report template, or human-feedback fragment {forbidden}",
                path.display()
            );
        }
    }
    let defect_patterns = fs::read_to_string(review_root.join("defect-patterns.md")).unwrap();
    assert!(defect_patterns.contains("占位符命名空间"));
    assert!(defect_patterns.contains("com.example"));
    assert!(defect_patterns.contains("批量赋值"));
    assert!(defect_patterns.contains("幂等范围"));
    let core = fs::read_to_string(review_root.join("core.md")).unwrap();
    assert!(core.contains("基于风险的深度"));
    assert!(core.contains("变更交互"));
    let spec = fs::read_to_string(review_root.join("spec-compliance.md")).unwrap();
    assert!(spec.contains("契约对检查"));
    assert!(spec.contains("跨面闭环"));
    let evidence = fs::read_to_string(review_root.join("test-evidence.md")).unwrap();
    assert!(evidence.contains("声明映射"));
    assert!(evidence.contains("陈旧证据"));
    let findings = fs::read_to_string(review_root.join("finding-quality.md")).unwrap();
    assert!(findings.contains("按影响的严重性"));
    assert!(findings.contains("根因"));
}

#[test]
fn loom_api_references_preserve_production_contract_depth_without_policy_duplication() {
    let root = repo_root().join("plugins/shared/loom/references/tech/api");
    let contract = fs::read_to_string(root.join("contract.md")).unwrap();
    for required in [
        "Operation 对象",
        "OpenAPI 3.1 Schema 语义",
        "验证和生成",
        "operationId",
        "additionalProperties",
        "不要在 3.1 文档中使用 OpenAPI 3.0 的 `nullable` 关键字",
    ] {
        assert!(
            contract.contains(required),
            "API contract reference missing production rule {required}"
        );
    }

    let resource = fs::read_to_string(root.join("resource.md")).unwrap();
    for required in [
        "方法语义",
        "成功状态和头",
        "`202`",
        "`204`",
        "JSON Merge Patch",
        "Content-Type",
    ] {
        assert!(
            resource.contains(required),
            "API resource reference missing HTTP rule {required}"
        );
    }

    let pagination = fs::read_to_string(root.join("pagination.md")).unwrap();
    for required in [
        "Cursor 和 Keyset 契约",
        "唯一决胜键",
        "不透明的客户端 token",
        "cursor/过滤或 cursor/排序不匹配",
        "格式错误或篡改的 cursor",
    ] {
        assert!(
            pagination.contains(required),
            "API pagination reference missing production rule {required}"
        );
    }

    let errors = fs::read_to_string(root.join("errors.md")).unwrap();
    let operations = fs::read_to_string(root.join("operations.md")).unwrap();
    assert!(errors.contains("错误代码所有权"));
    assert!(errors.contains("本引用拥有错误类别"));
    for duplicated_policy in [
        "## Request Tracking And Retry Guidance",
        "X-Request-ID",
        "Retry-After",
    ] {
        assert!(
            !errors.contains(duplicated_policy),
            "errors.md must not duplicate operational policy {duplicated_policy}"
        );
    }
    assert!(operations.contains("本文件拥有运维策略"));
    assert!(operations.contains("重试和可用性响应"));
    assert!(operations.contains("请求追踪"));
}

#[test]
fn loom_tech_references_do_not_duplicate_mcp_contract_terms() {
    let repo = repo_root();
    let tech_root = repo.join("plugins/shared/loom/references/tech");
    let mut files = Vec::new();
    collect_markdown_files(&tech_root, &mut files);
    let forbidden = [
        "TaskResult",
        "ReviewResult",
        "TaskPlan",
        "AAC",
        "PGC",
        "apiContractEvidence",
        "apiContractRequirements",
        "architectureQualityEvidence",
        "architectureQualityRequirementRefs",
        "architectureQuality.",
        "interfaces[]",
        "interfaceId",
        "type: \"http_api\"",
        "recommendedNextAction",
        "nextAction",
        "execution_repair",
        "taskplan_repair",
        "architecture_artifact_repair",
        "manual_review",
        "continue_to_next_phase",
        "needs_user_decision",
        "approved_with_notes",
        "changes_requested",
        "requestReadPlan",
        "readFieldGroup",
        "referenceLoadPlan",
        "techReferenceProfile",
        "outputContract",
        "resultTemplate",
        "enumRefs",
        "schemaShape",
        "writeTargets",
        "MCP request",
        ".loom",
        "loom.",
    ];

    for path in files {
        let content = fs::read_to_string(&path).unwrap();
        for term in forbidden {
            assert!(
                !content.contains(term),
                "{} must not duplicate MCP contract term {term}",
                path.display()
            );
        }
    }
}

#[test]
fn loom_uix_references_do_not_duplicate_mcp_contract_terms() {
    let repo = repo_root();
    let uix_root = repo.join("plugins/shared/loom/references/uix");
    let mut files = Vec::new();
    collect_markdown_files(&uix_root, &mut files);
    let forbidden = [
        "TaskResult",
        "ReviewResult",
        "TaskPlan",
        "frontendQualitySelfCheck",
        "frontendExperienceRequirement",
        "uiProductionBrief",
        "uiSurfaceDecisionContract",
        "surfaceDecisionContract",
        "surfaceDecisionCandidate",
        "gateResults",
        "referenceGroupsChecked",
        "referenceFilesChecked",
        "referencePlanFilesChecked",
        "surfaceRegionEvidence",
        "surfaceActionEvidence",
        "surfaceStateEvidence",
        "surfaceQualityRuleEvidence",
        "contentBoundaryEvidence",
        "productIntent",
        "layoutContract",
        "informationContract",
        "actionContract",
        "stateContract",
        "visualContract",
        "contentBoundary",
        "mustShow",
        "scanPriority",
        "identityFields",
        "statusFields",
        "longContentPolicy",
        "dataViews",
        "primaryActions",
        "contextualActions",
        "dangerousActions",
        "placementRule",
        "postSuccessUpdate",
        "regionsInScope",
        "actionsInScope",
        "statesInScope",
        "qualityRulesInScope",
        "designTokenEvidence",
        "designTokenAssetPlan",
        "requestRef",
        "requestReadPlan",
        "readFieldGroup",
        "readRequestFields",
        "referenceLoadPlan",
        "outputContract",
        "resultTemplate",
        "resultRules",
        "enumRefs",
        "schemaShape",
        "writeTargets",
        "MCP",
        "Loom",
        "MCP request",
        ".loom",
        "loom.",
    ];

    for path in files {
        let relative = path
            .strip_prefix(&uix_root)
            .expect("relative uix path")
            .to_string_lossy()
            .to_string();
        let content = fs::read_to_string(&path).unwrap();
        for term in forbidden {
            assert!(
                !content.contains(term),
                "{} must not duplicate MCP contract term {term}",
                relative
            );
        }
    }
}

#[test]
fn loom_agent_adapters_use_current_ui_reference_evidence_field() {
    let repo = repo_root();
    let files = [repo.join("plugins/opencode/.opencode/commands/loom.md")];

    for path in files {
        let content = fs::read_to_string(&path).unwrap();
        assert!(
            content.contains("referencePlanFilesChecked"),
            "{} must use the current UI reference evidence field",
            path.display()
        );
        assert!(
            !content.contains("referenceFilesChecked"),
            "{} must not teach the removed UI reference evidence field",
            path.display()
        );
    }
}

fn collect_markdown_files(dir: &Path, output: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_markdown_files(&path, output);
        } else if path.extension().is_some_and(|extension| extension == "md") {
            output.push(path);
        }
    }
}

#[test]
fn deploy_references_explain_profile_and_provider_fallback_without_external_runtime_loading() {
    let repo = repo_root();
    let deploy_refs = repo.join("plugins/shared/loom-deploy/references");
    let providers = fs::read_to_string(deploy_refs.join("providers.md")).unwrap();
    let compose = fs::read_to_string(deploy_refs.join("compose.md")).unwrap();
    let dockerfile = fs::read_to_string(deploy_refs.join("dockerfile.md")).unwrap();
    let environment = fs::read_to_string(deploy_refs.join("environment.md")).unwrap();
    let repair = fs::read_to_string(deploy_refs.join("repair.md")).unwrap();
    let workspaces = fs::read_to_string(deploy_refs.join("workspaces.md")).unwrap();
    let matrix = fs::read_to_string(deploy_refs.join("matrix.md")).unwrap();
    let source_model = fs::read_to_string(deploy_refs.join("source-model.md")).unwrap();
    let topology = fs::read_to_string(deploy_refs.join("topology.md")).unwrap();
    let external =
        fs::read_to_string(repo.join("docs/maintainer/deploy-external-research.md")).unwrap();

    for required in [
        "当用户未强制提供者时，首先尝试现有资产",
        "回退到生成的提供者",
        "当用户显式选择了 `compose-existing` 或 `dockerfile-existing` 时，不允许回退",
    ] {
        assert!(
            providers.contains(required),
            "providers.md missing fallback rule {required}"
        );
    }
    assert!(
        !providers.contains("Do not automatically switch provider after a failure.\n"),
        "providers.md must not contradict unforced generated fallback"
    );

    for required in [
        "DeploymentSpec.runtime.ports",
        "`hostPort` 是 Loom 选择的实际可用本地端口",
        "build.context",
        "build.dockerfile",
        "前端加后端项目",
        "拓扑感知 Compose 契约",
        "publicEntryServiceId",
        "内部后端",
        "多端口",
    ] {
        assert!(
            compose.contains(required),
            "compose.md missing generation guardrail {required}"
        );
    }

    for required in [
        "源根目录、构建上下文、Workdir 与 COPY 闭包",
        "后端服务前端",
        "现有 Dockerfile 包装",
    ] {
        assert!(
            dockerfile.contains(required),
            "dockerfile.md missing context closure guardrail {required}"
        );
    }

    for required in [
        "环境事实流",
        "文件数据库处理并非仅针对 SQLite",
        "服务依赖 URL",
        "框架本地安全默认值",
    ] {
        assert!(
            environment.contains(required),
            "environment.md missing environment guardrail {required}"
        );
    }

    for required in [
        "修复决策树",
        "生成优先修复姿态",
        "sourceModelRef",
        "topologyRef",
        "才询问用户",
        "受保护资产边界",
    ] {
        assert!(
            repair.contains(required),
            "repair.md missing repair posture {required}"
        );
    }

    for required in [
        "应用路径与构建上下文矩阵",
        "选定的应用路径不总是构建上下文",
        "源根修复边界",
    ] {
        assert!(
            workspaces.contains(required),
            "workspaces.md missing workspace matrix {required}"
        );
    }

    for (file, content, required) in [
        (
            "matrix.md",
            &matrix,
            "不要将矩阵折叠为单一“一个容器服务一切”假设",
        ),
        (
            "source-model.md",
            &source_model,
            "`DeploymentSourceModel` 由 Loom 生成，是部署资产生成的权威",
        ),
        (
            "topology.md",
            &topology,
            "`DeploymentTopology` 从 Loom 部署事实生成",
        ),
    ] {
        assert!(
            content.contains(required),
            "{file} missing deploy fact authority statement {required}"
        );
    }

    for name in [
        "node", "java", "python", "go", "dotnet", "php", "ruby", "static",
    ] {
        let content = fs::read_to_string(deploy_refs.join(format!("{name}.md"))).unwrap();
        for required in ["扫描器信号到部署事实", "生成的资产预期", "修复边界"]
        {
            assert!(
                content.contains(required),
                "{name}.md missing stack deploy closure section {required}"
            );
        }
    }

    assert!(
        external.contains("Maintainer-only research note"),
        "deploy external research doc must be clearly maintainer-only"
    );
    assert!(
        !deploy_refs.join("external-references.md").exists(),
        "maintainer research must not live under runtime deploy references"
    );
}

#[test]
fn agent_templates_expose_knowledge_direct_route_and_semantic_pack_discipline() {
    let repo = repo_root();
    let plugin_root = repo.join("plugins");
    let files = ["opencode/.opencode/commands/loom.md"];

    for file in files {
        let content = fs::read_to_string(plugin_root.join(file)).unwrap();
        assert!(
            content.contains("knowledge"),
            "{file} must expose knowledge routing"
        );
        assert!(
            content.contains("loom.knowledge*") || content.contains("loom.knowledgeInspectChunk"),
            "{file} must route knowledge through MCP tools"
        );
        if file.contains("SKILL.md") || file.contains("opencode") {
            for required in [
                "GenerateKnowledgeSemanticsNext",
                "loom.knowledgeInspectChunk",
                "loom.knowledgeSemanticSubmitFile",
            ] {
                assert!(content.contains(required), "{file} missing {required}");
            }
        }
    }
}

#[test]
fn agent_templates_expose_run_loom_tool_next_discipline() {
    let repo = repo_root();
    let plugin_root = repo.join("plugins");
    let files = ["opencode/.opencode/commands/loom.md"];

    for file in files {
        let content = fs::read_to_string(plugin_root.join(file)).unwrap();
        for required in [
            "RunLoomToolNext",
            "检查 requestRef",
            "仅读取返回的 readGroups",
            "调用返回的 Loom MCP 工具",
            "重试返回的 retryTool",
        ] {
            assert!(content.contains(required), "{file} missing {required}");
        }
    }

    let opencode_plugin =
        fs::read_to_string(plugin_root.join("opencode/.opencode/plugins/loom.js")).unwrap();
    for required in [
        "run_loom_tool",
        "仅读取返回的 readGroups",
        "重试返回的 retryTool",
    ] {
        assert!(
            opencode_plugin.contains(required),
            "opencode plugin missing auto-continue prompt discipline {required}"
        );
    }
}

#[test]
fn product_docs_do_not_expose_legacy_install_or_protocol_paths() {
    let repo = repo_root();
    let files = ["README.md", "scripts/README.md", "tests/README.md"];
    let forbidden = [
        "npm run plugin:",
        "loom-cli",
        "LOOM_AGENT_PROFILE",
        "LOOM_COMPACT_OUTPUT",
        "commandInvocation",
        "submitCommand.argv",
        "retryCommand.argv",
        "CLI envelope",
        "dist/cli.js",
        "agent-neutral CLI",
        "next-task",
        "readCommand.argv",
        "agentAction.read",
        ".refs",
    ];
    for file in files {
        let path = repo.join(file);
        let content = fs::read_to_string(&path).unwrap();
        for term in forbidden {
            assert!(
                !content.contains(term),
                "{} must not contain legacy product term {term}",
                path.display()
            );
        }
    }
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

struct Fixture {
    root: PathBuf,
    user_home: PathBuf,
    loom_home: PathBuf,
    package_root: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("loom-setup-{name}-{unique}"));
        let user_home = root.join("home");
        let loom_home = user_home.join(".loom");
        let package_root = root.join("package");
        fs::create_dir_all(&package_root).unwrap();
        Self {
            root,
            user_home,
            loom_home,
            package_root,
        }
    }

    fn env(&self) -> SetupEnvironment {
        SetupEnvironment::for_test(
            self.user_home.clone(),
            self.loom_home.clone(),
            self.package_root.clone(),
        )
    }

    fn write_package(&self) {
        write_file(
            &self.package_root.join("bin/loom-mcp-server"),
            "#!/bin/sh\n",
        );
        write_file(&self.package_root.join("bin/loom-setup"), "#!/bin/sh\n");
        fs::create_dir_all(self.package_root.join("python/runtime")).unwrap();
        write_file(
            &self.package_root.join("python/algorithms/worker.py"),
            "print('{\"ok\": true}')\n",
        );
        self.write_opencode_template();
        self.write_shared_references();
        let manifest = ReleaseManifest::for_platform(TargetPlatform::DarwinArm64);
        write_json(&self.package_root.join("manifest.json"), &manifest);
        self.write_checksums();
    }

    fn write_opencode_template(&self) {
        write_file(
            &self
                .package_root
                .join("plugins/opencode/.opencode/commands/loom.md"),
            "Loom MCP-only OpenCode command\n",
        );
        write_file(
            &self
                .package_root
                .join("plugins/opencode/.opencode/commands/loom-deploy.md"),
            "Loom MCP-only OpenCode deploy command\n",
        );
        write_file(
            &self
                .package_root
                .join("plugins/opencode/.opencode/plugins/loom.js"),
            "export const LoomPlugin = async () => ({});\n",
        );
    }

    fn write_shared_references(&self) {
        for name in [
            "anti-patterns",
            "content",
            "core",
            "data",
            "frameworks",
            "interaction",
            "mobile",
            "system",
            "verification",
        ] {
            write_file(
                &self
                    .package_root
                    .join(format!("plugins/shared/loom/references/uix/{name}.md")),
                &format!("# {name} Reference\n"),
            );
        }
        for path in [
            "scenarios/admin-dashboard",
            "scenarios/consumer-app",
            "scenarios/corporate-site",
            "scenarios/data-console",
            "scenarios/developer-tool",
            "scenarios/docs-site",
            "scenarios/fintech-consumer-app",
            "scenarios/fintech-workstation",
            "scenarios/immersive-3d",
            "scenarios/marketing-site",
            "scenarios/mobile-native",
            "scenarios/mobile-responsive",
            "stacks/native-mobile",
            "stacks/plain-html",
            "stacks/react",
            "stacks/svelte",
            "stacks/threejs",
            "stacks/uniapp",
            "stacks/vue",
            "tokens/color-system",
            "tokens/layout-grid",
            "tokens/motion",
            "tokens/radius-elevation",
            "tokens/spacing",
            "tokens/typography",
        ] {
            write_file(
                &self
                    .package_root
                    .join(format!("plugins/shared/loom/references/uix/{path}.md")),
                &format!("# {path} Reference\n"),
            );
        }
        for path in ["templates/tokens.css.tpl", "templates/tokens.tailwind.tpl"] {
            write_file(
                &self
                    .package_root
                    .join(format!("plugins/shared/loom/references/uix/{path}")),
                &format!("/* {path} */\n"),
            );
        }
        for path in [
            "adr", "core", "data", "failure", "nfr", "patterns", "system",
        ] {
            write_file(
                &self.package_root.join(format!(
                    "plugins/shared/loom/references/tech/arch/{path}.md"
                )),
                &format!("# {path} Architecture Reference\n"),
            );
        }
        for path in [
            "contract",
            "core",
            "errors",
            "evolution",
            "jwt",
            "operations",
            "pagination",
            "resource",
            "security",
        ] {
            write_file(
                &self
                    .package_root
                    .join(format!("plugins/shared/loom/references/tech/api/{path}.md")),
                &format!("# {path} API Reference\n"),
            );
        }
        for path in REVIEW_REFERENCE_FILES {
            write_file(
                &self.package_root.join(format!(
                    "plugins/shared/loom/references/tech/review/{path}.md"
                )),
                &format!("# {path} Review Reference\n"),
            );
        }
        for path in PLAYWRIGHT_REFERENCE_FILES {
            write_file(
                &self.package_root.join(format!(
                    "plugins/shared/loom/references/tech/test/playwright/{path}.md"
                )),
                &format!("# {path} Playwright Reference\n"),
            );
        }
        write_file(
            &self
                .package_root
                .join("plugins/shared/loom/references/tech/code/common.md"),
            "# Common Code Reference\n",
        );
        for path in CODE_REFERENCE_FILES {
            write_file(
                &self.package_root.join(format!(
                    "plugins/shared/loom/references/tech/code/{path}.md"
                )),
                &format!("# {path} Code Reference\n"),
            );
        }
        for path in [
            "redis/core",
            "redis/cache",
            "redis/session",
            "redis/atomicity",
            "redis/messaging",
        ] {
            write_file(
                &self.package_root.join(format!(
                    "plugins/shared/loom/references/tech/code/{path}.md"
                )),
                &format!("# {path} Code Reference\n"),
            );
        }
        for path in BACKEND_REFERENCE_FILES {
            write_file(
                &self.package_root.join(format!(
                    "plugins/shared/loom/references/tech/backend/{path}.md"
                )),
                &format!("# {path} Backend Reference\n"),
            );
        }
        for path in FRONTEND_REFERENCE_FILES {
            write_file(
                &self.package_root.join(format!(
                    "plugins/shared/loom/references/tech/frontend/{path}.md"
                )),
                &format!("# {path} Frontend Reference\n"),
            );
        }
        for name in [
            "bootstrap",
            "compose",
            "dockerfile",
            "dotnet",
            "environment",
            "go",
            "java",
            "matrix",
            "node",
            "php",
            "providers",
            "python",
            "redis",
            "repair",
            "ruby",
            "source-model",
            "static",
            "topology",
            "workspaces",
        ] {
            write_file(
                &self
                    .package_root
                    .join(format!("plugins/shared/loom-deploy/references/{name}.md")),
                &format!("# {name} Reference\n"),
            );
        }
    }

    fn write_checksums(&self) {
        let mut files = collect_files(&self.package_root);
        files.retain(|path| {
            path.file_name().and_then(|name| name.to_str()) != Some("checksums.txt")
        });
        let mut lines = Vec::new();
        for file in files {
            let relative = file.strip_prefix(&self.package_root).unwrap();
            lines.push(format!("{}  {}", sha256(&file), relative.to_string_lossy()));
        }
        lines.sort();
        write_file(
            &self.package_root.join("checksums.txt"),
            &format!("{}\n", lines.join("\n")),
        );
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn write_file(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn write_json<T: serde::Serialize>(path: &Path, value: &T) {
    write_file(
        path,
        &format!("{}\n", serde_json::to_string_pretty(value).unwrap()),
    );
}

fn read_json(path: &Path) -> serde_json::Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn path_string_for_test(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn collect_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            out.extend(collect_files(&path));
        } else if path.is_file() {
            out.push(path);
        }
    }
    out
}

fn sha256(path: &Path) -> String {
    let bytes = fs::read(path).unwrap();
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}
