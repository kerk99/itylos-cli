use anyhow::Result;
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};

use crate::{network::ItylosApi, services};

pub fn start_mcp_server() -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let api = ItylosApi::new()?;

    for line in stdin.lock().lines() {
        let line = line?;
        let req: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(_) => continue,
        };

        let method = match req.get("method").and_then(Value::as_str) {
            Some(method) => method,
            None => continue,
        };
        let id = req.get("id").cloned().unwrap_or(Value::Null);

        let result = match method {
            "initialize" => json!({
                "protocolVersion": "2024-11-05",
                "serverInfo": { "name": "itylos-mcp", "version": crate::types::VERSION },
                "capabilities": { "tools": {} }
            }),
            "tools/list" => json!({
                "tools": [{
                    "name": "itylos_create_capsule",
                    "description": "Chiffre un secret localement et genere un lien Itylos a lecture unique.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "text": { "type": "string", "description": "Le secret a securiser" }
                        },
                        "required": ["text"]
                    }
                }]
            }),
            "tools/call" => handle_tool_call(&api, &req),
            _ => continue,
        };

        let response = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        });
        writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
        stdout.flush()?;
    }

    Ok(())
}

fn handle_tool_call(api: &ItylosApi, request: &Value) -> Value {
    let params = match request.get("params").and_then(Value::as_object) {
        Some(params) => params,
        None => {
            return error_response("Requete MCP invalide.");
        }
    };
    let tool_name = match params.get("name").and_then(Value::as_str) {
        Some(name) => name,
        None => return error_response("Nom d'outil manquant."),
    };
    if tool_name != "itylos_create_capsule" {
        return error_response("Outil MCP inconnu.");
    }

    let text = params
        .get("arguments")
        .and_then(Value::as_object)
        .and_then(|args| args.get("text"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    if text.is_empty() {
        return error_response("Erreur: Le texte du secret est vide.");
    }

    match services::create_capsule_link(api, text.to_string(), None, crate::types::Ttl::OneHour) {
        Ok(link) => json!({
            "content": [{ "type": "text", "text": format!("Capsule creee avec succes : {link}") }]
        }),
        Err(_) => error_response("Erreur reseau avec l'API ITYLOS"),
    }
}

fn error_response(message: &str) -> Value {
    json!({
        "isError": true,
        "content": [{ "type": "text", "text": message }]
    })
}
