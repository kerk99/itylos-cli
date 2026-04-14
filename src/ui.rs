use colored::Colorize;
use console::measure_text_width;

use crate::types::VERSION;

/// Embedded composite ANSI art: butterfly logo (chafa 30col) + ITYLOS text (Y in cyan).
/// Generated at build time with chafa, compiled into the binary. No runtime dependencies.
const LOGO_COMPOSITE: &str = include_str!("../assets/logo-composite.ans");

/// Build the framed banner displayed when `itylos` is run without arguments.
fn build_banner() -> String {
    let w: usize = 86;
    let dash = "\u{2500}";
    let c = "\x1b[36m"; // cyan
    let r = "\x1b[0m"; // reset
    let d = "\x1b[2m"; // dim
    let wb = "\x1b[1;37m"; // white bold

    let title = format!(" itylos {} ", VERSION);
    let tl = (w - title.len()) / 2;
    let tr = w - title.len() - tl;

    let mut o = String::new();

    // ── Top ──
    o.push_str(&format!(
        "{c}\u{256D}{}{}{}\u{256E}{r}\n",
        dash.repeat(tl),
        title,
        dash.repeat(tr)
    ));

    // ── Logo ──
    for line in LOGO_COMPOSITE.lines() {
        if line.starts_with("[?25") {
            continue;
        }
        let vw = measure_text_width(line);
        let p = w.saturating_sub(vw);
        o.push_str(&format!(
            "{c}\u{2502}{r}{}{}{c}\u{2502}{r}\n",
            line,
            " ".repeat(p)
        ));
    }

    // ── Empty ──
    o.push_str(&format!("{c}\u{2502}{}\u{2502}{r}\n", " ".repeat(w)));

    // ── Subtitle ──
    let sub = "L'\u{00E9}ph\u{00E9}m\u{00E8}re souverain en CLI  \u{2022}  kachouri.com";
    let sl = sub.chars().count();
    let spl = w.saturating_sub(sl) / 2;
    let spr = w.saturating_sub(sl).saturating_sub(spl);
    o.push_str(&format!(
        "{c}\u{2502}{r}{}{d}{}{r}{}{c}\u{2502}{r}\n",
        " ".repeat(spl),
        sub,
        " ".repeat(spr)
    ));

    // ── Separator ──
    o.push_str(&format!("{c}\u{251C}{}\u{2524}{r}\n", dash.repeat(w)));

    // ── Commands ──
    let commands: &[(&str, &str, &str)] = &[
        (
            "send",
            "Chiffre localement et cr\u{00E9}e un lien \u{00E9}ph\u{00E9}m\u{00E8}re",
            "itylos send \"secret\" | itylos send -f secret.pdf -d 24h",
        ),
        (
            "read",
            "D\u{00E9}chiffre une capsule puis d\u{00E9}truit la copie serveur",
            "itylos read https://itylos.com/v/<id>#<cl\u{00E9}>",
        ),
        (
            "verify",
            "V\u{00E9}rifie la signature Ed25519 d'une preuve de destruction",
            "itylos verify proof.json",
        ),
        (
            "mcp",
            "D\u{00E9}marre le serveur MCP pour Intelligence Artificielle",
            "itylos mcp",
        ),
        (
            "update",
            "Met \u{00E0} jour itylos vers la derni\u{00E8}re version",
            "itylos update",
        ),
    ];

    for (cmd, desc, example) in commands {
        // Command line
        let line = format!("  {}  {}", cmd, desc);
        let colored_cmd = format!("{c}{}{r}", cmd);
        o.push_str(&format!(
            "{c}\u{2502}{r}  {}  {}{}{c}\u{2502}{r}\n",
            colored_cmd,
            desc,
            " ".repeat(w.saturating_sub(line.chars().count()))
        ));

        // Example line
        let ex = format!("       {}", example);
        o.push_str(&format!(
            "{c}\u{2502}{r}       {d}{}{r}{}{c}\u{2502}{r}\n",
            example,
            " ".repeat(w.saturating_sub(ex.chars().count()))
        ));
    }

    // ── Empty ──
    o.push_str(&format!("{c}\u{2502}{}\u{2502}{r}\n", " ".repeat(w)));

    // ── Footer ──
    let foot = "La cl\u{00E9} de d\u{00E9}chiffrement n'a jamais quitt\u{00E9} cet ordinateur.";
    let fl = foot.chars().count();
    let fpl = w.saturating_sub(fl) / 2;
    let fpr = w.saturating_sub(fl).saturating_sub(fpl);
    o.push_str(&format!(
        "{c}\u{2502}{r}{}{wb}{}{r}{}{c}\u{2502}{r}\n",
        " ".repeat(fpl),
        foot,
        " ".repeat(fpr)
    ));

    // ── Bottom ──
    o.push_str(&format!("{c}\u{2570}{}\u{256F}{r}\n", dash.repeat(w)));

    o
}

pub fn print_banner() {
    println!("{}", build_banner());
}

pub fn separator() {
    println!("  {}", "\u{2500}".repeat(50).dimmed());
}

pub fn print_link(link: &str) {
    println!();
    println!(
        "  {} Capsule s\u{00E9}curis\u{00E9}e avec succ\u{00E8}s",
        "\u{2713}".green().bold()
    );
    separator();
    println!("  {} {}", "LIEN SECRET :".cyan().bold(), link);
    separator();
    println!(
        "  {}",
        "La cl\u{00E9} (#...) n'a jamais quitt\u{00E9} cet ordinateur.".dimmed()
    );
}

pub fn print_decrypted_header() {
    println!();
    println!(
        "  {} Capsule d\u{00E9}chiffr\u{00E9}e",
        "\u{2713}".green().bold()
    );
    separator();
}

pub fn print_decrypted_footer() {
    separator();
}

pub fn print_burn_verified() {
    println!(
        "  {} Capsule d\u{00E9}truite du serveur \u{2014} preuve de destruction v\u{00E9}rifi\u{00E9}e",
        "\u{2713}".green().bold()
    );
}

pub fn print_burn_unverified() {
    println!(
        "  {} Capsule d\u{00E9}truite du serveur, mais la preuve Ed25519 n'a pas pu \u{00EA}tre v\u{00E9}rifi\u{00E9}e",
        "!".yellow().bold()
    );
}

pub fn print_burn_no_proof() {
    println!(
        "  {} Capsule d\u{00E9}truite du serveur \u{2014} preuve de destruction g\u{00E9}n\u{00E9}r\u{00E9}e",
        "\u{2713}".green().bold()
    );
}

pub fn print_burn_failed() {
    println!(
        "  {} D\u{00E9}chiffrement r\u{00E9}ussi, mais la purge serveur a retourn\u{00E9} une erreur",
        "!".yellow().bold()
    );
}

pub fn print_proof_authentic() {
    println!(
        "  {} PREUVE AUTHENTIQUE \u{2014} la destruction a \u{00E9}t\u{00E9} confirm\u{00E9}e cryptographiquement par ITYLOS",
        "\u{2713}".green().bold()
    );
}

pub fn print_proof_forged() {
    println!(
        "  {} PREUVE FALSIFI\u{00C9}E \u{2014} la signature ne correspond pas \u{00E0} l'empreinte de la donn\u{00E9}e",
        "\u{2717}".red().bold()
    );
}

pub fn print_file_loaded(name: &str, size: usize) {
    println!(
        "  {} Fichier charg\u{00E9} : {} ({} octets)",
        "\u{2713}".green(),
        name,
        size
    );
}

pub fn print_file_extracted(name: &str, size: usize) {
    println!(
        "  {} Fichier extrait : {} ({} octets)",
        "\u{2192}".cyan(),
        name,
        size
    );
}
