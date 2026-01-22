<?php
/**
 * ITYLOS API v1.0.1-beta - EARLY ACCESS GATEWAY
 * Propulsion : PHP 8.4 Secure Engine
 * Focus : Bienveillance, Double-Shield & Auto-Update
 */

// 1. CHARGEMENT DE L'INFRASTRUCTURE
require_once('../../inc/config.php');
require_once('../../inc/functions.php');
require_once('../../inc/lang.php'); 

// Paramètres de l'accès anticipé (Version synchronisée avec itylos.go)
const CURRENT_VERSION = "1.0.1-beta";
const UPDATE_URL      = "https://itylos.com/download/itylos.exe";

// Paramètres de sécurité (Quotas anonymes)
const MAX_MSG_PER_HOUR = 10;
const SECRET_PEPPER    = "itylos_butterfly_early_access_2026"; 

header('Content-Type: application/json; charset=utf-8');
header('X-Content-Type-Options: nosniff');
header('X-Frame-Options: DENY');

$action = $_GET['action'] ?? '';
$ip     = $_SERVER['REMOTE_ADDR'] ?? '0.0.0.0';

/**
 * PROTECTION ANONYME : Rate Limiting
 * On protège le serveur sans jamais identifier l'humain.
 */
function isAbusing($pdo, $ip) {
    $hour = date('YmdH'); 
    $user_hash = hash('sha256', $ip . SECRET_PEPPER . $hour);
    $now = time();

    $stmt = $pdo->prepare("SELECT request_count FROM api_quotas WHERE user_hash = ?");
    $stmt->execute([$user_hash]);
    $quota = $stmt->fetch();

    if ($quota) {
        if ($quota['request_count'] >= MAX_MSG_PER_HOUR) return true;
        $pdo->prepare("UPDATE api_quotas SET request_count = request_count + 1 WHERE user_hash = ?")->execute([$user_hash]);
    } else {
        $pdo->prepare("INSERT INTO api_quotas (user_hash, request_count, expires_at) VALUES (?, 1, ?)")->execute([$user_hash, $now + 3600]);
    }
    return false;
}

/**
 * ACTION : VERSION (Vérification des mises à jour)
 */
if ($action === 'version') {
    echo json_encode([
        'status'  => 'success',
        'latest'  => CURRENT_VERSION,
        'url'     => UPDATE_URL,
        'message' => 'Nouveau système de mise à jour actif. 🦋'
    ]);
    exit;
}

/**
 * ACTION : SAVE (Sécurisation et Envoi)
 */
if ($action === 'save' && $_SERVER['REQUEST_METHOD'] === 'POST') {
    if (isAbusing($pdo, $ip)) {
        http_response_code(429);
        exit(json_encode(['status' => 'error', 'message' => 'Quota atteint.']));
    }

    $data = json_decode(file_get_contents('php://input'), true);
    $payload = $data['content'] ?? ''; // Format "iv.cipher" du terminal
    
    if (empty($payload)) { exit(json_encode(['status' => 'error', 'message' => 'Void'])); }

    // --- COUCHE 2 : DOUBLE-SHIELD (Serveur) ---
    // v.php déchiffrera cette couche avant de passer au client
    $method = 'aes-256-gcm';
    $iv_len = openssl_cipher_iv_length($method);
    $iv     = openssl_random_pseudo_bytes($iv_len);
    $tag    = ""; 
    
    $encrypted = openssl_encrypt($payload, $method, ENCRYPTION_KEY, OPENSSL_RAW_DATA, $iv, $tag, "", 16);
    $final_blob = base64_encode($iv . $tag . $encrypted);

    // Identifiants via ton moteur sémantique
    $secret_id = generateItylosSlug("Early Access Transmission");
    $m_token   = bin2hex(random_bytes(32)); 
    $durations = ['1h' => 3600, '24h' => 86400, '7d' => 604800];
    $seconds   = $durations[$data['duration'] ?? '1h'] ?? 3600;
    $expires_at = date('Y-m-d H:i:s', time() + $seconds);

    try {
        $pdo->beginTransaction();
        $pdo->prepare("INSERT INTO secrets (id, content, expires_at, created_at) VALUES (?, ?, ?, NOW())")->execute([$secret_id, $final_blob, $expires_at]);
        $pdo->prepare("INSERT INTO receipts (secret_id, management_token, status, created_at) VALUES (?, ?, 'pending', NOW())")->execute([$secret_id, $m_token]);
        $pdo->commit();

        echo json_encode([
            'status'     => 'success',
            'url'        => "https://itylos.com/v/" . $secret_id,
            'proof_url'  => "https://itylos.com/" . $lang . "/proof?t=" . $m_token
        ]);
    } catch (Exception $e) {
        if ($pdo->inTransaction()) $pdo->rollBack();
        exit(json_encode(['status' => 'error']));
    }
}

/**
 * ACTION : FETCH (Pour la commande 'get' du CLI)
 */
elseif ($action === 'fetch' && isset($_GET['slug'])) {
    $slug = $_GET['slug'];
    $stmt = $pdo->prepare("SELECT content FROM secrets WHERE id = ? AND expires_at > NOW()");
    $stmt->execute([$slug]);
    $secret = $stmt->fetch();

    if ($secret) {
        $pdo->beginTransaction();
        // Déchiffrement de l'armure serveur
        $raw = base64_decode($secret['content']);
        $iv_l = openssl_cipher_iv_length('aes-256-gcm');
        $outer_decrypted = openssl_decrypt(substr($raw, $iv_l + 16), 'aes-256-gcm', ENCRYPTION_KEY, OPENSSL_RAW_DATA, substr($raw, 0, $iv_l), substr($raw, $iv_l, 16));
        
        $pdo->prepare("UPDATE receipts SET status = 'burned', burned_at = NOW() WHERE secret_id = ?")->execute([$slug]);
        $pdo->prepare("DELETE FROM secrets WHERE id = ?")->execute([$slug]);
        $pdo->commit();
        
        echo json_encode(['status' => 'success', 'content' => $outer_decrypted]);
    } else {
        http_response_code(404);
        echo json_encode(['status' => 'error']);
    }
}