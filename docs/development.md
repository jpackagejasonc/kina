# Development

## Development Environment Setup

kina uses [mise](https://mise.jdx.dev/) for development environment management and task automation. This provides consistent tooling and streamlined workflows.

```bash
# Install all tools via mise
mise install

# Verify Apple Container CLI availability
mise run container:check
```

## Node Image Building

kina requires custom Kubernetes node images optimized for Apple Container. These images contain the necessary components for running Kubernetes nodes in Apple Container VMs.

```bash
# Build custom kina node image
mise run image:build

# Test the built node image
mise run image:test

# Build and test in one command
mise run image:validate

# List available images
mise run image:list

# Clean up unused images
mise run image:clean
```

**Node Image Components:**
- **Base System**: Debian (13-slim) with systemd for container orchestration
- **Container Runtime**: containerd configured for Apple Container integration
- **Kubernetes Components**: kubelet, kubeadm, kubectl (v1.36.1)
- **CNI Plugins**: PTP support
- **Init Scripts**: Apple Container-specific initialization and networking setup

The built images are tagged as `kina/node:v1.36.1` and can be used with:
```bash
kina create my-cluster --image kina/node:v1.36.1
```

## Pre-commit and Secret Scanning

`mise run pre-commit` runs formatting, linting, tests, audit, and **gitleaks secret scanning** before each commit. Gitleaks is also available standalone:

```bash
mise run gitleaks                # Run gitleaks secret scanner
```

## Common Development Tasks

```bash
# Build and install
mise run build                   # Release build
mise run dev                     # Development build
mise run test                    # Run tests
mise run kina:install            # Install kina CLI from project root
mise run pre-commit              # Format, lint, test, audit, gitleaks
mise run ci                      # Run full CI pipeline locally

# Code quality
mise run fmt                     # Format code with rustfmt
mise run lint                    # Run clippy with strict settings
mise run audit                   # Security audit with cargo-audit
mise run check                   # Check code without building
mise run gitleaks                # Secret scanning with gitleaks

# Documentation and utilities
mise run docs                    # Generate and open documentation
mise run clean                   # Clean build artifacts
mise run watch                   # Watch files and rebuild on changes
mise run bench                   # Run benchmarks

# CLI testing
mise run kina -- create test     # Run kina with arguments (release build)
mise run kina:dev -- --help      # Run kina in dev mode (faster build)
mise run test:cli                # Basic CLI functionality tests

# Available tasks
mise tasks                       # List all available mise tasks

# Integration testing workflows
mise run test:cluster            # Create test cluster with Traefik and demo app
mise run test:cluster:validate   # Validate most recent test cluster
mise run test:cluster:cleanup    # Clean up all test clusters
```

## Project Structure

```
kina/
├── kina-cli/                   # Main CLI application
│   ├── src/
│   │   ├── cli/               # Command implementations
│   │   ├── config/            # Configuration management
│   │   ├── core/              # Core cluster management
│   │   └── main.rs            # Application entry point
│   ├── tests/                 # Integration tests
│   ├── manifests/             # Kubernetes manifests
│   ├── images/                # Custom node image Dockerfile
│   └── Cargo.toml
├── scripts/                    # Extracted mise task scripts (Nushell)
├── docs/                       # Documentation
├── CLAUDE.md                   # AI assistant context
├── mise.toml                   # Development automation
├── Cargo.toml                  # Workspace configuration
└── README.md
```
