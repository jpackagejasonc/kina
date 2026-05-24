# kina (Kubernetes in Apple Container) - AI Assistant Context

## Project Overview
**Technology Stack**: Rust 2021 edition with clap CLI framework, Tokio async runtime, kube-rs for Kubernetes integration, Apple Container runtime
**Architecture**: Monolithic CLI with provider abstraction layer, following domain-driven layered architecture
**Domain**: Kubernetes orchestration and container management for macOS using Apple Container technology
**Development Phase**: Active development with established project structure, comprehensive tooling, and advanced development practices

## Project Structure
- **kina-cli/src/**: Rust CLI source (cli/, core/, config/, errors/, utils/)
- **kina-cli/tests/**: CLI and config tests
- **kina-cli/manifests/**: Kubernetes manifests (traefik, demo-app)
- **kina-cli/images/**: Custom node image Dockerfile and build scripts
- **scripts/**: Extracted mise task scripts (Nushell `.nu` and Bash `.sh`)

## AI Assistant Guidance

### Project-Specific Focus Areas
- **Rust Development**: Apply Cargo-based project management, CLI framework integration (clap), error handling (anyhow/thiserror)
- **Container Integration**: Use Apple Container native patterns, Docker API compatibility considerations
- **Kubernetes Operations**: Consider kube-rs client patterns, RBAC configuration, resource management
- **CLI Patterns**: Focus on command parsing, configuration management, output formatting

### Workflow
- **Branch per task**: `git checkout main && git pull` then `git checkout -b type/description`
- **Discover tools with mise**: Run `mise tasks` to see all available development tasks. Task namespaces: `test:` (unit tests), `test:cluster:` (integration tests), `test:action:` (GitHub Actions), `kina:` (CLI), `image:` (node images), `gitleaks:` (security scanning)

### Anti-Fabrication Requirements
All AI assistants working on this project MUST adhere to strict factual accuracy:
- Base all outputs on actual project analysis using tool execution (Read, Glob, Bash)
- Execute validation tools before making claims about file existence or system capabilities
- Mark uncertain information as "requires analysis" or "needs validation"
- Use precise, factual language without superlatives or unsubstantiated performance claims
- Never fabricate time estimates, effort calculations, or completion timelines without measurement

### Development Context
- **Technology Requirements**: macOS 26+, Apple Container 0.5.0+, kubectl, mise, Nushell
- **Integration Goals**: Kind (Kubernetes in Docker) workflow compatibility using Apple Container technology
