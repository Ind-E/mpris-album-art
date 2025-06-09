use anyhow::{Context, Result};
use base64::{Engine, engine::general_purpose};
use std::{collections::HashMap, env, process::Stdio};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
};

#[tokio::main]
async fn main() -> Result<()> {
    let mut art_map: HashMap<String, String> = HashMap::new();

    let mut child = Command::new("playerctl")
        .args([
            "--all-players",
            "metadata",
            "--format",
            "{{playerName}} {{mpris:artUrl}}",
            "--follow",
        ])
        .stdout(Stdio::piped())
        .spawn()
        .context("Failed to spawn playerctl. Is it installed and in your PATH?")?;

    let stdout = child
        .stdout
        .take()
        .context("Failed to get stdout from playerctl child process")?;

    let mut reader = BufReader::new(stdout).lines();

    let temp_dir = env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());

    println!("Listening for player metadata changes...");

    while let Some(line) = reader.next_line().await? {
        let Some((player, art_url)) = line.split_once(' ') else {
            continue;
        };

        let art_path = if let Some(url) = art_url.strip_prefix("file://") {
            url.to_string()
        } else if let Some(b64_data) = art_url
            .strip_prefix("data:image")
            .and_then(|s| s.split_once(";base64,"))
        {
            let image_data = match general_purpose::STANDARD.decode(b64_data.1) {
                Ok(data) => data,
                Err(e) => {
                    eprintln!("Failed to decode base64 for player {}: {}", player, e);
                    continue;
                }
            };

            let cache_file_path = format!("{}/{}_art.jpg", temp_dir, player);

            if let Err(e) = tokio::fs::write(&cache_file_path, &image_data).await {
                eprintln!("Failed to write cache file for player {}: {}", player, e);
                continue;
            }
            cache_file_path
        } else {
            continue;
        };

        art_map.insert(player.to_string(), art_path);

        let json_output = serde_json::to_string(&art_map)?;

        let json_arg = format!("album-art={}", json_output);
        tokio::spawn(async move {
            let mut eww_cmd = Command::new("eww");
            eww_cmd.arg("update").arg(&json_arg);

            if let Err(e) = eww_cmd.status().await {
                eprintln!("Failed to execute eww update: {}", e);
            }
        });

        println!("{}", json_output);
    }

    Ok(())
}
