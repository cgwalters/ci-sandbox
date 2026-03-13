//! Upgrade compatibility tests.
//!
//! Verifies that a repository populated by the *current* `cfsctl` binary can
//! be read by a *previous* version shipped in the dev container image.  This
//! catches on-disk format regressions early: if a new commit changes the
//! repository layout in a way that older versions cannot read, this test
//! fails.

use anyhow::Result;
use xshell::{cmd, Shell};

use crate::{cfsctl, create_oci_layout, integration_test};

/// Default dev container image containing the previously-released `cfsctl`.
/// Override with `COMPOSEFS_DEV_IMAGE` for forks or local testing.
const DEFAULT_DEV_IMAGE: &str = "ghcr.io/composefs/dev-composefs-rs:centos-stream10";

/// Return the dev image reference, respecting `COMPOSEFS_DEV_IMAGE` override.
fn dev_image() -> String {
    std::env::var("COMPOSEFS_DEV_IMAGE").unwrap_or_else(|_| DEFAULT_DEV_IMAGE.to_string())
}

/// Verify that a repo written by the current cfsctl is readable by the
/// previous version from the dev container image.
///
/// Flow:
///  1. Build an OCI layout fixture with [`create_oci_layout`].
///  2. Pull it into a fresh repo using the *current* `cfsctl`.
///  3. Mount that repo into the dev container and run `cfsctl oci images`
///     and `cfsctl oci inspect` with the *old* binary to confirm it can
///     still read the data.
///
/// Skipped when `COMPOSEFS_SKIP_NETWORK` or `COMPOSEFS_SKIP_UPGRADE` is set,
/// or when the dev image has not been published yet.
fn test_upgrade_repo_readable_by_previous_version() -> Result<()> {
    if std::env::var_os("COMPOSEFS_SKIP_NETWORK").is_some() {
        eprintln!("Skipping (COMPOSEFS_SKIP_NETWORK is set)");
        return Ok(());
    }
    if std::env::var_os("COMPOSEFS_SKIP_UPGRADE").is_some() {
        eprintln!("Skipping (COMPOSEFS_SKIP_UPGRADE is set)");
        return Ok(());
    }

    let sh = Shell::new()?;
    let dev_image = dev_image();

    // --- Pull the dev image (skip gracefully if it doesn't exist yet) ---
    eprintln!("Pulling dev image {dev_image} ...");
    let pull_result = cmd!(sh, "podman pull {dev_image}").run();
    if let Err(e) = pull_result {
        eprintln!("Skipping: could not pull dev image ({e:#}). Image may not be published yet.");
        return Ok(());
    }

    // --- Populate a repo with the *current* cfsctl ---
    let cfsctl = cfsctl()?;
    let repo_dir = tempfile::tempdir()?;
    let repo = repo_dir.path();
    let fixture_dir = tempfile::tempdir()?;
    let oci_layout = create_oci_layout(fixture_dir.path())?;

    eprintln!("Populating repo with current cfsctl ...");
    let pull_output = cmd!(
        sh,
        "{cfsctl} --insecure --repo {repo} oci pull oci:{oci_layout} upgrade-test"
    )
    .read()?;
    eprintln!("{pull_output}");

    // --- Verify the old cfsctl can list images ---
    eprintln!("Running old cfsctl (oci images) inside container ...");
    let list_output = cmd!(
        sh,
        "podman run --rm -v {repo}:/repo:Z {dev_image} cfsctl --insecure --repo /repo oci images"
    )
    .read()?;
    eprintln!("{list_output}");
    assert!(
        list_output.contains("upgrade-test"),
        "expected 'upgrade-test' in image list from old cfsctl, got: {list_output}"
    );

    // --- Verify the old cfsctl can inspect the image ---
    eprintln!("Running old cfsctl (oci inspect) inside container ...");
    let inspect_output = cmd!(
        sh,
        "podman run --rm -v {repo}:/repo:Z {dev_image} cfsctl --insecure --repo /repo oci inspect upgrade-test"
    )
    .read()?;
    let inspect: serde_json::Value = serde_json::from_str(&inspect_output)?;
    assert!(
        inspect.get("manifest").is_some(),
        "expected 'manifest' key in inspect output from old cfsctl"
    );
    assert!(
        inspect.get("config").is_some(),
        "expected 'config' key in inspect output from old cfsctl"
    );
    eprintln!("Upgrade compatibility check passed.");

    Ok(())
}
integration_test!(test_upgrade_repo_readable_by_previous_version);
