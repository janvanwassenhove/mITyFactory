# mITyFactory Roadmap

## Current Version: 0.3.0 (AI Integration)

### ✅ Completed (v0.1.0 - Foundation)
- Core workspace structure with 10 crates
- CLI with init, create-app, add-feature, validate, smoke-templates, quality-gate commands
- Spec Kit management (init, read, write, validate)
- Workflow engine with DAG execution
- Container runner with Docker/Podman support
- Template system with variable substitution
- Policy engine with DoD and quality gates
- IaC scaffolding for Terraform (AWS, Azure, GCP)
- Agent handlers for all SDLC roles (Analyst, Architect, Implementer, Tester, Reviewer, Security, DevOps, Designer, A11y)
- CI/CD pipeline configuration
- ADR documentation structure (7 ADRs)

### ✅ Completed (v0.2.0 - Chat & UI)
- Agent chat system (`mity_chat` crate) with LLM support (OpenAI, Anthropic)
- Autopilot mode for autonomous factory execution
- Guided intake flow for new applications
- Cost tracking and token usage monitoring
- Session persistence to filesystem
- Tauri 2.0 desktop UI (`mity_ui` crate)
- Alpine.js reactive frontend
- Custom design system with WCAG 2.1 AA accessibility
- Real-time timeline for factory progress

### ✅ Production Templates
- Python FastAPI (backend)
- Java Spring Boot (backend)
- Java Quarkus (backend)
- .NET Web API (backend)
- Angular 17+ (frontend)
- Vue.js 3 (frontend)
- Spring Boot + Vue.js (fullstack)
- Spring Boot + React (fullstack)
- Quarkus + Vue.js (fullstack)
- Quarkus + React (fullstack)

### 🚧 In Progress (v0.3.0 - AI Integration)
- [ ] LLM integration for spec analysis
- [ ] AI-assisted code generation in agents
- [ ] Intelligent error diagnosis
- [ ] Natural language feature specifications

### 📋 Stub Templates (Need Completion)
- [ ] Rust API (rust-api)
- [ ] React SPA (frontend-react)
- [ ] Electron Desktop (electron-app)

---

## Version 0.4.0 (Enterprise)

### Planned
- [ ] Multi-tenant support
- [ ] Custom policy engines
- [ ] Audit logging and compliance
- [ ] SSO/OIDC authentication
- [ ] Role-based access control

---

## Version 0.5.0 (Polish & GA Prep)

### Planned
- [ ] Template versioning and updates
- [ ] Template discovery from remote registries
- [ ] Visual workflow editor enhancements
- [ ] Real-time build monitoring
- [ ] Template marketplace

---

## Version 1.0.0 (GA)

### Prerequisites
- [ ] 100% API stability
- [ ] Comprehensive documentation
- [ ] >80% test coverage
- [ ] Security audit complete
- [ ] Performance benchmarks published

---

## Future Considerations

### Potential Features
- Kubernetes native deployment
- Multi-cloud orchestration
- Plugin marketplace
- IDE extensions (VS Code, JetBrains)
- Git hosting integration (GitHub, GitLab, Azure DevOps)
- Issue tracker sync
- Slack/Teams notifications

### Community
- Open contribution guidelines
- Plugin development SDK
- Template authoring guide
- Video tutorials

---

*This roadmap is subject to change based on community feedback and priorities.*

*Last Updated: 2026-02-01*
