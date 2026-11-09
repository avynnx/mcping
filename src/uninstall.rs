use anyhow::{Context, Result};

pub fn run() -> Result<()> {
    let exe = std::env::current_exe().context("couldn't find the mcping binary")?;

    if let Some(cmd) = package_manager(&exe.to_string_lossy()) {
        println!("mcping was installed with a package manager, remove it with:");
        println!("  {cmd}");
        return Ok(());
    }

    if cfg!(windows) {
        println!("delete {} to uninstall mcping", exe.display());
        return Ok(());
    }

    std::fs::remove_file(&exe).with_context(|| format!("couldn't remove {}", exe.display()))?;
    println!("removed {}", exe.display());
    Ok(())
}

fn package_manager(path: &str) -> Option<&'static str> {
    let path = path.replace('\\', "/").to_lowercase();
    if path.starts_with("/nix/store/") || path.contains("/.nix-profile/") {
        Some("nix profile remove mcping")
    } else if path.contains("/.cargo/bin/") {
        Some("cargo uninstall mcping")
    } else if path.contains("/microsoft/winget/") {
        Some("winget uninstall avynnx.mcping")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::package_manager;

    #[test]
    fn detects_how_mcping_was_installed() {
        assert_eq!(
            package_manager("/nix/store/abc123-mcping-0.1.1/bin/mcping"),
            Some("nix profile remove mcping")
        );
        assert_eq!(
            package_manager("/home/me/.cargo/bin/mcping"),
            Some("cargo uninstall mcping")
        );
        assert_eq!(
            package_manager(
                r"C:\Users\me\AppData\Local\Microsoft\WinGet\Packages\avynnx.mcping_Microsoft.Winget.Source_8wekyb3d8bbwe\mcping.exe"
            ),
            Some("winget uninstall avynnx.mcping")
        );
        assert_eq!(
            package_manager(r"C:\Users\me\AppData\Local\Microsoft\WinGet\Links\mcping.exe"),
            Some("winget uninstall avynnx.mcping")
        );
        assert_eq!(package_manager("/home/me/.local/bin/mcping"), None);
    }
}
