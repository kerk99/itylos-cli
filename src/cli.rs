use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "itylos",
    version,
    about = "Sovereign ephemeral messaging CLI.",
    long_about = "ITYLOS chiffre localement des capsules ephemeres, les transmet au sanctuaire distant et detruit la copie serveur apres lecture.\n\nArchitecture CLI inspiree de ai-rsk: main minimal, clap derive, sous-commandes strictement typees.",
    after_help = "Examples:\n  itylos send \"secret\"\n  itylos send -f secret.pdf -d 24h\n  itylos read https://almowatin.org/v/<id>#<key>\n  itylos verify proof.json\n  itylos mcp"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Chiffre localement et cree un lien ephemere.
    Send {
        /// Message a chiffrer. Peut etre vide si -f est fourni.
        text: Option<String>,
        /// Duree de vie (1h, 24h, 7j).
        #[arg(short = 'd', default_value = "1h")]
        duration: String,
        /// Fichier a joindre.
        #[arg(short = 'f')]
        file: Option<PathBuf>,
    },
    /// Dechiffre une capsule localement puis demande sa destruction serveur.
    Read { url: String },
    /// Verifie la signature Ed25519 d'une preuve de destruction.
    Verify { proof: PathBuf },
    /// Demarre le serveur MCP sur stdio.
    Mcp,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::error::ErrorKind;

    #[test]
    fn parses_send_with_defaults() {
        let cli = Cli::try_parse_from(["itylos", "send", "secret"]).expect("cli should parse");
        match cli.command {
            Some(Commands::Send {
                text,
                duration,
                file,
            }) => {
                assert_eq!(text.as_deref(), Some("secret"));
                assert_eq!(duration, "1h");
                assert!(file.is_none());
            }
            _ => panic!("expected send command"),
        }
    }

    #[test]
    fn parses_send_with_file_and_duration() {
        let cli = Cli::try_parse_from(["itylos", "send", "-f", "doc.pdf", "-d", "24h"])
            .expect("cli should parse");
        match cli.command {
            Some(Commands::Send {
                text,
                duration,
                file,
            }) => {
                assert_eq!(text, None);
                assert_eq!(duration, "24h");
                assert_eq!(file, Some(PathBuf::from("doc.pdf")));
            }
            _ => panic!("expected send command"),
        }
    }

    #[test]
    fn parses_read_verify_and_mcp() {
        let read = Cli::try_parse_from(["itylos", "read", "https://example/v/id#key"])
            .expect("read should parse");
        assert!(matches!(read.command, Some(Commands::Read { .. })));

        let verify =
            Cli::try_parse_from(["itylos", "verify", "proof.json"]).expect("verify should parse");
        assert!(matches!(verify.command, Some(Commands::Verify { .. })));

        let mcp = Cli::try_parse_from(["itylos", "mcp"]).expect("mcp should parse");
        assert!(matches!(mcp.command, Some(Commands::Mcp)));
    }

    #[test]
    fn help_and_version_are_handled_by_clap() {
        let help = Cli::try_parse_from(["itylos", "--help"]).expect_err("help should exit");
        assert_eq!(help.kind(), ErrorKind::DisplayHelp);

        let version =
            Cli::try_parse_from(["itylos", "--version"]).expect_err("version should exit");
        assert_eq!(version.kind(), ErrorKind::DisplayVersion);
    }
}
