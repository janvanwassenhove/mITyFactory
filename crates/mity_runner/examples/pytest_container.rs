//! Example: Running pytest in a container
//!
//! This example demonstrates how to run Python tests inside a container
//! using the mity_runner container execution layer.
//!
//! Run with: cargo run --example pytest_container

use mity_runner::{
    CliRunner, CliRunnerOptions, ContainerConfig, ContainerRunner, MountConfig, RunConfig,
    StackRegistry, presets, Stack,
};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== mITyFactory Container Runner Example ===\n");

    // Method 1: Using the Stack preset
    println!("Method 1: Using Stack presets");
    println!("------------------------------");
    run_with_preset().await?;

    println!("\nMethod 2: Manual configuration");
    println!("------------------------------");
    run_with_manual_config().await?;

    println!("\nMethod 3: Dry-run mode");
    println!("------------------------------");
    run_dry_mode().await?;

    Ok(())
}

/// Run pytest using the preset configuration.
async fn run_with_preset() -> anyhow::Result<()> {
    let registry = StackRegistry::new();
    
    // Show what image would be used
    if let Some(python_stack) = registry.get(Stack::Python) {
        println!("Python stack image: {}", python_stack.full_image());
        println!("Default workdir: {}", python_stack.default_workdir);
        println!("Default env vars: {:?}", python_stack.default_env);
    }

    // Create a pytest configuration
    let project_path = PathBuf::from(".");
    if let Some(config) = presets::python_pytest(&registry, project_path) {
        println!("\nGenerated container config:");
        println!("  Image: {}:{}", config.image, config.tag);
        println!("  Command: {:?}", config.command);
        println!("  Workdir: {:?}", config.workdir);
    }

    Ok(())
}

/// Run pytest with manual configuration.
async fn run_with_manual_config() -> anyhow::Result<()> {
    // Try to create a runner (will fail if Docker/Podman not available)
    let runner = match CliRunner::new(CliRunnerOptions::default()) {
        Ok(r) => {
            println!("Detected runtime: {:?}", r.runtime());
            r
        }
        Err(e) => {
            println!("Container runtime not available: {}", e);
            println!("Showing configuration that would be used...\n");
            
            // Still show the configuration
            let config = ContainerConfig::new("python")
                .tag("3.12-slim")
                .workdir("/app")
                .env("PYTHONUNBUFFERED", "1")
                .env("PYTHONDONTWRITEBYTECODE", "1")
                .mount(MountConfig::new(
                    PathBuf::from("."),
                    "/app",
                ))
                .command(vec![
                    "python".to_string(),
                    "-m".to_string(),
                    "pytest".to_string(),
                    "-v".to_string(),
                    "--tb=short".to_string(),
                ]);
            
            println!("Container configuration:");
            println!("  Image: {}:{}", config.image, config.tag);
            println!("  Command: {:?}", config.command);
            println!("  Environment:");
            for (k, v) in &config.env {
                println!("    {}={}", k, v);
            }
            
            return Ok(());
        }
    };

    // Check version
    match runner.version().await {
        Ok(version) => println!("Runtime version: {}", version),
        Err(e) => println!("Could not get version: {}", e),
    }

    // Configuration for pytest
    let config = ContainerConfig::new("python")
        .tag("3.12-slim")
        .workdir("/app")
        .env("PYTHONUNBUFFERED", "1")
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .mount(MountConfig::new(
            std::env::current_dir()?,
            "/app",
        ))
        .command(vec![
            "python".to_string(),
            "-m".to_string(),
            "pytest".to_string(),
            "-v".to_string(),
            "--tb=short".to_string(),
        ]);

    let _run_config = RunConfig::default()
        .timeout(300) // 5 minutes
        .stream_logs(true)
        .auto_pull(true);

    println!("\nWould run: python -m pytest -v --tb=short");
    println!("In container: {}:{}", config.image, config.tag);

    // Note: Actual execution would happen here with:
    // let result = runner.run_container(&config, &run_config).await?;

    Ok(())
}

/// Demonstrate dry-run mode.
async fn run_dry_mode() -> anyhow::Result<()> {
    // Create runner in dry-run mode (doesn't need actual Docker/Podman)
    let options = CliRunnerOptions::new()
        .dry_run()
        .ci_mode();

    // Use with_runtime to avoid detection in dry-run
    use mity_runner::ContainerRuntime;
    let runner = CliRunner::with_runtime(ContainerRuntime::Docker, options);

    println!("Dry-run mode: {}", runner.is_dry_run());

    let config = ContainerConfig::new("python")
        .tag("3.12-slim")
        .workdir("/app")
        .env("PYTHONUNBUFFERED", "1")
        .mount(MountConfig::new(
            PathBuf::from("/project"),
            "/app",
        ))
        .command(vec![
            "python".to_string(),
            "-m".to_string(),
            "pytest".to_string(),
            "-v".to_string(),
        ]);

    let run_config = RunConfig::default();

    // This will just print the command without executing
    let result = runner.run_container(&config, &run_config).await?;

    println!("\nDry-run result:");
    println!("  Exit code: {}", result.exit_code);
    println!("  Output: {}", result.stdout);

    Ok(())
}
