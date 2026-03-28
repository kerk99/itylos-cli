use thiserror::Error;

#[derive(Debug, Error)]
pub enum ItylosError {
    #[error("Le message est vide.")]
    EmptyMessage,
    #[error("Le fichier depasse la limite de 8 Mo pour la V2.")]
    FileTooLarge,
    #[error("URL invalide. La cle (#...) est manquante.")]
    MissingUrlKey,
    #[error("L'identifiant de la capsule est malforme ou dangereux.")]
    InvalidSecretId,
    #[error("TTL absent dans la reponse serveur - dechiffrement impossible.")]
    MissingTtl,
    #[error(
        "Cette capsule est protegee par mot de passe. Ouvrez ce lien dans votre navigateur pour la dechiffrer."
    )]
    PasswordProtected,
    #[error("Ce document n'est pas signe (unsigned).")]
    UnsignedProof,
    #[error("{0}")]
    Message(String),
}
