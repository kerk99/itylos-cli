<?php
/**
 * ITYLOS API v1.0.1-beta - EARLY ACCESS GATEWAY
 */

require_once('../../inc/config.php');
require_once('../../inc/functions.php');
require_once('../../inc/lang.php'); 

const CURRENT_VERSION = "1.0.1-beta";

header('Content-Type: application/json; charset=utf-8');

// Stabilisation de l'IP pour les quotas
function getRealIp() {
    if (!empty($_SERVER['HTTP_CLIENT_IP'])) return $_SERVER['HTTP_CLIENT_IP'];
    if (!empty($_SERVER['HTTP_X_FORWARDED_FOR'])) return explode(',', $_SERVER['HTTP_X_FORWARDED_FOR'])[0];
    return $_SERVER['REMOTE_ADDR'];
}

$action = $_GET['action'] ?? '';
$ip     = getRealIp();
$pepper = "itylos_butterfly_2026"; 

if ($action === 'version') {
    echo json_encode(['status' => 'success', 'latest' => CURRENT_VERSION, 'url' => 'https://itylos.com/download/itylos.exe']);
    exit;
}

if ($action === 'save' && $_SERVER['REQUEST_METHOD'] === 'POST') {
    $data = json_decode(file_get_contents('php://input'), true);
    $payload = $data['content'] ?? '';
    if (empty($payload)) exit(json_encode(['status' => 'error']));

    // Double-Shield Protocol
    $method = 'aes-256-gcm';
    $iv_len = openssl_cipher_iv_length($method);
    $iv = openssl_random_pseudo_bytes($iv_len);
    $tag = ""; 
    $encrypted = openssl_encrypt($payload, $method, ENCRYPTION_KEY, OPENSSL_RAW_DATA, $iv, $tag, "", 16);
    $final_blob = base64_encode($iv . $tag . $encrypted);

    // Identifiant discret
    $secret_id = bin2hex(random_bytes(6)); 
    $m_token = bin2hex(random_bytes(32)); 
    $durations = ['1h' => 3600, '24h' => 86400, '7d' => 604800];
    $expires_at = date('Y-m-d H:i:s', time() + ($durations[$data['duration'] ?? '1h'] ?? 3600));

    try {
        $pdo->beginTransaction();
        $pdo->prepare("INSERT INTO secrets (id, content, expires_at, created_at) VALUES (?, ?, ?, NOW())")->execute([$secret_id, $final_blob, $expires_at]);
        $pdo->prepare("INSERT INTO receipts (secret_id, management_token, status, created_at) VALUES (?, ?, 'pending', NOW())")->execute([$secret_id, $m_token]);
        
        // Quota Limit logic stabilisée
        $user_hash = hash('sha256', $ip . $pepper . date('YmdH'));
        $pdo->prepare("INSERT INTO api_quotas (user_hash, request_count, expires_at) VALUES (?, 1, ?) ON DUPLICATE KEY UPDATE request_count = request_count + 1")->execute([$user_hash, time() + 3600]);
        
        $pdo->commit();
        echo json_encode([
            'status' => 'success',
            'url' => "https://itylos.com/v/" . $secret_id,
            'proof_url' => "https://itylos.com/fr/proof?t=" . $m_token
        ]);
    } catch (Exception $e) { if ($pdo->inTransaction()) $pdo->rollBack(); exit(json_encode(['status' => 'error'])); }
}
