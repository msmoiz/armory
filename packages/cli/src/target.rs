use anyhow::bail;
use model::Triple;

/// Get the triple for the current platform.
pub fn triple() -> anyhow::Result<Triple> {
    let triple = match (std::env::consts::ARCH, std::env::consts::OS) {
        ("x86_64", "linux") => Triple::X86_64Linux,
        ("aarch64", "linux") => Triple::Aarch64Linux,
        ("x86_64", "macos") => Triple::X86_64Darwin,
        ("aarch64", "macos") => Triple::Aarch64Darwin,
        ("x86_64", "windows") => Triple::X86_64Windows,
        ("aarch64", "windows") => Triple::Aarch64Windows,
        target @ _ => bail!("unrecognized target {target:?}"),
    };
    Ok(triple)
}
