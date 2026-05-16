#!/bin/bash

# Gateway API Examples Deployment Script for Kina (Traefik)
#
# Usage: ./deploy-examples.sh <command> [example-name] [cluster-name]

set -euo pipefail

CLUSTER_NAME="${3:-test-cluster}"
KUBECONFIG_PATH="$HOME/.kube/$CLUSTER_NAME"
CLUSTER_IP=""

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log()   { echo -e "${GREEN}[INFO]${NC} $1"; }
warn()  { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

check_prerequisites() {
    log "Checking prerequisites..."
    command -v kina    &>/dev/null || error "kina CLI not found in PATH."
    command -v kubectl &>/dev/null || error "kubectl not found."
    kina list | grep -q "$CLUSTER_NAME" \
        || error "Cluster '$CLUSTER_NAME' not found. Create it with: kina create $CLUSTER_NAME"
    [[ -f "$KUBECONFIG_PATH" ]] || error "Kubeconfig not found at $KUBECONFIG_PATH"

    CLUSTER_IP=$(container list | grep "$CLUSTER_NAME-control-plane" | awk '{print $NF}' || true)
    [[ -n "$CLUSTER_IP" ]] || error "Could not determine cluster IP for $CLUSTER_NAME"
    log "Using cluster: $CLUSTER_NAME ($CLUSTER_IP)"
}

check_traefik() {
    log "Checking Traefik gateway controller..."
    kubectl --kubeconfig="$KUBECONFIG_PATH" get pods -n traefik &>/dev/null \
        || error "traefik namespace not found. Install with: kina install traefik --cluster $CLUSTER_NAME"

    local ready_pods
    ready_pods=$(kubectl --kubeconfig="$KUBECONFIG_PATH" get pods -n traefik --no-headers \
        | awk '$2 ~ /1\/1/ && $3 == "Running"' | wc -l)
    [[ $ready_pods -gt 0 ]] || error "No ready traefik pods. Check: kubectl --kubeconfig $KUBECONFIG_PATH get pods -n traefik"
    log "Traefik ready ($ready_pods pods)"
}

deploy_example() {
    local example="$1"
    local example_file="$example.yaml"
    [[ -f "$example_file" ]] || error "Example file '$example_file' not found"

    log "Deploying $example..."
    kubectl --kubeconfig="$KUBECONFIG_PATH" apply -f "$example_file"
    log "Waiting for pods..."
    kubectl --kubeconfig="$KUBECONFIG_PATH" wait --for=condition=Ready pods --all --timeout=60s
    log "✅ $example deployed"
}

cleanup_example() {
    local example="$1"
    local example_file="$example.yaml"
    [[ -f "$example_file" ]] || error "Example file '$example_file' not found"
    log "Cleaning up $example..."
    kubectl --kubeconfig="$KUBECONFIG_PATH" delete -f "$example_file" --ignore-not-found=true
    log "✅ $example cleaned up"
}

list_examples() {
    log "Available examples:"
    echo "  • basic-web-app         - Single-service HTTPRoute with host match"
    echo "  • multi-service-routing - Path-based routing with URLRewrite filters"
    echo "  • virtual-hosts         - Multiple HTTPRoutes for different hosts"
}

show_usage() {
    cat <<EOF
Gateway API Examples Deployment Script (Traefik)

Usage: $0 <command> [example-name] [cluster-name]

Commands:
  deploy <example>     Deploy an example
  cleanup <example>    Remove an example
  list                 List available examples
  help                 Show this help

Default cluster: test-cluster
EOF
}

main() {
    local command="${1:-help}"
    case "$command" in
        deploy)
            [[ $# -ge 2 ]] || error "Usage: $0 deploy <example-name> [cluster-name]"
            check_prerequisites
            check_traefik
            deploy_example "$2"
            ;;
        cleanup)
            [[ $# -ge 2 ]] || error "Usage: $0 cleanup <example-name> [cluster-name]"
            check_prerequisites
            cleanup_example "$2"
            ;;
        list) list_examples ;;
        help|*) show_usage ;;
    esac
}

cd "$(dirname "${BASH_SOURCE[0]}")"
main "$@"
