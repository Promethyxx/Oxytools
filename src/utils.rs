// ═══════════════════════════════════════════════════════════════
//  OXYTOOLS — Fonctions utilitaires pures (aucune dépendance à OxytoolsApp)
// ═══════════════════════════════════════════════════════════════

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub fn expand_to_mkv(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut result = Vec::new();
    for p in paths {
        if p.is_dir() {
            if let Ok(entries) = std::fs::read_dir(p) {
                let mut sub: Vec<PathBuf> = entries
                    .filter_map(|e| e.ok().map(|e| e.path()))
                    .collect();
                sub.sort();
                for sp in &sub {
                    if sp.is_dir() {
                        result.extend(expand_to_mkv(std::slice::from_ref(sp)));
                    } else if sp.extension().is_some_and(|e| e.eq_ignore_ascii_case("mkv")) {
                        result.push(sp.clone());
                    }
                }
            }
        } else if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("mkv")) {
            result.push(p.clone());
        }
    }
    result
}

/// Lit la durée d'un fichier media via ffprobe. Retourne les secondes ou None.
pub fn get_duration_secs(path: &std::path::Path) -> Option<f64> {
    let out = crate::modules::binaries::silent_cmd(crate::modules::binaries::get_ffprobe())
        .args(["-v", "quiet", "-print_format", "json", "-show_entries",
               "format=duration", path.to_str()?])
        .output().ok()?;
    let json = String::from_utf8_lossy(&out.stdout);
    // "duration": "3725.123456"
    let key = "\"duration\":";
    let pos = json.find(key)?;
    let rest = json[pos + key.len()..].trim();
    let val = rest.trim_matches(|c| c == '"' || c == ' ' || c == '\n');
    val.split('"').next()?.trim().parse::<f64>().ok()
}

/// Parse "HH:MM:SS.ss" ou "MM:SS.ss" en secondes.
pub fn parse_time_to_secs(t: &str) -> Option<f64> {
    let parts: Vec<&str> = t.split(':').collect();
    match parts.as_slice() {
        [h, m, s] => {
            let secs = h.parse::<f64>().ok()? * 3600.0
                + m.parse::<f64>().ok()? * 60.0
                + s.parse::<f64>().ok()?;
            Some(secs)
        }
        [m, s] => Some(m.parse::<f64>().ok()? * 60.0 + s.parse::<f64>().ok()?),
        _ => None,
    }
}

/// Lance un Child ffmpeg et lit son stderr en temps réel pour mettre à jour conv_progress.
/// Bloque jusqu'à la fin du process. Retourne Ok/Err.
pub fn wait_ffmpeg_with_progress(
    mut child: std::process::Child,
    duration_secs: Option<f64>,
    conv_progress: &Arc<Mutex<f32>>,
    active_pids: &Arc<Mutex<Vec<u32>>>,
) -> Result<(), String> {
    use std::io::Read;

    // Enregistrer le PID pour pouvoir killer depuis on_exit
    let pid = child.id();
    active_pids.lock().unwrap_or_else(|e| e.into_inner()).push(pid);

    let stderr = child.stderr.take();
    let (tx, rx) = std::sync::mpsc::channel::<String>();

    if let Some(mut stderr) = stderr {
        std::thread::spawn(move || {
            let mut buf = [0u8; 512];
            let mut accum = String::new();
            loop {
                match stderr.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        accum.push_str(&String::from_utf8_lossy(&buf[..n]));
                        // ffmpeg sépare ses lignes de progress par \r
                        while let Some(pos) = accum.find('\r') {
                            let line = accum[..pos].to_string();
                            accum = accum[pos + 1..].to_string();
                            let _ = tx.send(line);
                        }
                    }
                }
            }
        });
    }

    *conv_progress.lock().unwrap_or_else(|e| e.into_inner()) = 0.0;

    loop {
        for line in rx.try_iter() {
            if let Some(pos) = line.find("time=") {
                let time_str = line[pos + 5..].split_whitespace().next().unwrap_or("");
                if let (Some(elapsed), Some(total)) = (parse_time_to_secs(time_str), duration_secs)
                    && total > 0.0 {
                        *conv_progress.lock().unwrap_or_else(|e| e.into_inner()) = (elapsed / total).min(1.0) as f32;
                    }
            }
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                *conv_progress.lock().unwrap_or_else(|e| e.into_inner()) = -1.0;
                active_pids.lock().unwrap_or_else(|e| e.into_inner()).retain(|&p| p != pid);
                return if status.success() { Ok(()) }
                       else { Err(format!("ffmpeg exited with code {:?}", status.code())) };
            }
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(100)),
            Err(e) => {
                *conv_progress.lock().unwrap_or_else(|e| e.into_inner()) = -1.0;
                active_pids.lock().unwrap_or_else(|e| e.into_inner()).retain(|&p| p != pid);
                return Err(format!("wait error: {}", e));
            }
        }
    }
}

pub fn parse_pages_spec(spec: &str) -> Option<Vec<u32>> {
    let trimmed = spec.trim();
    if trimmed.is_empty() || trimmed == "1-end" {
        return None;
    }
    let pages: Vec<u32> = trimmed.split(',')
        .filter_map(|s| s.trim().parse::<u32>().ok())
        .collect();
    if pages.is_empty() { None } else { Some(pages) }
}
