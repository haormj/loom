use planning::technical_baseline::RepoSignalEngine;
use reference_catalog::resolved_catalog;
use std::fs;

fn temp_project(name: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("create temp dir");
    let _ = name;
    dir
}

#[test]
fn detects_python_pyproject() {
    let dir = temp_project("python-pyproject");
    fs::write(
        dir.path().join("pyproject.toml"),
        "[project]\nname = 'test'\n",
    )
    .expect("write");
    fs::write(dir.path().join("requirements.txt"), "fastapi\n").expect("write");

    let catalog = resolved_catalog();
    let engine = RepoSignalEngine {
        config: &catalog.repo_signals,
    };
    let signals = engine.collect(dir.path());

    let json = signals.to_json();
    assert!(json["languages"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "Python"));
    assert!(json["manifests"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "pyproject.toml"));
    assert!(json["manifests"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "requirements.txt"));
    assert!(json["packageManagers"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "pip"));
    assert!(json["frameworks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "FastAPI"));
}

#[test]
fn detects_python_setup_py() {
    let dir = temp_project("python-setup-py");
    fs::write(
        dir.path().join("setup.py"),
        "from setuptools import setup\n",
    )
    .expect("write");

    let catalog = resolved_catalog();
    let engine = RepoSignalEngine {
        config: &catalog.repo_signals,
    };
    let signals = engine.collect(dir.path());

    let json = signals.to_json();
    assert!(json["languages"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "Python"));
    assert!(json["manifests"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "setup.py"));
}

#[test]
fn detects_python_via_extension_scan() {
    let dir = temp_project("python-no-manifest");
    let src = dir.path().join("src");
    fs::create_dir_all(&src).expect("mkdir");
    fs::write(src.join("main.py"), "print('hello')\n").expect("write");
    fs::write(src.join("utils.py"), "def helper():\n    pass\n").expect("write");

    let catalog = resolved_catalog();
    let engine = RepoSignalEngine {
        config: &catalog.repo_signals,
    };
    let signals = engine.collect(dir.path());

    let json = signals.to_json();
    assert!(json["languages"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "Python"));
}

#[test]
fn detects_node_with_react() {
    let dir = temp_project("node-react");
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies": {"react": "^18.0.0", "next": "^14.0.0"}}"#,
    )
    .expect("write");

    let catalog = resolved_catalog();
    let engine = RepoSignalEngine {
        config: &catalog.repo_signals,
    };
    let signals = engine.collect(dir.path());

    let json = signals.to_json();
    assert!(json["languages"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "JavaScript"));
    assert!(json["frameworks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "React"));
    assert!(json["frameworks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "Next.js"));
    assert!(json["packageManagers"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "npm"));
}

#[test]
fn detects_java_spring_boot() {
    let dir = temp_project("java-spring");
    fs::write(
        dir.path().join("pom.xml"),
        "<project><dependencies><dependency><groupId>org.springframework.boot</groupId><artifactId>spring-boot-starter-web</artifactId></dependency></dependencies></project>",
    )
    .expect("write");

    let catalog = resolved_catalog();
    let engine = RepoSignalEngine {
        config: &catalog.repo_signals,
    };
    let signals = engine.collect(dir.path());

    let json = signals.to_json();
    assert!(json["languages"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "Java"));
    assert!(json["frameworks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "Spring Boot"));
}

#[test]
fn detects_rust_cargo() {
    let dir = temp_project("rust-cargo");
    fs::write(dir.path().join("Cargo.toml"), "[package]\nname = 'test'\n").expect("write");

    let catalog = resolved_catalog();
    let engine = RepoSignalEngine {
        config: &catalog.repo_signals,
    };
    let signals = engine.collect(dir.path());

    let json = signals.to_json();
    assert!(json["languages"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "Rust"));
}

#[test]
fn detects_go_mod() {
    let dir = temp_project("go-mod");
    fs::write(
        dir.path().join("go.mod"),
        "module github.com/test\n\ngo 1.21\n",
    )
    .expect("write");

    let catalog = resolved_catalog();
    let engine = RepoSignalEngine {
        config: &catalog.repo_signals,
    };
    let signals = engine.collect(dir.path());

    let json = signals.to_json();
    assert!(json["languages"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "Go"));
}

#[test]
fn detects_source_roots() {
    let dir = temp_project("source-roots");
    fs::create_dir_all(dir.path().join("src")).expect("mkdir");
    fs::create_dir_all(dir.path().join("tests")).expect("mkdir");

    let catalog = resolved_catalog();
    let engine = RepoSignalEngine {
        config: &catalog.repo_signals,
    };
    let signals = engine.collect(dir.path());

    let json = signals.to_json();
    assert!(json["sourceRoots"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "src"));
    assert!(json["sourceRoots"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "tests"));
}

#[test]
fn detects_multi_stack() {
    let dir = temp_project("multi-stack");
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies": {"react": "^18"}}"#,
    )
    .expect("write");
    fs::write(dir.path().join("requirements.txt"), "django\n").expect("write");

    let catalog = resolved_catalog();
    let engine = RepoSignalEngine {
        config: &catalog.repo_signals,
    };
    let signals = engine.collect(dir.path());

    let json = signals.to_json();
    assert!(json["languages"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "JavaScript"));
    assert!(json["languages"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "Python"));
}
