<?php
/**
 * ITYLOS API v1.0.1-beta - EARLY ACCESS GATEWAY
 */

require_once('../../inc/config.php');
require_once('../../inc/functions.php');
require_once('../../inc/lang.php'); 

const CURRENT_VERSION = "1.0.1-beta";

header('Content-Type: application/json; charset=utf-8');

$action = $_GET['action'] ?? '';

if ($action === 'version') {
    echo json_encode(['status' => 'success', 'latest' => CURRENT_VERSION]);
    exit;
}

if ($action === 'save' && $_SERVER['REQUEST_METHOD'] === 'POST') {
    $data = json_decode(file_get_contents('php://input'), true);
    $payload = $data['content'] ?? '';
    if (empty($payload)) { exit(json_encode(['status' => 'error'])); }

    $method = 'aes-256-gcm';
    $iv_len = openssl_cipher_iv_length($method);
    $iv = openssl_random_pseudo_bytes($iv_len);
    $tag = ""; 
    $encrypted = openssl_encrypt($payload, $method, ENCRYPTION_KEY, OPENSSL_RAW_DATA, $iv, $tag, "", 16);
    $final_blob = base64_encode($iv . $tag . $encrypted);

    // IDENTIFIANT ALÉATOIRE DISCRET
    $secret_id = bin2hex(random_bytes(6)); 
    $m_token = bin2hex(random_bytes(32)); 
    $durations = ['1h' => 3600, '24h' => 86400, '7d' => 604800];
    $expires_at = date('Y-m-d H:i:s', time() + ($durations[$data['duration'] ?? '1h'] ?? 3600));

    try {
        $pdo->beginTransaction();
        $pdo->prepare("INSERT INTO secrets (id, content, expires_at, created_at) VALUES (?, ?, ?, NOW())")->execute([$secret_id, $final_blob, $expires_at]);
        $pdo->prepare("INSERT INTO receipts (secret_id, management_token, status, created_at) VALUES (?, ?, 'pending', NOW())")->execute([$secret_id, $m_token]);
        $pdo->commit();

        echo json_encode([
            'status' => 'success',
            'url' => "https://itylos.com/v/" . $secret_id,
            'proof_url' => "https://itylos.com/" . $lang . "/proof?t=" . $m_token
        ]);
    } catch (Exception $e) { if ($pdo->inTransaction()) $pdo->rollBack(); exit(json_encode(['status' => 'error'])); }
}
