# Features

This directory contains feature specifications for mITyFactory.

## Structure

Each feature should be a separate YAML file following this schema:

```yaml
id: FEAT-001
title: Feature Title
description: |
  Detailed description of the feature.
status: draft | spec-complete | in-progress | implemented | validated
priority: critical | high | medium | low
assignee: optional
created: 2026-01-15
updated: 2026-01-15

acceptance_criteria:
  - Criterion 1
  - Criterion 2

dependencies:
  - FEAT-000 (if any)

notes: |
  Additional notes or context.
```

## Lifecycle

1. **Draft**: Initial idea captured
2. **Spec-Complete**: Full specification written and reviewed
3. **In-Progress**: Development has started
4. **Implemented**: Code complete, pending validation
5. **Validated**: All acceptance criteria met

## Adding a Feature

Use the CLI to add features:

```bash
mity add-feature --app myapp --title "New Feature" --spec-file feature.md
```

Or manually create a YAML file in this directory.
