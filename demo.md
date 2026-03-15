# Sealed composefs demo

This demo builds a CentOS Stream 10 bootc host image that can run sealed
application containers verified at mount time using composefs fs-verity
signatures.

## What this proves

The host boots with composefs as the root filesystem. At boot, a systemd
service loads a composefs app-signing certificate into the kernel's
`.fs-verity` keyring. When crun mounts a sealed application container
through composefs, the kernel verifies every file's content against the
fs-verity signature created at build time with the corresponding private
key.

```
  bootc composefs root (digest verified by kernel)
       |
       v
  .fs-verity keyring (app-signing cert loaded at boot)
       |
       v
  Sealed app container (crun mounts via composefs, kernel enforces sigs)
```

CentOS Stream 10 bootc ships with composefs support built in — the dracut
module, composefs-setup-root, SELinux policy, and systemd-networkd are all
part of the base image. This demo only adds cfsctl, crun, and the
app-signing certificate.

## Prerequisites

- podman (>= 4.7 for heredoc syntax)
- openssl
- cargo (Rust toolchain)
- just (task runner)
- bcvk (for local VM testing, from bootc-dev/bcvk)

## Local workflow

```
just keygen        # Generate composefs signing keypair
just build-host    # Build cfsctl + host container image
just build-app     # Build the httpd app container
just seal-app      # Seal and sign the app image
just boot          # Boot the host in a bcvk VM
just test          # SSH in and verify
```

The `keygen` target creates a composefs signing keypair under `target/keys/`.
The host build installs cfsctl and crun, embeds the signing certificate, and
adds a systemd service that loads the cert into the kernel keyring at boot.
The `seal-app` step pulls the app image into a cfsctl repo, seals it, and
signs it with the composefs private key. The `boot` target launches a bcvk
VM with the host image.

## GHA workflow

The `.github/workflows/build-sealed.yml` workflow automates the build for CI.
It expects two repository secrets:

- `COMPOSEFS_SIGNING_KEY` — PEM-encoded composefs signing private key
- `COMPOSEFS_SIGNING_CERT` — PEM-encoded composefs signing certificate

The workflow builds cfsctl from source, builds the host image with the
signing cert embedded, then builds and seals the app image in a separate job.

## Key details

### Hash algorithm

This demo uses `fsverity-sha256-12` for broader filesystem compatibility.
Production deployments may prefer `fsverity-sha512-12` for stronger hashes.

### Host image (Containerfile.host)

A single-stage build on centos-bootc:stream10. Installs crun and cfsctl,
embeds the app-signing certificate, and enables the keyring-loading
systemd service. The composefs digest injection and UKI building are
handled by `bootc install` at deploy time.

### App image (Containerfile.app)

A minimal CentOS Stream 10 httpd container. Nothing composefs-specific
happens at build time — sealing is a post-build step.

### Sealing workflow

After the app image is built:

1. `cfsctl init` creates a composefs repository
2. `cfsctl oci pull` imports the app image
3. `cfsctl oci seal` creates a sealed manifest with fs-verity digests
4. `cfsctl oci sign` creates a PKCS#7 signature artifact

The sealed image can then be run via crun's composefs integration, which
verifies signatures against the `.fs-verity` keyring.

---

Assisted-by: OpenCode (Claude Opus 4)
