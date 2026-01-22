package main

import (
	"bytes"
	"crypto/aes"
	"crypto/cipher"
	"crypto/rand"
	"encoding/base64"
	"encoding/json"
	"flag"
	"fmt"
	"io"
	"net/http"
	"os"
	"strings"
	"time"

	"github.com/fatih/color"
)

// --- CONFIGURATION GLOBALE ---
const (
	DOMAIN  = "https://itylos.com"
	API_URL = DOMAIN + "/api/v1/cli.php"
	VERSION = "v1.0.1-beta"
)

// --- SYSTÈME DE TRADUCTION INTÉGRAL ---
type Translation struct {
	Title, Mission, Intro, Usage, Options, Examples, Success, Share, Proof, Note, Err, Faq string
}

var Locales = map[string]Translation{
	"fr": {
		Title:   "ENVOI DE MESSAGES CHIFFRÉS",
		Mission: "MANIFESTE DE BIENVEILLANCE ITYLOS\n" +
			"1. PROTÉGER : Le web n'oublie rien, nous vous redonnons le droit à l'oubli.\n" +
			"2. RESPECTER : Vos secrets ne nous regardent pas. Ils sont chiffrés chez vous.\n" +
			"3. ÉDUQUER : Protégez votre vie privée et celle de vos proches.\n" +
			"4. RESPONSABILISER : Un message envoyé est un acte de confiance, pas une trace.",
		Faq: "QUESTIONS FRÉQUEMMENT POSÉES :\n" +
			"Q: Où est stockée la clé ?\n" +
			"R: Uniquement dans l'URL (#) sur votre terminal. Elle ne traverse jamais le réseau.\n\n" +
			"Q: Pourquoi l'auto-destruction ?\n" +
			"R: Pour garantir qu'une fuite de données future ne compromette pas vos secrets passés.\n\n" +
			"Q: Qui a créé ITYLOS ?\n" +
			"R: Une initiative pour une tech plus humaine et souveraine.",
		Intro:   "Souveraineté activée. Votre message est crypté et protégé.",
		Usage:   "COMMANDES DISPONIBLES :",
		Options: "DURÉES AVANT EFFACEMENT : 1h (par défaut), 24h, 7j (7 jours)",
		Examples: "EXEMPLES BIENVEILLANTS :",
		Success: "🦋 MESSAGE ENVOYÉ : IL EST MAINTENANT SÉCURISÉ",
		Share:   "LIEN À ENVOYER (S'effacera tout seul après lecture)",
		Proof:   "PREUVE D'EFFACEMENT (Pour vérifier que tout a été supprimé)",
		Note:    "Note : La clé (#) reste sur votre ordinateur. ITYLOS est aveugle.",
		Err:     "Erreur : Le message est vide.",
	},
	"en": {
		Title:   "SECURE MESSAGE SENDER",
		Mission: "ITYLOS BENEVOLENCE MANIFESTO\n" +
			"1. PROTECT: The web forgets nothing, we give you back the right to be forgotten.\n" +
			"2. RESPECT: Your secrets are none of our business. They are encrypted locally.\n" +
			"3. EDUCATE: Protect your privacy and that of your loved ones.\n" +
			"4. EMPOWER: A sent message is an act of trust, not a permanent trace.",
		Faq: "FREQUENTLY ASKED QUESTIONS:\n" +
			"Q: Where is the key stored?\n" +
			"R: Only in the URL (#) on your terminal. It never crosses the network.\n\n" +
			"Q: Why self-destruction?\n" +
			"R: To ensure that a future data breach doesn't compromise your past secrets.\n\n" +
			"Q: Who built ITYLOS?\n" +
			"R: An initiative for a more human and sovereign tech.",
		Intro:   "Sovereignty active. Your message is encrypted and protected.",
		Usage:   "AVAILABLE COMMANDS:",
		Options: "DURATIONS BEFORE DELETION: 1h (default), 24h, 7d (7 days)",
		Examples: "BENEVOLENT EXAMPLES:",
		Success: "🦋 MESSAGE SENT: IT IS NOW SECURE",
		Share:   "LINK TO SEND (Will self-destruct after reading)",
		Proof:   "DELETION PROOF (To check that everything is deleted)",
		Note:    "Note: The key (#) stays on your computer. ITYLOS is blind.",
		Err:     "Error: Message cannot be empty.",
	},
}

// --- INTERFACE VISUELLE ---

func drawLogo() {
	w := color.New(color.FgWhite, color.Bold); y := color.New(color.FgCyan, color.Bold)
	fmt.Println("")
	w.Print("  ██╗████████╗") ; y.Print("██╗   ██╗") ; w.Println("██║      ██████╗ ███████╗")
	w.Print("  ██║╚══██╔══╝") ; y.Print("╚██╗ ██╔╝") ; w.Println("██║     ██╔═══██╗██╔════╝")
	w.Print("  ██║   ██║    ") ; y.Print("╚████╔╝ ") ; w.Println("██║     ██║   ██║███████╗")
	w.Print("  ██║   ██║     ") ; y.Print("╚██╔╝  ") ; w.Println("██║     ██║   ██║╚════██║")
	w.Print("  ██║   ██║      ") ; y.Print("██║   ") ; w.Println("███████╗╚██████╔╝███████║")
	w.Print("  ╚═╝   ╚═╝      ") ; y.Print("╚═╝   ") ; w.Println("╚══════╝ ╚═════╝ ╚══════╝")
}

func drawHeader(t Translation) {
	drawLogo()
	color.New(color.FgMagenta, color.Bold).Printf("\n          %s • %s • 🦋\n", t.Title, VERSION)
	fmt.Println(strings.Repeat("─", 62))
	color.New(color.FgHiBlack, color.Italic).Println(t.Intro)
	fmt.Println(strings.Repeat("─", 62))
}

func drawBox(title, content string, c *color.Color) {
	maxWidth := 78
	tLen := len([]rune(title)) // Calcul sur les caractères réels pour UTF-8
	repeatCount := maxWidth - tLen - 5
	if repeatCount < 0 { repeatCount = 0 }
	c.Printf("┌── %s %s\n", title, strings.Repeat("─", repeatCount))
	fmt.Printf("│  %s\n", content)
	c.Println("└" + strings.Repeat("─", maxWidth-1))
}

func send(msg, duration, lang string) {
	t := Locales[lang]
	if msg == "" { color.Red(t.Err); return }
	k := make([]byte, 32); io.ReadFull(rand.Reader, k)
	keyFrag := base64.RawURLEncoding.EncodeToString(k)
	
	block, _ := aes.NewCipher(k); gcm, _ := cipher.NewGCM(block)
	nonce := make([]byte, gcm.NonceSize()); io.ReadFull(rand.Reader, nonce)
	sealed := gcm.Seal(nil, nonce, []byte(msg), nil)
	payload := base64.StdEncoding.EncodeToString(nonce) + "." + base64.StdEncoding.EncodeToString(sealed)

	data, _ := json.Marshal(map[string]string{"content": payload, "duration": duration})
	resp, err := http.Post(API_URL+"?action=save&l="+lang, "application/json", bytes.NewBuffer(data))
	if err != nil { color.Red("✘ Service indisponible."); return }
	defer resp.Body.Close()

	var res map[string]string
	json.NewDecoder(resp.Body).Decode(&res)

	color.New(color.FgGreen, color.Bold).Println("\n" + t.Success)
	drawBox(t.Share, res["url"]+"#"+keyFrag, color.New(color.FgCyan, color.Bold))
	drawBox(t.Proof, res["proof_url"], color.New(color.FgYellow, color.Bold))
	color.New(color.FgHiBlack, color.Italic).Println("\n " + t.Note + "\n")
}

func main() {
	langPtr := flag.String("l", "fr", "Langue"); durPtr := flag.String("d", "1h", "Durée"); flag.Parse()
	lang := *langPtr
	if _, ok := Locales[lang]; !ok { lang = "fr" }
	t := Locales[lang]
	args := flag.Args()

	if len(args) < 1 {
		drawHeader(t)
		color.New(color.FgCyan).Println(t.Mission)
		color.New(color.FgYellow, color.Bold).Printf("\n%s\n", t.Usage)
		fmt.Println("  itylos send \"message\"   : Sécuriser un message")
		fmt.Println("  itylos mission          : Notre manifeste de bienveillance")
		fmt.Println("  itylos faq              : Questions fréquentes")
		fmt.Println("  itylos status           : État du service")
		
		color.New(color.FgHiBlack).Printf("\n%s\n", t.Options) // LIGNE RÉTABLIE

		color.New(color.FgMagenta, color.Bold).Printf("\n%s\n", t.Examples)
		if lang == "fr" {
			color.White("  itylos send \"Voici le code d'accès temporaire : 8822\" -d 1h")
			color.White("  itylos send \"Document confidentiel pour la réunion de demain\" -d 24h")
		} else {
			color.White("  itylos send \"Temporary access code: 8822\" -d 1h")
			color.White("  itylos send \"Confidential document for tomorrow's meeting\" -d 24h")
		}
		os.Exit(0)
	}

	switch args[0] {
	case "send":
		if len(args) > 1 { drawHeader(t); send(args[1], *durPtr, lang) }
	case "mission":
		drawHeader(t)
		color.Cyan(t.Mission)
	case "faq":
		drawHeader(t)
		color.Cyan(t.Faq)
	case "status":
		resp, _ := http.Get(DOMAIN)
		if resp != nil && resp.StatusCode == 200 { color.Green("\n ✔ SERVICE OPÉRATIONNEL. 🦋") }
	}
}
