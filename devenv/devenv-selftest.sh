#!/bin/bash
# Test that the devcontainer environment is functional.
# This script is designed to be run inside the container after devenv-init.sh
# has already been executed (e.g., via postCreateCommand).
set -euo pipefail

echo "=== Basic tool availability ==="
echo "Podman version: $(podman --version)"
echo "Rust: $(rustc --version)"
echo "Cargo: $(cargo --version)"

# Nested podman requires --security-opt unmask=/proc/* (Podman-only)
# or --privileged. Skip gracefully when not available (e.g. under Docker).
echo ""
echo "=== Testing nested podman ==="

# Use CentOS Stream 10 as the test image for both container and VM
image="quay.io/centos-bootc/centos-bootc:stream10"

if podman pull "$image" 2>/dev/null && \
   podman run --rm "$image" echo "Hello from nested podman!" 2>/dev/null; then
    echo "=== Nested container test passed ==="
else
    echo "=== Nested podman not available (expected under Docker without --privileged) ==="
fi

# Test bcvk (VM) if available and /dev/kvm exists
if command -v bcvk >/dev/null 2>&1 && [ -e /dev/kvm ]; then
    echo ""
    echo "=== Testing bcvk VM ==="
    echo "bcvk version:"
    bcvk --version

    echo "Running bcvk ephemeral VM with SSH..."
    bcvk ephemeral run-ssh "$image" -- echo "Hello from bcvk VM!"

    echo "=== bcvk VM test passed ==="
else
    echo ""
    echo "=== Skipping bcvk VM test (bcvk not available or /dev/kvm missing) ==="
fi

echo ""
echo "=== All tests passed ==="
