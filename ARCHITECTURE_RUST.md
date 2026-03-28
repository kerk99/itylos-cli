# Itylos Rust Rewrite

## Phase 1 - Ingestion

### Etat actuel du CLI Go

Commandes detectees :
- `send [text] -d <1h|24h|7j> -f <file>`
- `read <url>`
- `verify <proof.json>`
- `mcp`

Machines d'etats fonctionnelles :
- `send`
  1. Valider input texte/fichier
  2. Construire `CapsuleV3`
  3. Generer cle URL locale aleatoire
  4. Deriver cle AES via `SHA-256(url_key)`
  5. Appliquer padding anti traffic analysis
  6. Chiffrer localement en `AES-256-GCM` avec AAD JSON
  7. Poster `payload`, `ttl`, `aad_hash` vers `/api/v2/create_secret`
  8. Afficher le lien `DOMAIN/v/<secret_id>#<key>`
- `read`
  1. Extraire `secret_id` et `#key` depuis l'URL
  2. Valider `secret_id` par regex hex 32
  3. Recuperer le payload via `/api/v2/fetch_secret?id=...`
  4. Refuser les capsules a mot de passe
  5. Dechiffrer localement avec la cle d'URL et l'AAD reconstruit
  6. Afficher le message et extraire les pieces jointes
  7. Demander la destruction serveur via `/api/v2/burn_secret`
- `verify`
  1. Charger le JSON de preuve
  2. Vider `verification.ed25519_signature`
  3. Re-serialiser
  4. Verifier la signature Ed25519 avec la cle publique serveur
- `mcp`
  1. Lire requetes JSON-RPC sur stdin
  2. Exposer `initialize`, `tools/list`, `tools/call`
  3. Deleguer `itylos_create_capsule` au flux `send`

### Flux reseau identifies

- `POST /api/v2/create_secret`
  Corps : `{ payload, ttl, aad_hash, has_password?, pwd_salt? }`
- `GET /api/v2/fetch_secret?id=<secret_id>`
  Reponse : `{ success, payload, has_password, pwd_salt, ttl, error }`
- `POST /api/v2/burn_secret`
  Corps : `{ id }`

### Processus cryptographique observe

- Cle locale aleatoire : 32 octets CSPRNG
- Derivation : `SHA-256(url_key)`
- Chiffrement : `AES-256-GCM`
- Nonce : 12 octets aleatoires
- AAD : `{"v":"2.0","alg":"AES-256-GCM","ttl":<seconds>}`
- Hash serveur : `SHA-256(aad_bytes)` encode en hex
- Payload : `base64url(ciphertext).base64url(nonce)`
- Format metier : `CapsuleV3` puis `PaddedPayload`

## Phase 2 - Mapping vers ai-rsk

Patterns repris de `Krigsexe/ai-rsk` :
- `src/main.rs` minimal avec `run() -> anyhow::Result<()>`
- CLI derive `clap` dans un module dedie
- separation stricte des concerns par modules
- types partages centralises
- gestion d'erreurs par propagation `Result`
- profil release deterministe : `lto`, `strip`, `codegen-units = 1`, `panic = "abort"`

Adaptation Itylos :
- `cli.rs` porte uniquement le parsing
- `services.rs` orchestre les use cases utilisateur
- `crypto/` encapsule toutes les primitives sensibles
- `network/` encapsule le contrat HTTP
- `mcp/` isole le protocole stdio
- `types.rs` centralise le schema JSON et les constantes metier

## Contraintes techniques

- Crypto native Rust : `aes-gcm`, `sha2`, `ed25519-dalek`, `rand`, `base64`
- Zeroization locale : `zeroize` sur cles derivees, plaintext dechiffres et buffers binaires extraits
- TLS HTTP sans OpenSSL systeme : `reqwest` avec `rustls-tls`
- Pas de FFI necessaire pour la couche cliente actuellement visible

## Bloqueurs

- `cargo` et `rustc` ne sont pas installes dans cet environnement.
- En consequence, `Cargo.lock` n'a pas pu etre genere localement et la compilation n'a pas pu etre verifiee ici.
- Le protocole "Double-Shield" cote serveur n'est pas dans ce depot : seule la couche cliente locale peut etre reecrite de maniere certaine.
