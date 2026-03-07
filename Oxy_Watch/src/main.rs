use std::{thread, time::Duration, fs::OpenOptions, io::Write};
use serde_json::json;

fn main() {
    let client = reqwest::blocking::Client::new();
    let mut last_file = String::new();

    println!("--- SURVEILLANCE ACTIVE ---");

    loop {
        // On demande à VLC : "Tu lis quoi ?"
        let resp = client.get("http://localhost:8080/requests/status.json")
            .basic_auth("", Some("rust"))
            .send();

        if let Ok(r) = resp {
            if let Ok(data) = r.json::<serde_json::Value>() {
                let file = data["information"]["category"]["meta"]["filename"].as_str().unwrap_or("");
                let time = data["time"].as_f64().unwrap_or(0.0);
                let length = data["length"].as_f64().unwrap_or(1.0);
                
                let progress = (time / length) * 100.0;

                // LOGIQUE : Si > 90% et que c'est un nouveau fichier (Binge-watch)
                if progress >= 90.0 && file != last_file && !file.is_empty() {
                    let mut f = OpenOptions::new().create(true).append(true).open("historique.json").unwrap();
                    writeln!(f, "{}", json!({"video": file, "status": "VU"}).to_string()).unwrap();
                    
                    println!("✅ Loggé : {}", file);
                    last_file = file.to_string();
                }
            }
        }
        thread::sleep(Duration::from_secs(10));
    }
}