# mITyFactory Guiding Principles

## 🧭 Decision-Making Framework

These principles guide decision-making when the constitution doesn't provide explicit direction.

---

## Principle 1: Spec-Driven Development

> "If it's not specified, it doesn't exist."

**Rationale**: Specifications create shared understanding, enable review before implementation, and serve as documentation.

**In Practice**:
- Write specs before code
- Update specs when requirements change
- Generate code from specs where possible
- Validate implementations against specs

---

## Principle 2: Fail Fast, Fail Clearly

> "Errors should be immediate, loud, and actionable."

**Rationale**: Silent failures are the most dangerous. Early, clear failures save debugging time.

**In Practice**:
- Validate inputs at boundaries
- Return structured errors with context
- Prefer errors over warnings for violations
- Include remediation hints in error messages

---

## Principle 3: Progressive Disclosure

> "Simple things should be simple. Complex things should be possible."

**Rationale**: Most users have simple needs. Don't burden them with complexity they don't need.

**In Practice**:
- Sensible defaults for common cases
- Optional configuration for advanced needs
- Documentation tiered by expertise level
- Quick-start paths for new users

---

## Principle 4: Explicit Over Implicit

> "Prefer explicit configuration over magic."

**Rationale**: Implicit behavior is hard to discover, debug, and maintain.

**In Practice**:
- Configuration files over conventions
- Named parameters over positional
- Visible state over hidden caches
- Documented behavior over tribal knowledge

---

## Principle 5: Composition Over Inheritance

> "Build complex systems from simple, composable parts."

**Rationale**: Composition enables flexibility, reuse, and easier testing.

**In Practice**:
- Small, focused components
- Clear interfaces between components
- Dependency injection
- Plugins and extensions over modifications

---

## Principle 6: Immutability by Default

> "Treat data as immutable unless mutation is explicitly needed."

**Rationale**: Immutable data is easier to reason about, test, and parallelize.

**In Practice**:
- Copy-on-write semantics
- Versioned state changes
- Append-only logs
- Clear ownership boundaries

---

## Principle 7: Observable Systems

> "If you can't measure it, you can't improve it."

**Rationale**: Observability enables debugging, optimization, and capacity planning.

**In Practice**:
- Structured logging
- Metrics for key operations
- Distributed tracing
- Health check endpoints

---

## Principle 8: Documentation as Code

> "Documentation should be versioned, tested, and deployed with code."

**Rationale**: Out-of-date documentation is worse than no documentation.

**In Practice**:
- Docs live in the repo
- API docs generated from code
- Examples are tested
- Doc reviews with code reviews

---

## Using These Principles

When facing a decision:

1. **Check the constitution**: Is there a rule that applies?
2. **Apply principles**: Which principles are relevant?
3. **Balance trade-offs**: When principles conflict, document the trade-off
4. **Document the decision**: Record rationale for future reference

---

*These principles guide but don't dictate. Use judgment and document exceptions.*
