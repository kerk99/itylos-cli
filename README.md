# itylos-cli
🦋 ITYLOS : L’art de l’éphémère numérique. Outil de messagerie souverain avec chiffrement local AES-256, protocole Double-Shield et auto-destruction physique après lecture. Reprenez le contrôle sur vos secrets. / Sovereign ephemeral messaging with local encryption and burn-on-read technology. Restore your digital right to be forgotten.

# 🦋 ITYLOS Terminal  
**Early Access – v1.0.1-beta**

> **L'art de l'éphémère numérique.**  
> **The art of digital ephemerality.**

---

## 🌍 Présentation / Overview

**ITYLOS** est un outil souverain en ligne de commande permettant d’envoyer des **messages chiffrés éphémères** qui s’autodétruisent physiquement après lecture.  
Le chiffrement est effectué localement, la destruction est vérifiable, et aucune donnée sensible n’est conservée au-delà de sa durée de vie.

**ITYLOS** is a sovereign command-line tool designed to send **encrypted ephemeral messages** that physically self-destruct after being read.

---

## 🛡️ Manifeste de Bienveillance / Benevolence Manifesto

### 🇫🇷 Pourquoi ITYLOS ?
Internet n’oublie rien. Les humains, si.  
ITYLOS restaure un droit fondamental : **l’oubli numérique réel**.

1. **PROTÉGER** — Réduire les traces numériques inutiles.  
2. **RESPECTER** — Vos secrets sont chiffrés chez vous.  
3. **ÉDUQUER** — La confidentialité est une compétence.  
4. **RESPONSABILISER** — Un message est un acte de confiance.

### 🇬🇧 Why ITYLOS?
The internet forgets nothing — humans should be allowed to.  
ITYLOS restores a fundamental right: **real digital oblivion**.

---

## 🔐 Sécurité & Architecture / Security & Architecture

### 🔒 Double-Shield Protocol (Zero-Knowledge)

| Layer | Scope | Description |
|------|------|-------------|
| **Layer 1** | Local | AES-256-GCM encryption on your machine. The key never leaves your terminal. |
| **Layer 2** | Server | Additional encryption before storage (API → MariaDB). |
| **Burn-on-Read** | Lifecycle | Physical destruction immediately after successful read. |

✔ Zero-knowledge by design  
✔ No plaintext storage  
✔ No recovery possible

---

## 🚀 Installation Rapide / Direct Installation

### Prérequis
- **Go 1.21+**

### Installation via GitHub

```bash
go install github.com/kerk99/itylos-cli@latest
```

### Configuration par OS

#### 🪟 Windows (PowerShell)

```powershell
Set-Alias itylos "$HOME\go\bin\itylos.exe"
```

#### 🍎 macOS & 🐧 Linux

```bash
sudo mv ~/go/bin/itylos /usr/local/bin/itylos
```

---

## 🛠️ Commandes CLI réelles (v1.0.1-beta)

| Commande | Action concrète (FR) | Action (EN) |
|--------|---------------------|-------------|
| `itylos send "msg"` | Chiffre localement et génère un lien sécurisé | Encrypt and generate secure link |
| `-d 24h / 7j` | Définit la durée de vie du message | Set message lifetime |
| `itylos mission` | Affiche le Manifeste de bienveillance | Display the manifesto |
| `itylos status` | Vérifie l’état du Sanctuaire ITYLOS | Real-time service status |
| `itylos update` | Vérifie les mises à jour | Check for updates |

---

## 🌐 À propos de itylos.com / About itylos.com

**ITYLOS** est également une plateforme web souveraine dédiée au partage de secrets éphémères.  
Hébergement en **Suisse (Genève)** – conformité **RGPD / LPD suisse**.

Principes clés :
- Aucune clé de déchiffrement stockée
- Aucune journalisation des secrets
- Destruction vérifiable

---

## ⚖️ Conformité – RGPD (Art. 17)

Chaque message génère une **preuve de destruction** permettant de vérifier la suppression définitive de la donnée.

---

## 🧪 Guide de test d’installation (GitHub)

### Étapes

1. Ajouter `itylos.go` et `go.mod` à la racine du dépôt
2. Attendre ~60 secondes
3. Tester :

```bash
go install github.com/kerk99/itylos-cli@latest
```

### Résultat attendu
- Téléchargement des dépendances
- Binaire généré dans `~/go/bin`

---

## 🤝 Contribution

Projet en **Early Access (beta)**.  
Retours, audits et contributions bienvenus.

---

## 🦋 Note finale

**Souveraineté activée. Votre message est protégé.**  
**Sovereignty active. Your message is protected.**
https://itylos.com/
