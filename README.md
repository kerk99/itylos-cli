# itylos-cli
🦋 ITYLOS : L’art de l’éphémère numérique. Outil de messagerie souverain avec chiffrement local AES-256, protocole Double-Shield et auto-destruction physique après lecture. Reprenez le contrôle sur vos secrets. / Sovereign ephemeral messaging with local encryption and burn-on-read technology. Restore your digital right to be forgotten.

# 🦋 ITYLOS — Ephemeral Secrets Engine
**Early Access – v1.0.1-beta**

> **L’art de l’éphémère numérique.**  
> **The art of digital ephemerality.**

---

## 🌍 Overview

**ITYLOS** is a sovereign command-line tool designed to send **encrypted messages that self-destruct after being read**.  
Encryption happens locally, destruction is verifiable, and no readable secret is ever stored long-term.

**ITYLOS** est un outil en ligne de commande souverain permettant d’envoyer des **messages chiffrés éphémères**, détruits de manière irréversible après lecture.

---

## 🛡️ Benevolence Manifesto / Manifeste de Bienveillance

### 🇫🇷 Pourquoi ITYLOS ?
Internet n’oublie rien. Les humains, si.  
ITYLOS restaure un droit fondamental : **l’oubli numérique réel**.

1. **PROTÉGER** — Réduire les traces numériques inutiles.  
2. **RESPECTER** — Vos secrets sont chiffrés chez vous.  
3. **ÉDUQUER** — La confidentialité est une compétence.  
4. **RESPONSABILISER** — Un message est un acte de confiance.

### 🇬🇧 Why ITYLOS?
The internet forgets nothing — humans should be allowed to.

1. **PROTECT** — Reduce unnecessary digital traces.  
2. **RESPECT** — Secrets are encrypted locally.  
3. **EDUCATE** — Privacy is a skill.  
4. **EMPOWER** — A message is trust, not storage.

---

## 🔐 Security & Architecture

### 🔒 Double-Shield & Zero-Knowledge

| Layer | Scope | Description |
|------|------|-------------|
| **Layer 1** | Local | AES-256-GCM encryption on your device. The key never leaves your terminal. |
| **Layer 2** | Server | Additional encryption before storage. |
| **Burn-on-Read** | Lifecycle | Data is **physically destroyed** after first read. |

✔ Zero-knowledge by design  
✔ No plaintext storage  
✔ No recovery possible

---


## 🚀 Installation Rapide / Direct Installation

Si **Go** est installé sur votre machine, vous pouvez installer ITYLOS directement via GitHub :

```bash
go install [github.com/kerk99/itylos-cli@latest](https://github.com/kerk99/itylos-cli@latest)
```
Configuration par OS :
## 🚀 Installation

### Windows (PowerShell)

```powershell
Set-Alias itylos "C:\path\to\itylos.exe"
```

🍎 macOS & 🐧 Linux
```
# Ajouter à votre PATH ou créer un alias
sudo mv ~/go/bin/itylos /usr/local/bin/itylos
```


---

## 🛠️ Commandes réelles – v1.0.1-beta

> Ces commandes correspondent **exactement** à la version actuelle du terminal.

| Commande | Action concrète |
|--------|----------------|
| `itylos send "message"` | Chiffre ton message **localement** et génère un lien de partage sécurisé. |
| `-d 24h` / `-d 7j` | Définit la durée de vie du message avant effacement automatique. |
| `itylos mission` | Affiche le **Manifeste de bienveillance** (vision et principes). |
| `itylos status` | Vérifie en temps réel si le **Sanctuaire ITYLOS** est prêt à recevoir des messages. |
| `itylos update` | Recherche si une nouvelle version du terminal est disponible. |

---

## 🌐 About itylos.com

**ITYLOS** est aussi une plateforme web souveraine dédiée au partage de secrets éphémères.  
Le projet est hébergé en **Suisse (Genève)**, sur une infrastructure conforme **RGPD / LPD suisse**.

Principes clés :
- Aucune clé de déchiffrement stockée
- Aucune journalisation des secrets
- Destruction vérifiable via preuve d’effacement

---

## ⚖️ GDPR – Right to Erasure (Art. 17)

Chaque message génère une **preuve de destruction** permettant de vérifier la suppression définitive de la donnée.

---

## 🤝 Contributing

ITYLOS est en **Early Access (beta)**.  
Les retours, audits et contributions sont encouragés.

---

## 🦋 Closing Note

**Souveraineté activée. Votre message est protégé.**  
**Sovereignty active. Your message is protected.**
