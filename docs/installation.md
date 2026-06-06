# Installation

## Option 1: From Source (Recommended)

```bash
# Clone the repository
git clone https://github.com/vinnie357/kina.git
cd kina

# Install using Cargo
cargo install --path kina-cli

# OR using mise (if installed)
mise run kina:install
```

## Option 2: Development Setup with mise

```bash
# Clone the repository
git clone https://github.com/vinnie357/kina.git
cd kina

# Build and install
mise run install
```

## Verification

```bash
# Verify installation (shows kina and Apple Container versions)
kina

# Check Apple Container availability (REQUIRED, 0.5.0+)
mise run container:check  # If using mise
# OR manually check:
container --version
container system start  # Start the service if not running

kubectl version --client
```

**Important**: Apple Container 0.5.0+ must be available before creating clusters. kina auto-detects and validates the version at startup. Run `kina` with no arguments, or `kina status` when clusters exist, to see Apple Container version information.
