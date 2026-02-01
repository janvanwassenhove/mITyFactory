# ADR-0006: Cost-Aware Agentic Factory

**Status:** Accepted  
**Date:** 2025-01-21  
**Authors:** mITyFactory Team

## Context

The mITyFactory Autonomous Factory uses LLMs, compute resources, and generates cloud infrastructure that incurs ongoing costs. Users need visibility into:

1. **LLM costs** - API calls to OpenAI, Anthropic, Azure OpenAI, etc.
2. **Compute costs** - Container builds, test runs, security scans
3. **Infrastructure costs** - Monthly cloud resource estimates from IaC
4. **Feature costs** - Incremental cost impact of each feature

Without transparent cost tracking, users cannot make informed decisions about:
- Whether to proceed with expensive operations
- Which features to prioritize
- How to optimize their factory workflows

## Decision

We implement a comprehensive **Cost Tracking System** that:

### 1. Cost Model

```
┌─────────────────────────────────────────────────────────┐
│                     SessionCostState                     │
├─────────────────────────────────────────────────────────┤
│  session_id: String                                      │
│  currency: Currency (USD | EUR)                          │
│  llm_records: Vec<LlmUsageRecord>                       │
│  execution_records: Vec<ExecutionCostEstimate>          │
│  infra_estimate: Option<CostEstimate>                   │
│  created_at: DateTime<Utc>                              │
│  updated_at: DateTime<Utc>                              │
└─────────────────────────────────────────────────────────┘
         │
         ├──────────────────────────────────────────┐
         ▼                                          ▼
┌─────────────────────┐                ┌─────────────────────────┐
│   LlmUsageRecord    │                │ ExecutionCostEstimate   │
├─────────────────────┤                ├─────────────────────────┤
│ timestamp           │                │ execution_type          │
│ model: LlmModel     │                │ description             │
│ input_tokens        │                │ cost: CostItem          │
│ output_tokens       │                │ duration_secs           │
│ cost: CostItem      │                │ timestamp               │
│ operation           │                └─────────────────────────┘
└─────────────────────┘
```

### 2. CostItem Structure

Every cost is represented as a **range** with three values:

```rust
struct CostItem {
    min: f64,       // Best case (cache hits, efficient prompts)
    expected: f64,  // Most likely cost
    max: f64,       // Worst case (retries, long outputs)
    unit: String,   // "$" or "€"
    notes: Option<String>,
}
```

### 3. LLM Pricing Tables

Built-in pricing for common models (prices per 1K tokens):

| Model | Input | Output |
|-------|-------|--------|
| GPT-4 | $0.030 | $0.060 |
| GPT-4-Turbo | $0.010 | $0.030 |
| GPT-4o | $0.005 | $0.015 |
| GPT-4o-Mini | $0.00015 | $0.0006 |
| GPT-3.5-Turbo | $0.0005 | $0.0015 |
| Claude-3-Opus | $0.015 | $0.075 |
| Claude-3-Sonnet | $0.003 | $0.015 |
| Claude-3.5-Sonnet | $0.003 | $0.015 |
| Claude-3-Haiku | $0.00025 | $0.00125 |

### 4. Infrastructure Cost Templates

Pre-built estimates for common architectures (monthly):

| Architecture | Min | Expected | Max |
|--------------|-----|----------|-----|
| Backend API (small) | $10 | $25 | $50 |
| Backend + Database | $30 | $60 | $120 |
| Frontend SPA | $1 | $5 | $20 |
| Full-Stack (small) | $40 | $90 | $180 |

### 5. Cost Persistence

Costs are persisted at:
```
.mity/chat/<sessionId>/cost.json
```

Feature costs are persisted at:
```
workspaces/<appName>/.mity/cost/features/<featureId>.json
```

### 6. Runtime Integration

The `FactoryRuntimeState` includes a `RuntimeCostSummary`:

```rust
struct RuntimeCostSummary {
    total_expected: f64,
    total_min: f64,
    total_max: f64,
    llm_cost: f64,
    compute_cost: f64,
    monthly_infra: f64,
    currency: String,
    llm_calls: u32,
    total_tokens: u64,
    exceeds_threshold: bool,
    threshold: f64,
    summary: String,
}
```

### 7. Configuration

Environment variables for customization:

| Variable | Default | Description |
|----------|---------|-------------|
| `MITY_COST_THRESHOLD` | `1.00` | Cost threshold for blocking confirmation |
| `MITY_COST_CURRENCY` | `USD` | Display currency (USD, EUR) |
| `MITY_COST_LLM_ENABLED` | `true` | Enable LLM cost tracking |

### 8. UI Components

1. **Cost Badge** - Always visible in status bar showing session total
2. **Cost Details Panel** - Expandable breakdown by category
3. **Threshold Warning** - Visual alert when costs exceed threshold
4. **Event Costs** - Per-event cost in timeline (planned)

## Consequences

### Positive

1. **Transparency** - Users see costs before, during, and after operations
2. **Control** - Configurable thresholds can pause expensive operations
3. **Explainability** - Breakdown by category helps identify cost drivers
4. **Predictability** - Range estimates (min/expected/max) set expectations
5. **Feature Comparison** - Feature-level deltas enable cost-based prioritization

### Negative

1. **Maintenance** - Pricing tables need periodic updates
2. **Accuracy** - Estimates may not match actual cloud bills precisely
3. **Complexity** - Additional state to persist and synchronize

### Risks

1. **Stale Prices** - LLM providers change pricing frequently
   - Mitigation: Design for easy updates, consider fetching from API

2. **Inaccurate Estimates** - IaC costs vary by region, usage patterns
   - Mitigation: Use ranges, allow custom overrides

## Implementation

### Rust Types (mity_chat/src/cost.rs)

- `Currency` - USD, EUR enum
- `CostScope` - Session, App, Feature, Station, LlmCall
- `CostItem` - Range with min/expected/max
- `CostBreakdown` - By category (llm, compute, storage, network, misc)
- `CostEstimate` - Full estimate with confidence and assumptions
- `LlmModel` - Enum with auto-detection from model name
- `LlmPricing` - Per-model pricing lookup
- `LlmUsageRecord` - Single API call record
- `ExecutionType` - ContainerBuild, TestRun, SecurityScan, IaCValidation
- `ExecutionCostEstimate` - Compute cost record
- `FeatureCostDelta` - Per-feature impact tracking
- `SessionCostState` - Complete session state
- `CostConfig` - Configuration from environment

### Tauri Commands (mity_ui/src/commands.rs)

- `cost_get` - Get session cost state
- `cost_get_config` - Get cost configuration
- `cost_record_llm` - Record an LLM API call
- `cost_update_infra` - Update infrastructure estimate
- `cost_record_execution` - Record compute cost
- `cost_check_threshold` - Check if threshold exceeded
- `cost_get_features` - Get feature costs for app
- `cost_save_feature` - Save feature cost delta
- `runtime_get_with_cost` - Get runtime with refreshed cost

### UI (mity_ui/dist/)

- `app.js` - Cost state, methods, polling updates
- `index.html` - Cost badge, details panel, icons
- `styles.css` - Cost-specific styling

## Alternatives Considered

### 1. External Cost Tracking Service

Could integrate with cloud cost management tools (AWS Cost Explorer, etc.).

**Rejected because:**
- Adds external dependency
- Real-time visibility harder to achieve
- Doesn't cover LLM costs

### 2. Post-hoc Analysis Only

Track actual costs after the fact from invoices/dashboards.

**Rejected because:**
- No predictive capability
- Can't pause expensive operations
- Poor user experience

### 3. Simple Token Counter

Just count tokens, let users calculate costs manually.

**Rejected because:**
- Poor UX
- Doesn't cover compute/infra
- No range estimates

## References

- [OpenAI Pricing](https://openai.com/pricing)
- [Anthropic Pricing](https://www.anthropic.com/pricing)
- [Azure Pricing Calculator](https://azure.microsoft.com/pricing/calculator/)
