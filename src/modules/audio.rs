#![allow(dead_code)]
use std::path::Path;
use crate::modules::binaries;

/// Conversion & Compression
/// qualite : VBR 0 (meilleure qualite) a 9 (plus leger), mappe selon le codec de sortie
pub fn convertir(input: &Path, output: &str, qualite: u32) -> std::io::Result<std::process::Child> {
    let ffmpeg = binaries::get_ffmpeg();
    crate::log_info(&format!("audio::convertir | ffmpeg={:?} | qualite={} | {:?} -> {}", ffmpeg, qualite, input, output));
    let ext = std::path::Path::new(output)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let mut cmd = binaries::silent_cmd(ffmpeg);
    cmd.arg("-i").arg(input);
    match ext.as_str() {
        "mp3" => {
            cmd.args(["-codec:a", "libmp3lame", "-q:a", &qualite.to_string()]);
        },
        "ogg" => {
            let q = (9u32.saturating_sub(qualite) + 1).min(10);
            cmd.args(["-codec:a", "libvorbis", "-q:a", &q.to_string()]);
        },
        "opus" => {
            let bitrate = 128u32.saturating_sub(qualite * 10).max(32);
            cmd.args(["-codec:a", "libopus", "-b:a", &format!("{}k", bitrate)]);
        },
        "aac" | "m4a" => {
            let q = (5u32.saturating_sub(qualite / 2)).max(1);
            cmd.args(["-codec:a", "aac", "-q:a", &q.to_string()]);
        },
        "flac" => {
            cmd.args(["-codec:a", "flac", "-compression_level", &qualite.to_string()]);
        },
        _ => {
            let bitrate = 320u32.saturating_sub(qualite * 28).max(64);
            cmd.args(["-b:a", &format!("{}k", bitrate)]);
        },
    }
    cmd.arg("-y").arg(output);
    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let child = cmd.spawn();
    if let Err(ref e) = child {
        crate::log_error(&format!("audio::convertir impossible de lancer ffmpeg : {}", e));
    }
    child
}

/// Détecter le codec via ffprobe
pub fn detecter_extension(input: &Path) -> String {
    let result = binaries::silent_cmd(binaries::get_ffprobe())
        .args([
            "-v", "error",
            "-select_streams", "a:0",
            "-show_entries", "stream=codec_name",
            "-of", "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(input)
        .output();

    let codec = match result {
        Ok(output) => String::from_utf8_lossy(&output.stdout).trim().to_string(),
        Err(e) => {
            crate::log_warn(&format!("audio::detecter_extension ffprobe échoué : {}", e));
            return String::new();
        }
    };

    match codec.as_str() {
        "vorbis" => "ogg".into(),
        "opus" => "opus".into(),
        _ => codec,
    }
}

/// Retourne les formats de sortie compatibles selon le codec source
pub fn formats_compatibles(codec_source: &str) -> Vec<&'static str> {
    // ffmpeg gère la quasi-totalité des conversions, mais on filtre
    // les cas problématiques connus
    match codec_source {
        "pcm_s16le" | "pcm_s24le" | "pcm_s32le" | "pcm_f32le" | "flac" | "alac" =>
            vec!["mp3", "aac", "flac", "ogg", "opus", "wav", "m4a"],
        "vorbis" =>
            vec!["mp3", "aac", "flac", "ogg", "opus", "wav", "m4a"],
        "opus" =>
            vec!["mp3", "aac", "flac", "ogg", "opus", "wav", "m4a"],
        "aac" =>
            vec!["mp3", "aac", "flac", "ogg", "opus", "wav", "m4a"],
        "mp3" =>
            vec!["mp3", "aac", "flac", "ogg", "opus", "wav", "m4a"],
        // Codec inconnu ou vide → tout proposer, ffmpeg tentera
        _ =>
            vec!["mp3", "aac", "flac", "ogg", "opus", "wav", "m4a"],
    }
}

/// Extraction : Récupère l'audio d'une vidéo
pub fn extraire(input: &Path, output: &str) -> std::io::Result<std::process::Child> {
    crate::log_info(&format!("audio::extraire | {:?} -> {}", input, output));
    let child = binaries::silent_cmd(binaries::get_ffmpeg())
        .arg("-i")
        .arg(input)
        .arg("-vn")
        .arg("-c:a")
        .arg("copy")
        .arg("-y")
        .arg(output)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();
    if let Err(ref e) = child {
        crate::log_error(&format!("audio::extraire impossible de lancer ffmpeg : {}", e));
    }
    child
}