#![allow(dead_code)]
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio}; // Stdio importé pour pouvoir utiliser .null()
use crate::modules::binaries;

/// Convertit ou change le conteneur d'une vidéo
pub fn traiter_video(
    input: &PathBuf,
    output: &Path,
    copie_flux: bool,
    est_audio_uniquement: bool,
    speed: u32,
) -> Result<Child, std::io::Error> {
    let ffmpeg = binaries::get_ffmpeg();
    crate::log_info(&format!(
        "video::traiter_video | ffmpeg={:?} | copie_flux={} | audio_only={} | speed={} | {:?} -> {:?}",
        ffmpeg, copie_flux, est_audio_uniquement, speed, input, output
    ));

    // -nostdin évite que FFmpeg attende une commande utilisateur en arrière-plan
    let mut args: Vec<OsString> = vec![
        "-nostdin".into(),
        "-i".into(),
        input.as_os_str().to_os_string(),
    ];

    if copie_flux {
        if est_audio_uniquement {
            args.extend(["-vn", "-c:a", "copy"].map(OsString::from));
        } else {
            args.extend(["-c", "copy"].map(OsString::from));
        }
    } else if est_audio_uniquement {
        args.push("-vn".into());
    } else {
        let ext = output
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        match ext.as_str() {
            "webm" => {
                let threads = num_cpus();
                // Ajout de -deadline realtime pour booster la vitesse du VP9
                args.extend(["-c:v", "libvpx-vp9", "-row-mt", "1", "-deadline", "realtime"].map(OsString::from));
                args.extend(["-threads".into(), OsString::from(threads.to_string())]);
                args.extend(["-speed".into(), OsString::from(speed.to_string())]);
            },
            "mp4" | "mkv" | "mov" => {
                let preset = match speed {
                    0..=1 => "slow",
                    2..=4 => "medium",
                    5..=6 => "fast",
                    _     => "ultrafast",
                };
                args.extend(["-c:v", "libx264", "-preset", preset].map(OsString::from));
            },
            _ => {}
        }
    }

    args.extend(["-y".into(), output.as_os_str().to_os_string()]);

    // On redirige tout vers le néant : pas de logs, pas de buffers pleins, pas de blocage
    let child = binaries::silent_cmd(binaries::get_ffmpeg())
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn();

    if let Err(ref e) = child {
        crate::log_error(&format!("video::traiter_video impossible de lancer ffmpeg : {}", e));
    }
    child
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

/// Analyse le codec audio d'un fichier via ffprobe
pub fn extraire_nom_codec(input: &PathBuf) -> String {
    let out = binaries::silent_cmd(binaries::get_ffprobe())
        .stdin(Stdio::null())
        .args(&[
            "-v",
            "error",
            "-select_streams",
            "a:0",
            "-show_entries",
            "stream=codec_name",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(input)
        .output();

    if let Ok(o) = out {
        String::from_utf8_lossy(&o.stdout).trim().to_string()
    } else {
        String::new()
    }
}