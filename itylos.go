package main

import (
	"bufio"
	"bytes"
	"crypto/aes"
	"crypto/cipher"
	"crypto/ed25519"
	"crypto/rand"
	"crypto/sha256"
	"encoding/base64"
	"encoding/hex"
	"encoding/json"
	"flag"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"regexp"
	"strings"

	"github.com/fatih/color"
)

// --- CONFIGURATION GLOBALE ---
const (
	DOMAIN     = "https://almowatin.org" // Domaine de dev
	API_CREATE = DOMAIN + "/api/v2/create_secret"
	API_FETCH  = DOMAIN + "/api/v2/fetch_secret"
	API_BURN   = DOMAIN + "/api/v2/burn_secret"
	VERSION    = "v2.0.0-dev-secure"

	// Clé publique Ed25519 du serveur pour vérifier les preuves (À remplacer)
	SERVER_PUB_KEY_B64 = "tsIkULXxSVudU1ZkJ3u5IpXN+11WpaVeog/4tG8qacI="
)

// Regex compilée une seule fois au niveau package (évite MustCompile à chaque appel)
var validSecretID = regexp.MustCompile(`^[a-fA-F0-9]{32}$`)

// --- STRUCTURES DE DONNÉES ---

// CapsuleV3 : Format unifié compatible avec le protocole web (crypto.js)
type CapsuleV3 struct {
	Protocol    string            `json:"protocol"`
	Message     string            `json:"message"`
	Attachments []CapsuleFileV3   `json:"attachments"`
}

type CapsuleFileV3 struct {
	Name string `json:"name"`
	Mime string `json:"mime"`
	Data string `json:"data"`
}

// PaddedPayload : Identique au format JS (content + noise pour masquer la taille)
type PaddedPayload struct {
	Content string `json:"content"`
	Noise   string `json:"noise"`
}

// AAD V2.0 : Doit être identique à crypto.js pour que le déchiffrement fonctionne
type AADV2 struct {
	V   string `json:"v"`
	Alg string `json:"alg"`
	TTL int    `json:"ttl"`
}

type CreateReq struct {
	Payload     string `json:"payload"`
	TTL         int    `json:"ttl"`
	AadHash     string `json:"aad_hash"`
	HasPassword bool   `json:"has_password,omitempty"`
	PwdSalt     string `json:"pwd_salt,omitempty"`
}

type CreateRes struct {
	Success  bool   `json:"success"`
	SecretID string `json:"secret_id"`
	ProofID  string `json:"proof_id"`
	Error    string `json:"error"`
}

type FetchRes struct {
	Success     bool   `json:"success"`
	Payload     string `json:"payload"`
	HasPassword bool   `json:"has_password"`
	PwdSalt     string `json:"pwd_salt"`
	TTL         int    `json:"ttl"`
	Error       string `json:"error"`
}

// --- LOGIQUE CRYPTOGRAPHIQUE SÉCURISÉE ---

func generateKey() ([]byte, string, error) {
	k := make([]byte, 32)
	if _, err := io.ReadFull(rand.Reader, k); err != nil {
		return nil, "", fmt.Errorf("erreur critique CSPRNG: %w", err)
	}
	return k, base64.RawURLEncoding.EncodeToString(k), nil
}

// deriveKey : SHA-256(urlKeyBytes) — identique à crypto.js
func deriveKey(urlKey []byte) []byte {
	h := sha256.Sum256(urlKey)
	return h[:]
}

// buildAAD : Construit l'AAD identique à crypto.js pour la compatibilité
func buildAAD(ttl int) ([]byte, error) {
	aad := AADV2{V: "2.0", Alg: "AES-256-GCM", TTL: ttl}
	return json.Marshal(aad)
}

// padContent : Padding par paliers identique à crypto.js (anti traffic analysis)
func padContent(content string) PaddedPayload {
	length := len(content)
	targetLength := length

	if length < 1024 {
		targetLength = 1024
	} else if length < 10240 {
		targetLength = 10240
	} else {
		targetLength = length + 512
	}

	paddingSize := targetLength - length
	noise := make([]byte, paddingSize)
	io.ReadFull(rand.Reader, noise)

	return PaddedPayload{
		Content: content,
		Noise:   base64.StdEncoding.EncodeToString(noise),
	}
}

// encryptLocal : Chiffrement AES-256-GCM avec AAD — protocole unifié V2.0
func encryptLocal(messageJSON string, urlKey []byte, ttl int) (string, string, error) {
	// 1. Padding identique à crypto.js
	padded := padContent(messageJSON)
	plaintext, err := json.Marshal(padded)
	if err != nil {
		return "", "", fmt.Errorf("erreur de sérialisation JSON: %w", err)
	}

	// 2. Dérivation de clé : SHA-256(urlKey) — identique à crypto.js
	finalKey := deriveKey(urlKey)

	// 3. AES-256-GCM avec AAD
	block, err := aes.NewCipher(finalKey)
	if err != nil {
		return "", "", fmt.Errorf("erreur AES: %w", err)
	}
	gcm, err := cipher.NewGCM(block)
	if err != nil {
		return "", "", fmt.Errorf("erreur GCM: %w", err)
	}

	nonce := make([]byte, gcm.NonceSize())
	if _, err := io.ReadFull(rand.Reader, nonce); err != nil {
		return "", "", fmt.Errorf("erreur nonce CSPRNG: %w", err)
	}

	// 4. AAD : {"v":"2.0","alg":"AES-256-GCM","ttl":ttl}
	aadBytes, err := buildAAD(ttl)
	if err != nil {
		return "", "", fmt.Errorf("erreur AAD: %w", err)
	}

	sealed := gcm.Seal(nil, nonce, plaintext, aadBytes)

	// 5. AAD Hash pour le serveur (SHA-256 de l'AAD, pas du ciphertext)
	aadHash := sha256.Sum256(aadBytes)
	aadHashHex := hex.EncodeToString(aadHash[:])

	// 6. Format payload : ciphertext.iv (Base64URL) — identique à crypto.js
	payloadStr := base64.RawURLEncoding.EncodeToString(sealed) + "." + base64.RawURLEncoding.EncodeToString(nonce)
	return payloadStr, aadHashHex, nil
}

// decryptLocal : Déchiffrement AES-256-GCM avec AAD — protocole unifié V2.0
func decryptLocal(payloadStr string, keyB64 string, ttl int) (string, error) {
	urlKey, err := base64.RawURLEncoding.DecodeString(keyB64)
	if err != nil {
		return "", fmt.Errorf("clé URL invalide: %w", err)
	}

	parts := strings.Split(payloadStr, ".")
	if len(parts) != 2 {
		return "", fmt.Errorf("format de payload invalide")
	}

	sealed, err := base64.RawURLEncoding.DecodeString(parts[0])
	if err != nil {
		return "", fmt.Errorf("ciphertext invalide: %w", err)
	}
	nonce, err := base64.RawURLEncoding.DecodeString(parts[1])
	if err != nil {
		return "", fmt.Errorf("nonce invalide: %w", err)
	}

	// Dérivation identique à l'encryption
	finalKey := deriveKey(urlKey)

	block, err := aes.NewCipher(finalKey)
	if err != nil {
		return "", err
	}
	gcm, err := cipher.NewGCM(block)
	if err != nil {
		return "", err
	}

	// Reconstruction de l'AAD identique
	aadBytes, err := buildAAD(ttl)
	if err != nil {
		return "", err
	}

	plaintext, err := gcm.Open(nil, nonce, sealed, aadBytes)
	if err != nil {
		return "", fmt.Errorf("déchiffrement échoué (clé invalide, mot de passe incorrect ou donnée corrompue)")
	}

	// Extraction du contenu depuis le padding { content, noise }
	var padded PaddedPayload
	if err := json.Unmarshal(plaintext, &padded); err != nil {
		return "", fmt.Errorf("donnée corrompue ou format JSON invalide")
	}

	return padded.Content, nil
}

// --- COMMANDES HUMAINES (CLI) ---

func sendSecret(text string, filePath string, ttl string) {
	capsule := CapsuleV3{
		Protocol:    "ITYLOS_CAPSULE_V3_MULTI",
		Message:     "",
		Attachments: []CapsuleFileV3{},
	}

	if filePath != "" {
		bytesFile, err := os.ReadFile(filePath)
		if err != nil {
			color.Red("✘ Erreur de lecture du fichier : %v", err)
			return
		}
		if len(bytesFile) > 8*1024*1024 {
			color.Red("✘ Le fichier dépasse la limite de 8 Mo pour la V2.")
			return
		}
		cleanName := filepath.Base(filePath)
		capsule.Attachments = append(capsule.Attachments, CapsuleFileV3{
			Name: cleanName,
			Mime: "application/octet-stream",
			Data: "data:application/octet-stream;base64," + base64.StdEncoding.EncodeToString(bytesFile),
		})
		capsule.Message = text // Le texte peut accompagner le fichier
		color.Cyan("✓ Fichier chargé : %s (%d octets)", cleanName, len(bytesFile))
	} else {
		if text == "" {
			color.Red("✘ Le message est vide.")
			return
		}
		capsule.Message = text
	}

	ttlSeconds := 3600
	if ttl == "24h" { ttlSeconds = 86400 } else if ttl == "7j" { ttlSeconds = 604800 }

	urlKey, keyStr, err := generateKey()
	if err != nil {
		color.Red("✘ Erreur critique de génération de clé : %v", err)
		return
	}

	// Sérialisation de la capsule V3 en JSON (identique au format web)
	capsuleJSON, err := json.Marshal(capsule)
	if err != nil {
		color.Red("✘ Erreur de sérialisation : %v", err)
		return
	}

	payload, aadHashHex, err := encryptLocal(string(capsuleJSON), urlKey, ttlSeconds)
	if err != nil {
		color.Red("✘ Erreur interne de chiffrement : %v", err)
		return
	}

	reqData := CreateReq{Payload: payload, TTL: ttlSeconds, AadHash: aadHashHex}
	jsonData, _ := json.Marshal(reqData)

	resp, err := http.Post(API_CREATE, "application/json", bytes.NewBuffer(jsonData))
	if err != nil {
		color.Red("✘ Erreur réseau : %v", err)
		return
	}
	defer resp.Body.Close()

	var res CreateRes
	json.NewDecoder(resp.Body).Decode(&res)

	if !res.Success {
		color.Red("✘ Erreur API : %s", res.Error)
		return
	}

	color.Green("\n🦋 CAPSULE SÉCURISÉE AVEC SUCCÈS")
	fmt.Println(strings.Repeat("─", 50))
	color.New(color.FgCyan, color.Bold).Printf("LIEN SECRET : %s/v/%s#%s\n", DOMAIN, res.SecretID, keyStr)
	fmt.Println(strings.Repeat("─", 50))
	color.New(color.FgHiBlack, color.Italic).Println("La clé (#...) n'a jamais quitté cet ordinateur.")
}

func readSecret(url string) {
	parts := strings.Split(url, "#")
	if len(parts) != 2 {
		color.Red("✘ URL invalide. La clé (#...) est manquante.")
		return
	}
	keyStr := parts[1]

	urlParts := strings.Split(parts[0], "/")
	secretID := urlParts[len(urlParts)-1]

	if !validSecretID.MatchString(secretID) {
		color.Red("✘ L'identifiant de la capsule est malformé ou dangereux.")
		return
	}

	// 1. Fetch du secret chiffré
	resp, err := http.Get(API_FETCH + "?id=" + secretID)
	if err != nil {
		color.Red("✘ Erreur réseau : %v", err)
		return
	}
	defer resp.Body.Close()

	var res FetchRes
	json.NewDecoder(resp.Body).Decode(&res)

	if !res.Success {
		color.Red("✘ Erreur : %s", res.Error)
		return
	}

	if res.TTL == 0 {
		color.Red("✘ TTL absent dans la réponse serveur — déchiffrement impossible.")
		return
	}

	// 2. Mot de passe si requis
	if res.HasPassword {
		color.Yellow("⚠ Cette capsule est protégée par mot de passe.")
		color.Yellow("  Le CLI ne supporte pas encore les capsules protégées par mot de passe.")
		color.Yellow("  Ouvrez ce lien dans votre navigateur pour la déchiffrer.")
		return
	}

	// 3. Déchiffrement local (protocole unifié V2.0 avec AAD)
	contentJSON, err := decryptLocal(res.Payload, keyStr, res.TTL)
	if err != nil {
		color.Red("✘ %v", err)
		return
	}

	// 4. Affichage du contenu
	color.Green("\n🦋 CAPSULE DÉCHIFFRÉE")
	fmt.Println(strings.Repeat("─", 50))

	// Tentative de parsing V3 MULTI
	var capsule CapsuleV3
	if err := json.Unmarshal([]byte(contentJSON), &capsule); err == nil && capsule.Protocol == "ITYLOS_CAPSULE_V3_MULTI" {
		// Format V3 : message + attachments
		if capsule.Message != "" {
			fmt.Println(capsule.Message)
		}
		for _, att := range capsule.Attachments {
			// Extraction du base64 depuis le data URI
			dataParts := strings.SplitN(att.Data, ",", 2)
			b64Data := att.Data
			if len(dataParts) == 2 {
				b64Data = dataParts[1]
			}

			fileBytes, err := base64.StdEncoding.DecodeString(b64Data)
			if err != nil {
				color.Yellow("⚠ Impossible de décoder le fichier joint : %s", att.Name)
				continue
			}

			cleanName := filepath.Base(att.Name)
			if cleanName == "." || cleanName == "/" || cleanName == "\\" {
				cleanName = "secret_file.dat"
			}
			savePath := filepath.Join(".", cleanName)

			if err := os.WriteFile(savePath, fileBytes, 0600); err != nil {
				color.Red("✘ Impossible de sauvegarder : %s (%v)", savePath, err)
				continue
			}
			color.Cyan("📎 Fichier extrait : %s (%d octets)", savePath, len(fileBytes))
		}
	} else {
		// Format texte simple (fallback)
		fmt.Println(contentJSON)
	}

	fmt.Println(strings.Repeat("─", 50))

	// 5. Burn-on-read : destruction côté serveur
	burnData, _ := json.Marshal(map[string]string{"id": secretID})
	burnResp, err := http.Post(API_BURN, "application/json", bytes.NewBuffer(burnData))
	if err != nil {
		color.Yellow("⚠ Déchiffrement réussi, mais la destruction serveur a échoué : %v", err)
		return
	}
	defer burnResp.Body.Close()

	var burnRes struct {
		Success bool   `json:"success"`
		Error   string `json:"error"`
	}
	json.NewDecoder(burnResp.Body).Decode(&burnRes)

	if burnRes.Success {
		color.Green("🔥 Capsule détruite du serveur. Preuve de destruction générée.")
	} else {
		color.Yellow("⚠ Déchiffrement réussi, mais la purge serveur a retourné : %s", burnRes.Error)
	}
}

func verifyProof(proofPath string) {
	bytesFile, err := os.ReadFile(proofPath)
	if err != nil {
		color.Red("✘ Fichier introuvable : %v", err)
		return
	}

	var proof map[string]interface{}
	if err := json.Unmarshal(bytesFile, &proof); err != nil {
		color.Red("✘ JSON invalide : %v", err)
		return
	}

	veriBlock, ok := proof["verification"].(map[string]interface{})
	if !ok {
		color.Red("✘ Preuve malformée : bloc 'verification' absent ou invalide.")
		return
	}

	sigB64, ok := veriBlock["ed25519_signature"].(string)
	if !ok || sigB64 == "" || sigB64 == "unsigned" {
		color.Yellow("⚠ Ce document n'est pas signé (unsigned).")
		return
	}

	// Retirer la signature pour recalculer le hash du payload originel
	veriBlock["ed25519_signature"] = ""
	cleanPayload, err := json.Marshal(proof)
	if err != nil {
		color.Red("✘ Erreur de re-sérialisation de la preuve : %v", err)
		return
	}

	sigBytes, err := base64.StdEncoding.DecodeString(sigB64)
	if err != nil {
		color.Red("✘ Signature base64 invalide : %v", err)
		return
	}

	pubKeyBytes, err := base64.StdEncoding.DecodeString(SERVER_PUB_KEY_B64)
	if err != nil {
		color.Red("✘ Clé publique serveur invalide : %v", err)
		return
	}

	if len(pubKeyBytes) != ed25519.PublicKeySize {
		color.Red("✘ Clé publique Ed25519 de taille incorrecte (%d octets, attendu %d).", len(pubKeyBytes), ed25519.PublicKeySize)
		return
	}

	if ed25519.Verify(pubKeyBytes, cleanPayload, sigBytes) {
		color.Green("✔ PREUVE AUTHENTIQUE : La destruction a été confirmée cryptographiquement par ITYLOS.")
	} else {
		color.Red("✘ PREUVE FALSIFIÉE : La signature ne correspond pas à l'empreinte de la donnée.")
	}
}

// --- SERVEUR MCP (POUR LES IA : CLAUDE, CURSOR) ---

func startMCPServer() {
	scanner := bufio.NewScanner(os.Stdin)
	for scanner.Scan() {
		line := scanner.Text()
		var req map[string]interface{}
		if err := json.Unmarshal([]byte(line), &req); err != nil { continue }

		// [FIX MEDIUM 5] : Type Assertions sécurisées
		method, ok := req["method"].(string)
		if !ok { continue }
		id := req["id"]

		if method == "initialize" {
			sendMCPResponse(id, map[string]interface{}{
				"protocolVersion": "2024-11-05",
				"serverInfo": map[string]string{"name": "itylos-mcp", "version": VERSION},
				"capabilities": map[string]interface{}{"tools": map[string]interface{}{}},
			})
		} else if method == "tools/list" {
			sendMCPResponse(id, map[string]interface{}{
				"tools": []map[string]interface{}{
					{
						"name": "itylos_create_capsule",
						"description": "Chiffre un secret localement et génère un lien Itylos à lecture unique.",
						"inputSchema": map[string]interface{}{
							"type": "object",
							"properties": map[string]interface{}{
								"text": map[string]string{"type": "string", "description": "Le secret à sécuriser"},
							},
							"required": []string{"text"},
						},
					},
				},
			})
		} else if method == "tools/call" {
			params, ok := req["params"].(map[string]interface{})
			if !ok { continue }
			
			toolName, ok := params["name"].(string)
			if !ok { continue }
			
			args, ok := params["arguments"].(map[string]interface{})
			if !ok { continue }

			if toolName == "itylos_create_capsule" {
				text, ok := args["text"].(string)
				if !ok || text == "" {
					sendMCPResponse(id, map[string]interface{}{
						"isError": true, "content": []map[string]string{{"type": "text", "text": "Erreur: Le texte du secret est vide."}},
					})
					continue
				}
				
				capsule := CapsuleV3{Protocol: "ITYLOS_CAPSULE_V3_MULTI", Message: text, Attachments: []CapsuleFileV3{}}
				capsuleJSON, _ := json.Marshal(capsule)
				urlKey, keyStr, err := generateKey()
				if err != nil {
					sendMCPResponse(id, map[string]interface{}{
						"isError": true, "content": []map[string]string{{"type": "text", "text": "Erreur critique de génération de clé"}},
					})
					continue
				}
				payload, aad, err := encryptLocal(string(capsuleJSON), urlKey, 3600)
				if err != nil {
					sendMCPResponse(id, map[string]interface{}{
						"isError": true, "content": []map[string]string{{"type": "text", "text": "Erreur interne de chiffrement"}},
					})
					continue
				}

				jsonData, _ := json.Marshal(CreateReq{Payload: payload, TTL: 3600, AadHash: aad})
				resp, err := http.Post(API_CREATE, "application/json", bytes.NewBuffer(jsonData))

				if err == nil {
					var res CreateRes
					json.NewDecoder(resp.Body).Decode(&res)
					resp.Body.Close() // Close explicite au lieu de defer dans une boucle

					if res.Success {
						link := fmt.Sprintf("%s/v/%s#%s", DOMAIN, res.SecretID, keyStr)
						sendMCPResponse(id, map[string]interface{}{
							"content": []map[string]interface{}{
								{"type": "text", "text": "Capsule créée avec succès : " + link},
							},
						})
					} else {
						sendMCPResponse(id, map[string]interface{}{"isError": true, "content": []map[string]string{{"type": "text", "text": "Erreur serveur : " + res.Error}}})
					}
				} else {
					sendMCPResponse(id, map[string]interface{}{"isError": true, "content": []map[string]string{{"type": "text", "text": "Erreur réseau avec l'API ITYLOS"}}})
				}
			}
		}
	}
}

func sendMCPResponse(id interface{}, result interface{}) {
	resp := map[string]interface{}{"jsonrpc": "2.0", "id": id, "result": result}
	jsonBytes, _ := json.Marshal(resp)
	fmt.Println(string(jsonBytes)) // stdio transport
}

// --- ENTRY POINT ---

func main() {
	if len(os.Args) > 1 && os.Args[1] == "mcp" {
		startMCPServer()
		return
	}

	durPtr := flag.String("d", "1h", "Durée (1h, 24h, 7j)")
	filePtr := flag.String("f", "", "Fichier à chiffrer et envoyer")
	flag.Parse()
	args := flag.Args()

	if len(args) < 1 {
		w := color.New(color.FgWhite, color.Bold); y := color.New(color.FgCyan, color.Bold)
		fmt.Println("")
		w.Print("  ██╗████████╗") ; y.Print("██╗   ██╗") ; w.Println("██║      ██████╗ ███████╗")
		w.Print("  ██║╚══██╔══╝") ; y.Print("╚██╗ ██╔╝") ; w.Println("██║     ██╔═══██╗██╔════╝")
		w.Print("  ██║   ██║    ") ; y.Print("╚████╔╝ ") ; w.Println("██║     ██║   ██║███████╗")
		w.Print("  ██║   ██║     ") ; y.Print("╚██╔╝  ") ; w.Println("██║     ██║   ██║╚════██║")
		w.Print("  ██║   ██║      ") ; y.Print("██║   ") ; w.Println("███████╗╚██████╔╝███████║")
		w.Print("  ╚═╝   ╚═╝      ") ; y.Print("╚═╝   ") ; w.Println("╚══════╝ ╚═════╝ ╚══════╝")
		color.New(color.FgMagenta, color.Bold).Printf("\n          L'ART DE L'ÉPHÉMÈRE EN CLI • %s\n", VERSION)
		fmt.Println(strings.Repeat("─", 62))
		
		fmt.Println("\nCOMMANDES :")
		fmt.Println("  itylos send \"secret\"      : Chiffre et crée un lien éphémère")
		fmt.Println("  itylos send -f secret.pdf : Chiffre et envoie un fichier joint")
		fmt.Println("  itylos read <url>         : Déchiffre une capsule localement")
		fmt.Println("  itylos verify <proof.json>: Audite la signature de destruction")
		fmt.Println("  itylos mcp                : Démarre le serveur pour Intelligence Artificielle")
		os.Exit(0)
	}

	switch args[0] {
	case "send":
		msg := ""
		if len(args) > 1 { msg = args[1] }
		sendSecret(msg, *filePtr, *durPtr)
	case "read":
		if len(args) > 1 { readSecret(args[1]) }
	case "verify":
		if len(args) > 1 { verifyProof(args[1]) }
	}
}