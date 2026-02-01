//! Integration tests for the container execution layer.
//!
//! These tests verify the container runner functionality using mocked
//! runners to avoid requiring actual Docker/Podman installation.

use std::path::PathBuf;

use mity_runner::{
    CliRunnerOptions, ContainerConfig, ContainerRunner, ContainerRuntime,
    MockResponse, MockRunner, MountConfig, RunConfig, RunnerError, Stack, StackConfigBuilder,
    StackRegistry, presets,
};

/// Test basic mock runner functionality.
#[tokio::test]
async fn test_mock_runner_basic_execution() {
    let runner = MockRunner::new()
        .add_response(MockResponse::success("test passed"));

    let config = ContainerConfig::new("python")
        .tag("3.12-slim")
        .workdir("/app")
        .command(vec!["pytest".to_string()]);

    let result = runner.run_container(&config, &RunConfig::default()).await.unwrap();

    assert_eq!(result.exit_code, 0);
    assert_eq!(result.stdout, "test passed");
    assert!(result.success());
}

/// Test mock runner captures all call details.
#[tokio::test]
async fn test_mock_runner_captures_config() {
    let runner = MockRunner::new();

    let config = ContainerConfig::new("python")
        .tag("3.12-slim")
        .workdir("/workspace")
        .env("CI", "true")
        .env("PYTHONUNBUFFERED", "1")
        .command(vec!["python".to_string(), "-m".to_string(), "pytest".to_string()]);

    let _ = runner.run_container(&config, &RunConfig::default()).await;

    let calls = runner.get_method_calls("run_container");
    assert_eq!(calls.len(), 1);

    let call = &calls[0];
    assert_eq!(call.image, Some("python:3.12-slim".to_string()));
    assert_eq!(call.workdir, Some("/workspace".to_string()));
    assert_eq!(call.command, Some(vec![
        "python".to_string(),
        "-m".to_string(),
        "pytest".to_string(),
    ]));

    let env = call.env.as_ref().unwrap();
    assert_eq!(env.get("CI"), Some(&"true".to_string()));
    assert_eq!(env.get("PYTHONUNBUFFERED"), Some(&"1".to_string()));
}

/// Test sequential execution with multiple responses.
#[tokio::test]
async fn test_mock_runner_sequential_responses() {
    let runner = MockRunner::new().with_responses(vec![
        MockResponse::success("step 1 output"),
        MockResponse::success("step 2 output"),
        MockResponse::failure(1, "step 3 failed"),
    ]);

    let config = ContainerConfig::new("test").command(vec!["test".to_string()]);
    let run_config = RunConfig::default();

    // First call
    let r1 = runner.run_container(&config, &run_config).await.unwrap();
    assert!(r1.success());
    assert_eq!(r1.stdout, "step 1 output");

    // Second call
    let r2 = runner.run_container(&config, &run_config).await.unwrap();
    assert!(r2.success());
    assert_eq!(r2.stdout, "step 2 output");

    // Third call (failure)
    let r3 = runner.run_container(&config, &run_config).await.unwrap();
    assert!(!r3.success());
    assert_eq!(r3.exit_code, 1);
    assert_eq!(r3.stderr, "step 3 failed");
}

/// Test image pull workflow.
#[tokio::test]
async fn test_mock_runner_image_workflow() {
    let runner = MockRunner::new()
        .add_existing_image("python:3.12-slim");

    // Check existing image
    assert!(runner.image_exists("python", "3.12-slim").await.unwrap());
    assert!(!runner.image_exists("node", "20-slim").await.unwrap());

    // Pull new image
    runner.pull_image("node", "20-slim").await.unwrap();

    // Now it should exist
    assert!(runner.image_exists("node", "20-slim").await.unwrap());

    // Verify calls
    assert!(runner.was_called("image_exists"));
    assert!(runner.was_called("pull_image"));
}

/// Test runner availability check.
#[tokio::test]
async fn test_mock_runner_availability() {
    let available_runner = MockRunner::new().set_available(true);
    assert!(available_runner.is_available().await.unwrap());

    let unavailable_runner = MockRunner::new().set_available(false);
    assert!(!unavailable_runner.is_available().await.unwrap());
}

/// Test failure simulation.
#[tokio::test]
async fn test_mock_runner_failure_simulation() {
    let runner = MockRunner::new()
        .simulate_failure("simulated failure");

    let config = ContainerConfig::new("test").command(vec!["test".to_string()]);
    let result = runner.run_container(&config, &RunConfig::default()).await;

    assert!(result.is_err());
    if let Err(RunnerError::ExecutionFailed(msg)) = result {
        assert!(msg.contains("simulated"));
    } else {
        panic!("Expected ExecutionFailed error");
    }
}

/// Test stack registry defaults.
#[tokio::test]
async fn test_stack_registry_defaults() {
    let registry = StackRegistry::new();

    // Python
    let python = registry.get(Stack::Python).unwrap();
    assert_eq!(python.image, "python");
    assert_eq!(python.tag, "3.12-slim");
    assert_eq!(python.default_workdir, "/app");
    assert_eq!(python.default_env.get("PYTHONUNBUFFERED"), Some(&"1".to_string()));

    // Node
    let node = registry.get(Stack::Node).unwrap();
    assert_eq!(node.image, "node");
    assert_eq!(node.tag, "20-slim");

    // Rust
    let rust = registry.get(Stack::Rust).unwrap();
    assert_eq!(rust.image, "rust");
    assert_eq!(rust.tag, "1.75-slim");

    // All stacks should be present
    for stack in Stack::all() {
        assert!(registry.get(*stack).is_some(), "Stack {:?} missing", stack);
    }
}

/// Test stack config builder.
#[tokio::test]
async fn test_stack_config_builder() {
    let registry = StackRegistry::new();

    let config = StackConfigBuilder::new(&registry, Stack::Python)
        .command(vec!["pytest".to_string(), "-v".to_string()])
        .env("COVERAGE", "true")
        .mount(PathBuf::from("/project"), "/app")
        .build()
        .unwrap();

    assert_eq!(config.image, "python");
    assert_eq!(config.tag, "3.12-slim");
    assert_eq!(config.command, vec!["pytest", "-v"]);
    assert_eq!(config.env.get("PYTHONUNBUFFERED"), Some(&"1".to_string()));
    assert_eq!(config.env.get("COVERAGE"), Some(&"true".to_string()));
}

/// Test preset configurations.
#[tokio::test]
async fn test_presets() {
    let registry = StackRegistry::new();
    let project_path = PathBuf::from("/test/project");

    // Python pytest
    let pytest = presets::python_pytest(&registry, project_path.clone()).unwrap();
    assert_eq!(pytest.image, "python");
    assert!(pytest.command.contains(&"pytest".to_string()));
    assert!(pytest.command.contains(&"-v".to_string()));

    // Python pytest with coverage
    let pytest_cov = presets::python_pytest_cov(&registry, project_path.clone()).unwrap();
    assert!(pytest_cov.command.contains(&"--cov=.".to_string()));

    // Node npm test
    let npm_test = presets::node_npm_test(&registry, project_path.clone()).unwrap();
    assert_eq!(npm_test.image, "node");
    assert!(npm_test.command.contains(&"npm".to_string()));
    assert!(npm_test.command.contains(&"test".to_string()));

    // Rust cargo test
    let cargo_test = presets::rust_cargo_test(&registry, project_path.clone()).unwrap();
    assert_eq!(cargo_test.image, "rust");
    assert!(cargo_test.command.contains(&"cargo".to_string()));

    // Terraform plan
    let tf_plan = presets::terraform_plan(&registry, project_path.clone()).unwrap();
    assert!(tf_plan.image.contains("terraform"));

    // Trivy scan
    let trivy = presets::trivy_scan(&registry, project_path.clone()).unwrap();
    assert!(trivy.image.contains("trivy"));
}

/// Test custom tag override.
#[tokio::test]
async fn test_custom_tag_override() {
    let mut registry = StackRegistry::new();
    
    // Override Python to use 3.11
    registry.set_tag(Stack::Python, "3.11-slim");

    let python = registry.get(Stack::Python).unwrap();
    assert_eq!(python.tag, "3.11-slim");

    // Build config with custom tag
    let config = StackConfigBuilder::new(&registry, Stack::Python)
        .command(vec!["python".to_string(), "--version".to_string()])
        .build()
        .unwrap();

    assert_eq!(config.tag, "3.11-slim");
}

/// Test workflow simulation with mock runner.
#[tokio::test]
async fn test_full_workflow_simulation() {
    let runner = MockRunner::new()
        .add_existing_image("python:3.12-slim")
        .with_responses(vec![
            MockResponse::success("Installing dependencies..."),
            MockResponse::success("Running linter...\nAll checks passed!"),
            MockResponse::success("Running tests...\n5 passed, 0 failed"),
            MockResponse::success("Building package..."),
        ]);

    let base_config = ContainerConfig::new("python")
        .tag("3.12-slim")
        .workdir("/app");

    // Step 1: Install dependencies
    let install_config = base_config.clone()
        .command(vec!["pip".to_string(), "install".to_string(), "-r".to_string(), "requirements.txt".to_string()]);
    let r1 = runner.run_container(&install_config, &RunConfig::default()).await.unwrap();
    assert!(r1.success());

    // Step 2: Lint
    let lint_config = base_config.clone()
        .command(vec!["ruff".to_string(), "check".to_string(), ".".to_string()]);
    let r2 = runner.run_container(&lint_config, &RunConfig::default()).await.unwrap();
    assert!(r2.success());
    assert!(r2.stdout.contains("All checks passed"));

    // Step 3: Test
    let test_config = base_config.clone()
        .command(vec!["pytest".to_string()]);
    let r3 = runner.run_container(&test_config, &RunConfig::default()).await.unwrap();
    assert!(r3.success());
    assert!(r3.stdout.contains("5 passed"));

    // Step 4: Build
    let build_config = base_config.clone()
        .command(vec!["python".to_string(), "-m".to_string(), "build".to_string()]);
    let r4 = runner.run_container(&build_config, &RunConfig::default()).await.unwrap();
    assert!(r4.success());

    // Verify all steps executed
    assert_eq!(runner.call_count(), 4);
}

/// Test container config builder.
#[tokio::test]
async fn test_container_config_builder() {
    let config = ContainerConfig::new("python")
        .tag("3.12-slim")
        .workdir("/app")
        .env("PYTHONUNBUFFERED", "1")
        .env("DEBUG", "true")
        .mount(MountConfig::new(PathBuf::from("/src"), "/app"))
        .mount(MountConfig::new(PathBuf::from("/cache"), "/cache").read_only())
        .command(vec!["pytest".to_string(), "-v".to_string()])
        .auto_remove(true);

    assert_eq!(config.image, "python");
    assert_eq!(config.tag, "3.12-slim");
    assert_eq!(config.workdir, Some("/app".to_string()));
    assert_eq!(config.env.len(), 2);
    assert_eq!(config.mounts.len(), 2);
    assert!(config.mounts[1].read_only);
    assert!(config.auto_remove);
    assert_eq!(config.full_image(), "python:3.12-slim");
}

/// Test run config builder.
#[tokio::test]
async fn test_run_config_builder() {
    let config = RunConfig::default()
        .timeout(120)
        .stream_logs(true)
        .auto_pull(true)
        .memory_limit(512 * 1024 * 1024) // 512MB
        .cpu_limit(2.0);

    assert_eq!(config.timeout_seconds, 120);
    assert!(config.stream_logs);
    assert!(config.pull_image);
    assert_eq!(config.memory_limit, Some(512 * 1024 * 1024));
    assert_eq!(config.cpu_limit, Some(2.0));
}

/// Test execution result methods.
#[tokio::test]
async fn test_execution_result_methods() {
    let runner = MockRunner::new().with_responses(vec![
        MockResponse {
            exit_code: 0,
            stdout: "stdout content".to_string(),
            stderr: "stderr content".to_string(),
            duration_ms: 1500,
        },
    ]);

    let config = ContainerConfig::new("test").command(vec!["test".to_string()]);
    let result = runner.run_container(&config, &RunConfig::default()).await.unwrap();

    assert!(result.success());
    assert!(result.combined_output().contains("stdout content"));
    assert!(result.combined_output().contains("stderr content"));
}

/// Test CLI runner options builder.
#[test]
fn test_cli_runner_options_builder() {
    let opts = CliRunnerOptions::new()
        .dry_run()
        .ci_mode()
        .prefer_docker()
        .fail_fast(true);

    assert!(opts.dry_run);
    assert!(opts.ci_mode);
    assert_eq!(opts.preferred_runtime, Some(ContainerRuntime::Docker));
    assert!(opts.fail_fast);

    let opts2 = CliRunnerOptions::new()
        .prefer_podman()
        .fail_fast(false);

    assert_eq!(opts2.preferred_runtime, Some(ContainerRuntime::Podman));
    assert!(!opts2.fail_fast);
}

/// Test container runtime display.
#[test]
fn test_container_runtime_display() {
    assert_eq!(ContainerRuntime::Docker.command(), "docker");
    assert_eq!(ContainerRuntime::Podman.command(), "podman");
    assert_eq!(format!("{}", ContainerRuntime::Docker), "docker");
    assert_eq!(format!("{}", ContainerRuntime::Podman), "podman");
}

/// Test call tracking with counter.
#[tokio::test]
async fn test_call_tracking() {
    let runner = MockRunner::new();
    let config = ContainerConfig::new("test").command(vec!["test".to_string()]);
    let run_config = RunConfig::default();

    // Make several calls
    let _ = runner.is_available().await;
    let _ = runner.version().await;
    let _ = runner.run_container(&config, &run_config).await;
    let _ = runner.run_container(&config, &run_config).await;

    // Check call counts
    assert_eq!(runner.call_count(), 4);
    assert_eq!(runner.get_method_calls("is_available").len(), 1);
    assert_eq!(runner.get_method_calls("version").len(), 1);
    assert_eq!(runner.get_method_calls("run_container").len(), 2);

    // Clear and verify
    runner.clear_calls();
    assert_eq!(runner.call_count(), 0);
}
