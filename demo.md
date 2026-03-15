# Sealed composefs demo

This demo builds a CentOS Stream 10 bootc host image that pulls and runs
sealed application containers verified at mount time using composefs
fs-verity signatures. The app image and its signature artifacts are
stored in a standard OCI registry (GHCR) using the OCI Referrers API.

## What this proves

At boot, a systemd service loads a composefs app-signing certificate into
the kernel's `.fs-verity` keyring. A second service pulls a sealed
application container from a registry (including its signature artifacts
via the OCI Referrers API), mounts it through composefs with
`--require-signature`, and runs httpd via crun.

```
  Registry (GHCR)
    ├── sealed-app:latest          (OCI image)
    └── referrer artifact          (composefs PKCS#7 signatures)
         │
         v (cfsctl oci pull + oci-client referrer fetch)
  Host VM
    ├── .fs-verity keyring         (app-signing cert loaded at boot)
    └── cfsctl oci mount --require-signature
         └── httpd running via crun
```

CentOS Stream 10 bootc ships with composefs support built in — the dracut
module, composefs-setup-root, SELinux policy, and systemd-networkd are all
part of the base image. This demo only adds cfsctl (with oci-client
feature for referrer fetch), crun, and the app-signing certificate.

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
just bcvk-ssh      # Boot VM and verify e2e
```

The `keygen` target creates a composefs signing keypair under `target/keys/`.
The host build installs cfsctl (with oci-client) and crun, embeds the
signing certificate, and adds systemd services for keyring loading and the
sealed app. The `seal-app` step pulls the app image into a cfsctl repo,
seals it, and signs it with the composefs private key.

## GHA workflow

The `.github/workflows/build-sealed.yml` workflow automates the full e2e:

1. Builds cfsctl from source (with oci-client feature)
2. Builds the httpd app container
3. Seals and signs it using stored secrets
4. Exports to OCI layout with signature referrer artifacts
5. Pushes to GHCR using `oras cp -r` (preserves referrers)
6. Builds the host image with the signing cert and correct app image ref
7. Pushes the host image to GHCR

Required repository secrets (use `util/keys.py github-store`):

- `COMPOSEFS_SIGNING_KEY` — PEM-encoded composefs signing private key
- `COMPOSEFS_SIGNING_CERT` — PEM-encoded composefs signing certificate

On PRs, only build validation runs (ephemeral signing cert, no push).

## Key details

### Hash algorithm

This demo uses the default `fsverity-sha512-12` algorithm.

### Host image (Containerfile.host)

Multi-stage build on centos-bootc:stream10. Stage 1 builds cfsctl from
source with `--features composefs-oci/oci-client` (needed for referrer
artifact fetch at pull time). Stage 2 installs crun, cfsctl, the signing
cert, and systemd services. The `SEALED_APP_IMAGE` build arg injects the
registry image reference into `/etc/default/sealed-httpd`.

### App image (Containerfile.app)

A minimal CentOS Stream 10 httpd container. Nothing composefs-specific
happens at build time — sealing is a post-build step.

### Sealing and signing workflow

After the app image is built:

1. `cfsctl init` creates a composefs repository
2. `cfsctl oci pull` imports the app image from containers-storage
3. `cfsctl oci seal` creates a sealed manifest with fs-verity digests
4. `cfsctl oci sign --cert --key` creates a PKCS#7 signature artifact
   stored as an OCI referrer
5. `cfsctl oci push --signatures` exports image + referrer to OCI layout
6. `oras cp -r` pushes to GHCR, preserving the referrer relationship

### Boot-time pull and verification

At boot, `sealed-httpd.service` runs `run-sealed-app.sh` which:

1. Initializes a local composefs repo
2. `cfsctl oci pull` fetches the image via skopeo, then the oci-client
   feature automatically fetches referrer artifacts from the registry
3. `cfsctl oci mount --require-signature --trust-cert` verifies PKCS#7
   signatures before mounting
4. httpd runs via crun with the verified composefs mount as rootfs

---

Assisted-by: OpenCode (Claude Opus 4)
