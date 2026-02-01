# Workflow Engine

The mITyFactory workflow engine provides a deterministic orchestration system for executing SDLC workflows as a sequence of stations.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   WorkflowExecutor                       │
├─────────────────────────────────────────────────────────┤
│  ┌───────────────┐    ┌───────────────────────────────┐ │
│  │ StationRegistry│    │      ExecutionLog            │ │
│  │               │    │ ┌───────┐ ┌───────┐ ┌───────┐│ │
│  │ "scaffold" ──▶│    │ │Station│ │Station│ │Station││ │
│  │ "validate" ──▶│    │ │Result │ │Result │ │Result ││ │
│  │ "commit"   ──▶│    │ └───────┘ └───────┘ └───────┘│ │
│  └───────────────┘    └───────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

## Core Components

### Station

A station is a single unit of work in a workflow. Each station:
- Has a unique name
- Declares its inputs and outputs
- Executes work and returns a result
- Can declare dependencies on other stations

```rust
use async_trait::async_trait;
use mity_core::{
    Station, StationInput, StationOutput, StationResult,
    WorkflowContext, CoreResult,
};

struct MyStation;

#[async_trait]
impl Station for MyStation {
    fn name(&self) -> &str {
        "my-station"
    }

    fn description(&self) -> &str {
        "Does something useful"
    }

    fn input(&self) -> StationInput {
        StationInput::new()
            .require_key("previous_output")
            .optional_key("config")
    }

    fn output(&self) -> StationOutput {
        StationOutput::new()
            .produces_key("my_result")
            .produces_artifact("output-file")
    }

    fn dependencies(&self) -> Vec<String> {
        vec!["previous-station".to_string()]
    }

    async fn execute(&self, context: &mut WorkflowContext) -> CoreResult<StationResult> {
        // Read input from context
        let input: String = context.get_input("previous_output")
            .unwrap_or_default();

        // Do work...

        // Set output for next stations
        context.set_output("my_result", serde_json::json!("success"));

        Ok(StationResult::success("my-station"))
    }
}
```

### StationRegistry

The registry maps station names to implementations:

```rust
use std::sync::Arc;
use mity_core::StationRegistry;

let mut registry = StationRegistry::new();
registry.register(Arc::new(MyStation));

// Retrieve by name
let station = registry.get("my-station").unwrap();
```

### Workflow

A workflow defines an ordered sequence of stations to execute:

```rust
use mity_core::Workflow;

let workflow = Workflow::new("create-app", "Create Application")
    .with_description("Creates a new application from a template")
    .station("scaffold")
    .station("validate")
    .station("commit");
```

### WorkflowExecutor

The executor runs workflows with persistence and resume support:

```rust
use std::sync::Arc;
use mity_core::{
    WorkflowExecutor, StationRegistry, Workflow, WorkflowContext,
    StackType,
};

// Create registry and register stations
let mut registry = StationRegistry::new();
registry.register(Arc::new(ScaffoldStation::new()));
registry.register(Arc::new(ValidateStation::new()));
registry.register(Arc::new(CommitStation::new()));

// Create executor
let executor = WorkflowExecutor::new(Arc::new(registry));

// Define workflow
let workflow = Workflow::new("my-workflow", "My Workflow")
    .station("scaffold")
    .station("validate")
    .station("commit");

// Create context
let context = WorkflowContext::new(
    workspace_path,
    "my-app",
    StackType::PythonFastapi,
);

// Execute
let log = executor.execute(&workflow, context).await?;
```

## Execution Flow

1. **Start**: Create a new `ExecutionLog` with `Pending` state
2. **Run**: For each station in order:
   - Look up station in registry
   - Execute station with mutable context
   - Record result in execution log
   - Persist log to disk
   - If failed, stop execution
3. **Complete**: Set state to `Completed` or `Failed`

## Execution States

```
     ┌─────────┐
     │ Pending │
     └────┬────┘
          │ execute()
          ▼
     ┌─────────┐
     │ Running │
     └────┬────┘
          │
    ┌─────┴─────┐
    │           │
    ▼           ▼
┌─────────┐ ┌────────┐
│Completed│ │ Failed │
└─────────┘ └────┬───┘
                 │ resume()
                 ▼
            ┌─────────┐
            │ Running │
            └─────────┘
```

## Re-run from Failed Station

When a workflow fails, the execution log captures the failure point. You can resume:

```rust
// Load failed execution log
let log = executor.load_log(&log_path)?;

if log.can_resume() {
    println!("Failed at station: {}", log.failed_station().unwrap());
    
    // Resume from failed station
    let resumed_log = executor.resume(log).await?;
}
```

## Execution Log Persistence

Logs are persisted to `.mity/logs/{workflow_id}.json` after each station completes.

```json
{
  "workflow_id": "create-app",
  "workflow_name": "Create Application",
  "state": "completed",
  "current_station_index": 3,
  "stations": ["scaffold", "validate", "commit"],
  "results": [
    {
      "station": "scaffold",
      "result": { "success": true, ... }
    },
    ...
  ],
  "started_at": "2024-01-15T10:30:00Z",
  "completed_at": "2024-01-15T10:30:05Z",
  "context": { ... }
}
```

## WorkflowContext

The context carries execution state between stations:

```rust
use mity_core::{WorkflowContext, StackType, IacConfig};

let context = WorkflowContext::new(
    workspace_path,
    "my-app",
    StackType::PythonFastapi,
)
.with_iac(IacConfig::terraform("azure"))
.with_env("API_KEY", "secret")
.with_metadata("custom_field", serde_json::json!({"key": "value"}));

// Access paths
let templates = context.templates_path();
let output = context.output_path;

// Get/set values
context.set_output("key", serde_json::json!("value"));
let val: String = context.get_output("key").unwrap();
```

## Predefined Workflows

```rust
use mity_core::Workflows;

// Create app: scaffold → validate → commit
let create_app = Workflows::create_app();

// Add feature: analyze → architect → implement → test → review → commit
let add_feature = Workflows::add_feature();

// Validation: validate → secure
let validate = Workflows::validate();

// Infrastructure: scaffold-iac → validate-iac
let iac = Workflows::iac();
```

## Example: Create-App Stations

The built-in create-app workflow includes three stations:

### ScaffoldStation

Creates project structure from templates:
- Reads stack type from context
- Copies template files to output path
- Replaces placeholders like `{{app_name}}`
- Produces `scaffolded_files` output

### ValidateStation

Validates the generated project:
- Checks required files exist
- Validates configuration
- Produces `validation_passed` output

### CommitStation

Commits to version control:
- Initializes git repository
- Creates initial commit
- Produces `commit_hash` output

## Stack Types

Supported technology stacks:

| Stack | Template |
|-------|----------|
| `python-fastapi` | Python FastAPI REST API |
| `java-springboot` | Java Spring Boot |
| `java-quarkus` | Java Quarkus |
| `dotnet-webapi` | .NET Web API |
| `rust-api` | Rust API |
| `frontend-react` | React SPA |
| `frontend-angular` | Angular SPA |
| `frontend-vue` | Vue.js SPA |
| `electron-app` | Electron desktop app |

## Error Handling

Station execution can fail in two ways:

1. **Station returns failure**: `StationResult::failure("reason")`
2. **Station throws error**: Returns `Err(CoreError::...)`

Both result in workflow state becoming `Failed` and execution stopping.

```rust
// Check result success
if !result.success {
    println!("Failed: {}", result.message.unwrap_or_default());
}

// Or propagate error
let result = station.execute(&mut context).await?;
```

## Self-Healing Stations

Critical stations like `build-test` and `launch` implement **agent-based self-healing loops** that keep trying to fix errors automatically before giving up.

### Self-Healing Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     Self-Healing Station                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                    AgentGuardrails                           ││
│  │  • max_attempts_per_error: 5                                 ││
│  │  • max_total_iterations: 15                                  ││
│  │  • max_healing_time_secs: 300 (5 min)                        ││
│  │  • max_consecutive_failures: 3                               ││
│  └─────────────────────────────────────────────────────────────┘│
│                              │                                    │
│                              ▼                                    │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                    Healing Loop                              ││
│  │                                                              ││
│  │   while !success && !escalate:                               ││
│  │     1. Check guardrails → escalate if exceeded               ││
│  │     2. Execute command (build/test/launch)                   ││
│  │     3. If success → exit loop                                ││
│  │     4. Analyze error → classify ErrorType                    ││
│  │     5. Route to specialist agent                             ││
│  │     6. Apply fix → retry                                     ││
│  │                                                              ││
│  └─────────────────────────────────────────────────────────────┘│
│                              │                                    │
│                              ▼                                    │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                 Specialist Agents                            ││
│  │                                                              ││
│  │  • Implementer: Fixes build/compilation errors               ││
│  │  • Tester: Fixes test failures                               ││
│  │  • DevOps: Fixes port conflicts, runtime errors              ││
│  │  • Architect: Re-scaffolds damaged projects                  ││
│  │  • Factory: Handles unknown errors, coordinates agents       ││
│  │                                                              ││
│  └─────────────────────────────────────────────────────────────┘│
│                              │                                    │
│                              ▼                                    │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │              User Escalation (if needed)                     ││
│  │                                                              ││
│  │  When guardrails are exceeded, station returns NeedsInput    ││
│  │  with options like:                                          ││
│  │  • "retry" - Try again with clean slate                      ││
│  │  • "skip" - Continue without success                         ││
│  │  • "rescaffold" - Recreate from template                     ││
│  │  • "help" - User provides fix instructions                   ││
│  │                                                              ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

### Error Classification

The healing loop classifies errors to route to the appropriate agent:

| ErrorType | Specialist | Examples |
|-----------|-----------|----------|
| `BuildError` | Implementer | Syntax errors, compilation failures |
| `DependencyError` | DevOps | Missing packages, version conflicts |
| `TestFailure` | Tester | Assertion failures, test errors |
| `RuntimeError` | DevOps | Startup crashes, exceptions |
| `ConfigError` | Architect | Invalid config, missing properties |
| `PortInUse` | DevOps | Address already in use |
| `Unknown` | Factory | Unclassified errors |

### Healing Session Tracking

Each healing attempt is tracked in a `HealingSession`:

```rust
struct HealingSession {
    iterations: u32,                           // Total fix attempts
    attempts_by_type: HashMap<String, u32>,    // Per-error-type counts
    consecutive_failures: u32,                 // Failures without progress
    start_time: Option<Instant>,               // When healing started
    resolved_errors: Vec<String>,              // Successfully fixed
    actions_taken: Vec<String>,                // Fix descriptions
}
```

### Escalation Conditions

The station escalates to the user when any guardrail is exceeded:

1. **Max iterations reached** - Too many total fix attempts
2. **Consecutive failures** - No progress after multiple tries
3. **Time limit exceeded** - Healing taking too long
4. **Max attempts per error** - Same error type keeps recurring

### Station Result Types

Self-healing stations can return:

- `StationResult::Done` - Build/tests succeeded
- `StationResult::NeedsInput` - User must decide (retry, skip, help)
- `StationResult::Retry` - Internal retry (handled within station)

## Testing

```rust
#[tokio::test]
async fn test_my_workflow() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    
    // Setup
    let mut registry = StationRegistry::new();
    registry.register(Arc::new(MyStation::new()));
    
    let executor = WorkflowExecutor::new(Arc::new(registry));
    let workflow = Workflow::new("test", "Test").station("my-station");
    let context = WorkflowContext::new(
        temp_dir.path().to_path_buf(),
        "test-app",
        StackType::RustApi,
    );
    
    // Execute
    let log = executor.execute(&workflow, context).await.unwrap();
    
    // Assert
    assert_eq!(log.state, ExecutionState::Completed);
    assert!(log.error.is_none());
}
```
