#!/bin/bash
# Start a sealed (composefs-verified) httpd container.
# Called by sealed-httpd.service; expects environment from /etc/default/sealed-httpd.
set -euo pipefail

: "${SEALED_APP_IMAGE:?SEALED_APP_IMAGE not set}"
: "${SEALED_APP_NAME:?SEALED_APP_NAME not set}"
: "${COMPOSEFS_REPO:=/var/lib/composefs/apps}"

BUNDLE="/run/sealed-apps/${SEALED_APP_NAME}"
ROOTFS="${BUNDLE}/rootfs"
CERT="/usr/lib/composefs/app-signing-cert.pem"

# 1. Ensure composefs repo exists
mkdir -p "${COMPOSEFS_REPO}"
cfsctl --insecure --repo "${COMPOSEFS_REPO}" init

# 2. Pull the image into the composefs repo
cfsctl --insecure --repo "${COMPOSEFS_REPO}" oci pull "${SEALED_APP_IMAGE}" "${SEALED_APP_NAME}"

# 3. Mount with signature verification
mkdir -p "${ROOTFS}"
cfsctl --insecure --repo "${COMPOSEFS_REPO}" oci mount \
    --require-signature --trust-cert "${CERT}" \
    "${SEALED_APP_NAME}" "${ROOTFS}"

# 4. Write a minimal OCI runtime config for httpd
cat > "${BUNDLE}/config.json" <<'OCICFG'
{
  "ociVersion": "1.0.0",
  "process": {
    "terminal": false,
    "user": { "uid": 0, "gid": 0 },
    "args": ["/usr/sbin/httpd", "-DFOREGROUND"],
    "env": [
      "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
      "LANG=C.UTF-8"
    ],
    "cwd": "/"
  },
  "root": {
    "path": "rootfs",
    "readonly": true
  },
  "linux": {
    "namespaces": [
      { "type": "pid" },
      { "type": "mount" },
      { "type": "ipc" },
      { "type": "uts" }
    ]
  },
  "mounts": [
    { "destination": "/proc",    "type": "proc",   "source": "proc" },
    { "destination": "/sys",     "type": "sysfs",  "source": "sysfs",  "options": ["nosuid","noexec","nodev","ro"] },
    { "destination": "/dev",     "type": "tmpfs",  "source": "tmpfs",  "options": ["nosuid","strictatime","mode=755","size=65536k"] },
    { "destination": "/tmp",     "type": "tmpfs",  "source": "tmpfs",  "options": ["nosuid","nodev"] },
    { "destination": "/run",     "type": "tmpfs",  "source": "tmpfs",  "options": ["nosuid","nodev","mode=755"] },
    { "destination": "/var/run", "type": "tmpfs",  "source": "tmpfs",  "options": ["nosuid","nodev","mode=755"] },
    { "destination": "/var/log", "type": "tmpfs",  "source": "tmpfs",  "options": ["nosuid","nodev"] }
  ]
}
OCICFG

# 5. Exec into crun — replaces this process so systemd tracks the container PID
exec crun run --bundle "${BUNDLE}" "${SEALED_APP_NAME}"
