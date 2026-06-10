# kina - Kubernetes in Apple Container

[![CI](https://github.com/jpackagejasonc/kina/actions/workflows/ci.yml/badge.svg)](https://github.com/jpackagejasonc/kina/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/jpackagejasonc/kina)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![Apple Container](https://img.shields.io/badge/Apple%20Container-1.0.0%2B-blue.svg)](https://github.com/apple/container)

**kina** is a Rust CLI tool for running local Kubernetes clusters using Apple Container technology. It provides similar functionality to [kind](https://kind.sigs.k8s.io/) (Kubernetes in Docker) but is optimized for macOS systems, leveraging native Apple Container technology for improved performance and integration.

📖 **New to kina?** Start with the [installation](docs/installation.md) and [quick start](docs/quick-start.md) guides.

## Documentation

- [Requirements](docs/requirements.md) - System requirements, Apple Container setup, and supporting Kubernetes tools.
- [Installation](docs/installation.md) - Source installation, mise setup, and verification steps.
- [Quick Start](docs/quick-start.md) - First cluster workflow, Traefik installation, integration test cluster setup, and setup verification.
- [Command Reference](docs/command-reference.md) - Cluster, resource, addon, and configuration commands.
- [Configuration](docs/configuration.md) - Configuration file location, defaults, and environment variables.
- [Apple Container Integration](docs/apple-container-integration.md) - Container lifecycle integration and cluster architecture.
- [CNI Support](docs/cni-support.md) - CNI plugin behavior and selection.
- [Development](docs/development.md) - Local development setup, node image builds, project structure, and mise tasks.
- [Troubleshooting](docs/troubleshooting.md) - Common issues, debug commands, and support links.
- [Kubelet CSR Auto-Approval](docs/kubelet-csr-auto-approval.md) - Background and commands for kubelet serving certificate approval.

## Features

- 🏗️ **Native Apple Container Integration** - Leverage macOS container technology for optimal performance
- ☸️ **Kubernetes API Compatibility** - Full Kubernetes cluster functionality with kubectl integration
- 🌐 **CNI Plugin Support** - PTP networking optimized for Apple Container, with a generic plugin selection surface for future options
- 🔧 **Traefik (Gateway API)** - Built-in support for Traefik gateway controller installation and configuration
- 📊 **Metrics Server** - Built-in support for Kubernetes Metrics Server installation (enables `kubectl top` and HPA)
- ⚙️ **Flexible Configuration** - TOML-based configuration with sensible defaults
- 📋 **Comprehensive CLI** - Rich command set for cluster management and operations
- 🚀 **Development Ready** - Integrated development workflow with mise task automation

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Development Workflow

1. **Fork and Clone**: Fork the repository and clone your fork
2. **Setup Environment**: Run `mise install` to install all tools
3. **Create Branch**: Create a feature branch for your changes
4. **Develop**: Make changes with comprehensive tests
5. **Quality Checks**: Run `mise run pre-commit` before committing (includes gitleaks)
6. **Submit PR**: Create a pull request with clear description

### Code Quality

- **Formatting**: `mise run fmt` (rustfmt)
- **Linting**: `mise run lint` (clippy with strict settings)
- **Testing**: `mise run test` (comprehensive test suite)
- **Security**: `mise run audit` (cargo-audit dependency scanning)

## License

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

---

**Note**: kina is in active development. While functional, some features are still being implemented. See the [project roadmap](https://github.com/vinnie357/kina/projects) for current status and planned features.
