use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "itylos",
    version,
    about = "Messagerie \u{00E9}ph\u{00E9}m\u{00E8}re souveraine en CLI.",
    long_about = "ITYLOS chiffre localement des capsules \u{00E9}ph\u{00E9}m\u{00E8}res, les transmet au sanctuaire distant\net d\u{00E9}truit la copie serveur apr\u{00E8}s lecture.\n\n\
                  Commandes :\n  \
                    itylos send \"secret\"       Chiffre et cr\u{00E9}e un lien \u{00E9}ph\u{00E9}m\u{00E8}re\n  \
                    itylos read <url>#<cl\u{00E9}>    D\u{00E9}chiffre puis d\u{00E9}truit\n  \
                    itylos verify proof.json   V\u{00E9}rifie une preuve Ed25519\n  \
                    itylos mcp                 Serveur MCP pour IA\n  \
                    itylos update              Met \u{00E0} jour vers la derni\u{00E8}re version",
    after_help = "Exemples :\n  \
                    itylos send \"secret\"\n  \
                    itylos send -f secret.pdf -d 24h\n  \
                    itylos read https://itylos.com/v/<id>#<cl\u{00E9}>\n  \
                    itylos verify proof.json\n  \
                    itylos update"
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
    /// D\u{00E9}marre le serveur MCP sur stdio.
    Mcp,
    /// Met \u{00E0} jour itylos vers la derni\u{00E8}re version.
    #[command(
        after_help = "V\u{00E9}rifie GitHub Releases pour une version plus r\u{00E9}cente et l'installe.\n\
                      Utilise cargo si disponible, sinon t\u{00E9}l\u{00E9}charge le binaire depuis GitHub."
    )]
    Update,
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

        let update = Cli::try_parse_from(["itylos", "update"]).expect("update should parse");
        assert!(matches!(update.command, Some(Commands::Update)));
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
