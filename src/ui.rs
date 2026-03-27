use colored::Colorize;

use crate::types::VERSION;

pub fn print_banner() {
    println!();
    println!("  ██╗████████╗██╗   ██╗██║      ██████╗ ███████╗");
    println!("  ██║╚══██╔══╝╚██╗ ██╔╝██║     ██╔═══██╗██╔════╝");
    println!("  ██║   ██║    ╚████╔╝ ██║     ██║   ██║███████╗");
    println!("  ██║   ██║     ╚██╔╝  ██║     ██║   ██║╚════██║");
    println!("  ██║   ██║      ██║   ███████╗╚██████╔╝███████║");
    println!("  ╚═╝   ╚═╝      ╚═╝   ╚══════╝ ╚═════╝ ╚══════╝");
    println!(
        "\n          {}",
        format!("L'ART DE L'EPHEMERE EN CLI • {VERSION}")
            .bold()
            .magenta()
    );
    println!("{}", "─".repeat(62));
    println!("\nCOMMANDES :");
    println!("  itylos send \"secret\"      : Chiffre et cree un lien ephemere");
    println!("  itylos send -f secret.pdf : Chiffre et envoie un fichier joint");
    println!("  itylos read <url>         : Dechiffre une capsule localement");
    println!("  itylos verify <proof.json>: Audite la signature de destruction");
    println!("  itylos mcp                : Demarre le serveur pour Intelligence Artificielle");
}
