use std::collections::HashSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::sync::OnceLock;
use crate::modules::binaries;

// ════════════════════════════════════════════════════════════════════════
//  DÉTECTION DYNAMIQUE DES ENCODEURS DISPONIBLES DANS FFMPEG
// ════════════════════════════════════════════════════════════════════════

/// Liste des encodeurs vidéo réellement compilés dans le binaire ffmpeg utilisé
/// (bundled ou système) — interrogée une seule fois par lancement de l'appli.
/// Coder en dur "libx264 est toujours dispo" est faux dès qu'un ffmpeg custom
/// ou un build minimal est utilisé ; on demande directement à ffmpeg.
static ENCODEURS_DISPO: OnceLock<HashSet<String>> = OnceLock::new();

fn encodeurs_disponibles() -> &'static HashSet<String> {
    ENCODEURS_DISPO.get_or_init(|| {
        let out = binaries::silent_cmd(binaries::get_ffmpeg())
            .stdin(Stdio::null())
            .arg("-hide_banner")
            .arg("-encoders")
            .output();

        let mut set = HashSet::new();
        if let Ok(o) = out {
            let texte = String::from_utf8_lossy(&o.stdout);
            // Format de sortie ffmpeg -encoders : "  V..... libx264   H.264 / AVC ..."
            // On ne garde que le nom de l'encodeur (2e colonne), sur les lignes vidéo (V au début).
            for ligne in texte.lines() {
                let ligne = ligne.trim_start();
                if !ligne.starts_with('V') { continue; }
                if let Some(nom) = ligne.split_whitespace().nth(1) {
                    set.insert(nom.to_string());
                }
            }
        }
        crate::log_info(&format!("video::encodeurs_disponibles | {} encodeurs vidéo détectés", set.len()));
        set
    })
}

// ════════════════════════════════════════════════════════════════════════
//  VÉRIFICATION RÉELLE DES ACCÉLÉRATIONS MATÉRIELLES
// ════════════════════════════════════════════════════════════════════════

/// Accélérations matérielles dont on a vérifié qu'elles fonctionnent
/// *réellement* sur cette machine — pas juste "un GPU de cette marque est
/// listé par l'OS", mais "ffmpeg a réussi à encoder une image avec". Un GPU
/// détecté par Windows/Linux ne garantit pas que le driver ou l'accélération
/// matérielle correspondante fonctionne (VM, driver absent, adaptateur
/// désactivé...) — la seule vérification fiable est de le tester pour de vrai.
static ACCEL_FONCTIONNELLES: OnceLock<HashSet<&'static str>> = OnceLock::new();

fn accels_fonctionnelles() -> &'static HashSet<&'static str> {
    ACCEL_FONCTIONNELLES.get_or_init(|| {
        let mut ok = HashSet::new();
        for &accel in ACCEL_MATERIELLES {
            if tester_encodeur_reel(accel) {
                ok.insert(accel);
            }
        }
        crate::log_info(&format!("video::accels_fonctionnelles | testées et fonctionnelles : {:?}", ok));
        ok
    })
}

/// Tente un encodage minimal (1 image 64x64) avec l'encodeur h264 correspondant
/// à cette accélération. Sert de test représentatif pour toute la famille
/// (si h264_nvenc fonctionne, hevc_nvenc/av1_nvenc ont de bonnes chances de
/// fonctionner aussi sur le même matériel) — tester chaque combinaison
/// codec×accel individuellement multiplierait inutilement les micro-encodages.
fn tester_encodeur_reel(accel: &str) -> bool {
    let Some(encodeur) = nom_encodeur("h264", accel) else { return false; };
    if !encodeurs_disponibles().contains(&encodeur) {
        return false; // même pas compilé dans ce ffmpeg, inutile de tester
    }
    let resultat = binaries::silent_cmd(binaries::get_ffmpeg())
        .stdin(Stdio::null())
        .args(["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "color=black:size=64x64"])
        .args(["-frames:v", "1", "-c:v", &encodeur, "-f", "null", "-"])
        .output();
    matches!(resultat, Ok(o) if o.status.success())
}

fn accel_plausible(accel: &str) -> bool {
    accels_fonctionnelles().contains(accel)
}

const ACCEL_MATERIELLES: &[&str] = &["nvenc", "qsv", "amf", "videotoolbox"];

/// Nom d'encodeur ffmpeg pour un codec + une accélération donnés.
/// VP9 n'a pas d'accélération matérielle largement disponible/testée ici —
/// on ne fait jamais semblant qu'une combinaison hardware existe pour lui.
fn nom_encodeur(codec: &str, accel: &str) -> Option<String> {
    match (codec, accel) {
        ("h264", "software") => Some("libx264".into()),
        ("h264", hw) => Some(format!("h264_{hw}")),
        ("hevc", "software") => Some("libx265".into()),
        ("hevc", hw) => Some(format!("hevc_{hw}")),
        ("av1", "software") => Some("libsvtav1".into()),
        ("av1", hw) => Some(format!("av1_{hw}")),
        ("vp9", "software") => Some("libvpx-vp9".into()),
        ("vp9", _) => None,
        _ => None,
    }
}

/// Résout le vrai encodeur ffmpeg à utiliser pour un codec demandé, en tenant
/// compte de l'accélération demandée (ou "auto"), de ce que ce ffmpeg sait
/// réellement faire, et du GPU réellement présent sur la machine. Retombe sur
/// le logiciel si le matériel demandé/auto n'est pas disponible ou pas
/// plausible, plutôt que d'échouer avec une erreur ffmpeg cryptique.
fn resoudre_encodeur(codec: &str, accel_demande: &str) -> (String, bool) {
    let dispo = encodeurs_disponibles();

    if accel_demande != "auto" && accel_demande != "software" {
        // Accélération explicitement demandée par l'utilisateur — elle prime,
        // mais seulement si le GPU correspondant est réellement là.
        if accel_plausible(accel_demande)
            && let Some(enc) = nom_encodeur(codec, accel_demande)
            && dispo.contains(&enc) {
                return (enc, true);
            }
        crate::log_warn(&format!(
            "video::resoudre_encodeur | accélération '{}' demandée mais indisponible/non plausible pour {} — repli logiciel",
            accel_demande, codec
        ));
    } else if accel_demande == "auto" {
        // Auto : matériel dans l'ordre de priorité, uniquement parmi ce qui
        // est à la fois compilé dans ffmpeg ET plausible sur cette machine.
        for &accel in ACCEL_MATERIELLES {
            if !accel_plausible(accel) { continue; }
            if let Some(enc) = nom_encodeur(codec, accel)
                && dispo.contains(&enc) {
                    return (enc, true);
                }
        }
    }

    (nom_encodeur(codec, "software").unwrap_or_else(|| "libx264".into()), false)
}

// ════════════════════════════════════════════════════════════════════════
//  CONVERSION
// ════════════════════════════════════════════════════════════════════════

/// Convertit ou change le conteneur d'une vidéo.
/// `codec` : "auto" (h264 pour mp4/mkv/mov, vp9 pour webm), "h264", "hevc", "av1", "vp9".
/// `accel` : "auto" (matériel plausible si dispo sinon logiciel), "software",
///           "nvenc", "qsv", "amf", "videotoolbox".
#[allow(clippy::too_many_arguments)]
pub fn traiter_video(
    input: &PathBuf,
    output: &Path,
    copie_flux: bool,
    est_audio_uniquement: bool,
    speed: u32,
    codec: &str,
    accel: &str,
) -> Result<Child, std::io::Error> {
    let ffmpeg = binaries::get_ffmpeg();
    crate::log_info(&format!(
        "video::traiter_video | ffmpeg={:?} | copie_flux={} | audio_only={} | speed={} | codec={} | accel={} | {:?} -> {:?}",
        ffmpeg, copie_flux, est_audio_uniquement, speed, codec, accel, input, output
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

        // "auto" garde le comportement historique par conteneur ; un choix
        // explicite de l'utilisateur (h264/hevc/av1/vp9) prime toujours dessus.
        let codec_effectif = if codec == "auto" {
            match ext.as_str() {
                "webm" => "vp9",
                _ => "h264",
            }
        } else {
            codec
        };

        let (encodeur, est_materiel) = resoudre_encodeur(codec_effectif, accel);
        crate::log_info(&format!("video::traiter_video | encodeur choisi={} (matériel={})", encodeur, est_materiel));

        args.extend(["-c:v".into(), OsString::from(&encodeur)]);

        if est_materiel {
            // Les encodeurs matériels ont chacun leurs propres presets/options
            // (souvent incompatibles entre eux) — on laisse ffmpeg utiliser ses
            // valeurs par défaut plutôt que de deviner des flags spécifiques à
            // chaque fournisseur (Nvidia/Intel/AMD) sans pouvoir les tester ici.
        } else {
            match codec_effectif {
                "vp9" => {
                    let threads = num_cpus();
                    args.extend(["-row-mt", "1", "-deadline", "realtime"].map(OsString::from));
                    args.extend(["-threads".into(), OsString::from(threads.to_string())]);
                    args.extend(["-speed".into(), OsString::from(speed.to_string())]);
                },
                "hevc" => {
                    let preset = match speed {
                        0..=1 => "slow",
                        2..=4 => "medium",
                        5..=6 => "fast",
                        _     => "ultrafast",
                    };
                    args.extend(["-preset", preset].map(OsString::from));
                },
                "av1" => {
                    // libsvtav1 : preset numérique 0 (le plus lent/meilleur) à 13 (le plus rapide)
                    let preset_svt = match speed {
                        0..=1 => 4,
                        2..=4 => 7,
                        5..=6 => 10,
                        _     => 12,
                    };
                    args.extend(["-preset".into(), OsString::from(preset_svt.to_string())]);
                },
                _ => {
                    // h264 (et repli par défaut)
                    let preset = match speed {
                        0..=1 => "slow",
                        2..=4 => "medium",
                        5..=6 => "fast",
                        _     => "ultrafast",
                    };
                    args.extend(["-preset", preset].map(OsString::from));
                },
            }
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
        .args([
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

/// Liste, pour chaque codec, les accélérations réellement utilisables sur cette
/// machine avec ce ffmpeg (compilé ET matériel plausible) — pour construire les
/// listes déroulantes de l'UI sans jamais proposer une option vouée à échouer.
pub fn codecs_et_accelerations_disponibles() -> Vec<(&'static str, Vec<&'static str>)> {
    let dispo = encodeurs_disponibles();
    let mut resultat = Vec::new();

    for codec in ["h264", "hevc", "av1", "vp9"] {
        let mut accels = vec!["software"];
        for &accel in ACCEL_MATERIELLES {
            if !accel_plausible(accel) { continue; }
            if let Some(enc) = nom_encodeur(codec, accel)
                && dispo.contains(&enc) {
                    accels.push(accel);
                }
        }
        resultat.push((codec, accels));
    }
    resultat
}
