// Vérification de version et auto-update pour itylos.
// check_for_update : non-bloquant, silencieux en cas d'erreur réseau.
// run_self_update : cargo install ou GitHub Releases en fallback.

use colored::Colorize;

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const GITHUB_REPO: &str = "kerk99/itylos-cli";
const GITHUB_API: &str = "https://api.github.com/repos/kerk99/itylos-cli/releases/latest";

/// Vérifie si une nouvelle version est disponible sur GitHub Releases.
/// Non-bloquant : les erreurs réseau sont ignorées silencieusement.
pub fn check_for_update() {
    let handle = std::thread::spawn(fetch_latest_version);

    if let Ok(Some(latest)) = handle.join() {
        if is_newer(&latest, CURRENT_VERSION) {
            eprintln!(
                "  {} itylos {} disponible (actuel : {}). Lancer : {}",
                "!".yellow(),
                latest.bold(),
                CURRENT_VERSION,
                "itylos update".cyan()
            );
        }
    }
}

/// Met à jour itylos vers la dernière version.
/// Essaie cargo en premier, puis télécharge depuis GitHub Releases.
pub fn run_self_update() {
    println!(
        "  {} Version actuelle : {}",
        "i".cyan(),
        CURRENT_VERSION.bold()
    );

    print!("  Vérification des mises à jour... ");
    let latest = match fetch_latest_version() {
        Some(v) => v,
        None => {
            println!("{}", "échec".red());
            eprintln!(
                "  {} Impossible de joindre GitHub. Vérifiez votre connexion.",
                "!".red()
            );
            std::process::exit(1);
        }
    };

    if !is_newer(&latest, CURRENT_VERSION) {
        println!("{}", "à jour".green().bold());
        println!(
            "  {} itylos {} est déjà la dernière version.",
            "✓".green(),
            CURRENT_VERSION
        );
        return;
    }

    println!("{} disponible", latest.green().bold());

    // Essayer cargo install d'abord
    if which::which("cargo").is_ok() {
        println!("  {} Mise à jour via cargo...", "→".cyan());
        let status = std::process::Command::new("cargo")
            .args([
                "install",
                "--git",
                &format!("https://github.com/{}", GITHUB_REPO),
            ])
            .status();

        match status {
            Ok(s) if s.success() => {
                println!(
                    "\n  {} itylos mis à jour vers {}",
                    "✓".green(),
                    latest.bold()
                );
                return;
            }
            _ => {
                eprintln!(
                    "  {} cargo install échoué. Téléchargement depuis GitHub Releases...",
                    "!".yellow()
                );
            }
        }
    }

    // Fallback : téléchargement depuis GitHub Releases
    println!("  {} Téléchargement depuis GitHub Releases...", "→".cyan());
    let gh_tag = format!("v{}", latest);
    let os = if cfg!(target_os = "linux") {
        "x86_64-unknown-linux-musl"
    } else if cfg!(target_os = "macos") {
        "x86_64-apple-darwin"
    } else if cfg!(target_os = "windows") {
        "x86_64-pc-windows-msvc"
    } else {
        eprintln!(
            "  {} Plateforme non supportée pour la mise à jour automatique.",
            "!".red()
        );
        eprintln!(
            "  Installation manuelle : cargo install --git https://github.com/{}",
            GITHUB_REPO
        );
        std::process::exit(1);
    };

    let ext = if cfg!(target_os = "windows") {
        ".zip"
    } else {
        ".tar.gz"
    };
    let asset = format!("itylos-{}{}", os, ext);
    let url = format!(
        "https://github.com/{}/releases/download/{}/{}",
        GITHUB_REPO, gh_tag, asset
    );

    let tmp_dir = std::env::temp_dir().join(format!("itylos-update-{}", std::process::id()));
    if std::fs::create_dir_all(&tmp_dir).is_err() {
        eprintln!(
            "  {} Impossible de créer le répertoire temporaire.",
            "!".red()
        );
        std::process::exit(1);
    }

    let dl_path = tmp_dir.join(&asset);
    let dl_status = std::process::Command::new("curl")
        .args([
            "-sfL",
            "--progress-bar",
            "-o",
            &dl_path.to_string_lossy(),
            &url,
        ])
        .status();

    if dl_status.map(|s| s.success()).unwrap_or(false) && dl_path.exists() {
        let extract_ok = if asset.ends_with(".tar.gz") {
            std::process::Command::new("tar")
                .args([
                    "-xzf",
                    &dl_path.to_string_lossy(),
                    "-C",
                    &tmp_dir.to_string_lossy(),
                ])
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        } else {
            std::process::Command::new("tar")
                .args([
                    "-xf",
                    &dl_path.to_string_lossy(),
                    "-C",
                    &tmp_dir.to_string_lossy(),
                ])
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        };

        if extract_ok {
            let bin_name = if cfg!(target_os = "windows") {
                "itylos.exe"
            } else {
                "itylos"
            };
            let extracted = tmp_dir.join(bin_name);
            if extracted.exists() {
                if let Ok(current_exe) = std::env::current_exe() {
                    if std::fs::copy(&extracted, &current_exe).is_ok() {
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            let _ = std::fs::set_permissions(
                                &current_exe,
                                std::fs::Permissions::from_mode(0o755),
                            );
                        }
                        println!("  {} itylos mis à jour vers {}", "✓".green(), latest.bold());
                        std::fs::remove_dir_all(&tmp_dir).ok();
                        return;
                    }
                }

                // Fallback : copier dans ~/.local/bin/
                let home = std::env::var("HOME")
                    .or_else(|_| std::env::var("USERPROFILE"))
                    .unwrap_or_default();
                if !home.is_empty() {
                    let local_bin = std::path::PathBuf::from(&home).join(".local").join("bin");
                    std::fs::create_dir_all(&local_bin).ok();
                    let dest = local_bin.join(bin_name);
                    if std::fs::copy(&extracted, &dest).is_ok() {
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            let _ = std::fs::set_permissions(
                                &dest,
                                std::fs::Permissions::from_mode(0o755),
                            );
                        }
                        println!(
                            "  {} itylos {} installé dans {}",
                            "✓".green(),
                            latest.bold(),
                            dest.display()
                        );
                        std::fs::remove_dir_all(&tmp_dir).ok();
                        return;
                    }
                }
            }
        }
    }

    std::fs::remove_dir_all(&tmp_dir).ok();
    eprintln!("  {} Mise à jour automatique échouée.", "!".red());
    eprintln!(
        "  Installation manuelle : {}",
        format!("cargo install --git https://github.com/{}", GITHUB_REPO).cyan()
    );
    std::process::exit(1);
}

/// Récupère la dernière version depuis l'API GitHub Releases.
fn fetch_latest_version() -> Option<String> {
    let output = std::process::Command::new("curl")
        .args([
            "-sfL",
            "--max-time",
            "5",
            "-H",
            "User-Agent: itylos-cli",
            GITHUB_API,
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let body = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&body).ok()?;
    // GitHub tag_name = "v2.0.1" → strip le "v"
    let tag = json["tag_name"].as_str()?;
    Some(tag.strip_prefix('v').unwrap_or(tag).to_string())
}

/// Compare deux versions semver. Retourne true si `latest` est strictement plus récente.
fn is_newer(latest: &str, current: &str) -> bool {
    let parse =
        |s: &str| -> Vec<u64> { s.split('.').filter_map(|p| p.parse::<u64>().ok()).collect() };

    let l = parse(latest);
    let c = parse(current);

    if l.len() < 3 || c.len() < 3 {
        return false;
    }

    (l[0], l[1], l[2]) > (c[0], c[1], c[2])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_newer_detects_major_minor_patch() {
        assert!(is_newer("3.0.0", "2.0.0"));
        assert!(is_newer("2.1.0", "2.0.0"));
        assert!(is_newer("2.0.1", "2.0.0"));
    }

    #[test]
    fn is_newer_rejects_same_and_older() {
        assert!(!is_newer("2.0.0", "2.0.0"));
        assert!(!is_newer("1.9.0", "2.0.0"));
    }

    #[test]
    fn is_newer_handles_invalid_input() {
        assert!(!is_newer("abc", "2.0.0"));
        assert!(!is_newer("2.0.0", "abc"));
    }

    #[test]
    fn current_version_is_semver() {
        let parts: Vec<&str> = CURRENT_VERSION.split('.').collect();
        assert_eq!(parts.len(), 3, "version should be semver x.y.z");
        for part in &parts {
            assert!(part.parse::<u64>().is_ok(), "each part should be numeric");
        }
    }
}
