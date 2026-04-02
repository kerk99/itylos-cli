use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "itylos",
    version,
    about = "Messagerie secrete qui s'autodetruit. / Self-destructing secret messenger.",
    long_about = "\
ITYLOS — Messagerie secrete qui s'autodetruit
ITYLOS — Self-destructing secret messenger

Vous ecrivez un secret, il est chiffre sur votre machine, envoye au serveur,
et detruit automatiquement apres lecture. Personne d'autre ne peut le lire.
The secret is encrypted on your machine, sent to the server,
and automatically destroyed after reading. Nobody else can read it.

Commandes / Commands :
  itylos send \"secret\"       Envoyer un secret / Send a secret
  itylos send \"secret\" -p    Avec mot de passe / With password
  itylos send -f doc.pdf     Envoyer un fichier / Send a file
  itylos read <lien>         Lire et detruire / Read and destroy
  itylos verify preuve.json  Verifier une preuve / Verify a proof
  itylos mcp                 Serveur MCP pour IA / MCP server for AI
  itylos update              Mise a jour / Update to latest version",
    after_help = "\
Exemples / Examples :

  Envoyer un message secret (dure 1h par defaut) :
    itylos send \"mot de passe du wifi\"

  Envoyer un fichier qui s'autodetruit apres 24h :
    itylos send -f document.pdf -d 24h

  Proteger avec un mot de passe (le destinataire devra le saisir) :
    itylos send \"secret\" -p

  Fichier + mot de passe + duree 7 jours :
    itylos send -f confidentiel.pdf -d 7j -p

  Lire un secret (le lien vous est donne par l'expediteur) :
    itylos read https://itylos.com/v/abc123#cle

  Mettre a jour :
    itylos update"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Envoyer un secret chiffre. / Send an encrypted secret.
    #[command(after_help = "\
Exemples :
  itylos send \"mon mot de passe\"           Secret texte, dure 1h
  itylos send -f photo.jpg -d 24h          Fichier, dure 24h
  itylos send \"code wifi\" -p               Protege par mot de passe
  itylos send -f rapport.pdf -d 7j -p      Fichier + password + 7 jours

Durees disponibles : 1h (defaut), 24h, 7j")]
    Send {
        /// Le secret a envoyer (texte). Peut etre vide si -f est utilise.
        /// The secret to send (text). Can be empty if -f is used.
        text: Option<String>,

        /// Duree de vie : 1h, 24h ou 7j. / Lifetime: 1h, 24h or 7d.
        #[arg(short = 'd', default_value = "1h")]
        duration: String,

        /// Fichier a joindre (max 8 Mo). / File to attach (max 8 MB).
        #[arg(short = 'f')]
        file: Option<PathBuf>,

        /// Ajouter un mot de passe. Le destinataire devra le saisir.
        /// Add a password. The recipient will need to enter it.
        #[arg(short = 'p', long = "password")]
        password: bool,
    },

    /// Lire un secret et le detruire du serveur. / Read a secret and destroy it.
    #[command(after_help = "\
Le lien vous est donne par la personne qui a cree le secret.
Apres lecture, le secret est definitivement detruit du serveur.

Exemple :
  itylos read https://itylos.com/v/abc123def456#MaCleSecrete

Si le secret est protege par mot de passe, le CLI vous le demandera.")]
    Read {
        /// Le lien complet recu de l'expediteur (avec le # et la cle).
        /// The full link received from the sender (with # and key).
        url: String,
    },

    /// Verifier une preuve de destruction. / Verify a destruction proof.
    #[command(after_help = "\
Quand quelqu'un lit votre secret, le serveur le detruit et genere une preuve.
Le destinataire recoit un Proof ID qu'il peut vous envoyer.
Vous collez ce Proof ID ici pour verifier que la destruction a bien eu lieu.

Exemples :
  itylos verify 5eeb1fcfa615d644006ab35348a02e55    Proof ID (recommande)
  itylos verify preuve.json                          Fichier JSON local")]
    Verify {
        /// Proof ID (ex: 5eeb1f...) ou fichier JSON. / Proof ID or JSON file.
        proof: String,
    },

    /// Demarrer le serveur MCP pour assistants IA (Claude, Cursor...).
    /// Start MCP server for AI assistants (Claude, Cursor...).
    Mcp,

    /// Mettre a jour vers la derniere version. / Update to the latest version.
    #[command(after_help = "\
Verifie GitHub Releases et installe la derniere version.
Utilise cargo si disponible, sinon telecharge le binaire directement.

Checks GitHub Releases and installs the latest version.
Uses cargo if available, otherwise downloads the binary directly.")]
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
                ..
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
                ..
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
