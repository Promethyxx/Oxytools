use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

static TOOLS_DIR: OnceLock<Option<PathBuf>> = OnceLock::new();

#[cfg(target_os = "windows")]
const EXT: &str = ".exe";
#[cfg(not(target_os = "windows"))]
const EXT: &str = "";

// ════════════════════════════════════════════════════════════════════════
//  BINAIRES EMBARQUÉS PAR PLATEFORME
// ════════════════════════════════════════════════════════════════════════
#[cfg(all(feature = "bundled", target_os = "windows", target_arch = "x86_64"))]
mod embedded {
    pub const FFMPEG:      &[u8] = include_bytes!("../../bin/ffmpeg.exe");
    pub const FFPROBE:     &[u8] = include_bytes!("../../bin/ffprobe.exe");
    pub const MKVPROPEDIT: &[u8] = include_bytes!("../../bin/mkvpropedit.exe");
}
#[cfg(all(feature = "bundled", target_os = "linux", target_arch = "x86_64"))]
mod embedded {
    pub const FFMPEG:      &[u8] = include_bytes!("../../bin-linux-x64/ffmpeg");
    pub const FFPROBE:     &[u8] = include_bytes!("../../bin-linux-x64/ffprobe");
    pub const MKVPROPEDIT: &[u8] = include_bytes!("../../bin-linux-x64/mkvpropedit");
}
#[cfg(all(feature = "bundled", target_os = "linux", target_arch = "aarch64"))]
mod embedded {
    pub const FFMPEG:      &[u8] = include_bytes!("../../bin-linux-arm/ffmpeg");
    pub const FFPROBE:     &[u8] = include_bytes!("../../bin-linux-arm/ffprobe");
    pub const MKVPROPEDIT: &[u8] = include_bytes!("../../bin-linux-arm/mkvpropedit");
}
#[cfg(all(feature = "bundled", target_os = "macos", target_arch = "aarch64"))]
mod embedded {
    pub const FFMPEG:      &[u8] = include_bytes!("../../bin-mac-arm/ffmpeg");
    pub const FFPROBE:     &[u8] = include_bytes!("../../bin-mac-arm/ffprobe");
    pub const MKVPROPEDIT: &[u8] = include_bytes!("../../bin-mac-arm/mkvpropedit");
}

// ════════════════════════════════════════════════════════════════════════
//  INTÉGRITÉ — SHA-256 des binaires embarqués/extraits
// ════════════════════════════════════════════════════════════════════════
fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    result.iter().map(|b| format!("{:02x}", b)).collect()
}

// ════════════════════════════════════════════════════════════════════════
//  EXTRACTION
// ════════════════════════════════════════════════════════════════════════
pub fn extraire_deps() -> Result<(), String> {
    #[cfg(feature = "bundled")]
    {
        let temp_dir = std::env::temp_dir().join("oxytools_tools");
        crate::log_info(&format!("binaries::extraire_deps | dossier temp={:?}", temp_dir));
        if !temp_dir.exists() {
            std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
        }

        // Restreint l'accès au dossier au seul propriétaire du process.
        // Empêche un autre utilisateur de la machine de lire, remplacer
        // ou pré-créer les binaires avant nous (multi-utilisateur / TOCTOU).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&temp_dir, std::fs::Permissions::from_mode(0o700))
                .map_err(|e| e.to_string())?;
        }

        let f = |name: &str, bytes: &[u8]| -> Result<(), String> {
            let path = temp_dir.join(name);
            let expected = sha256_hex(bytes);

            // Ré-extrait si absent OU si le contenu sur disque ne correspond
            // plus au binaire embarqué (corruption, extraction interrompue,
            // ou fichier remplacé entre deux lancements).
            let deja_valide = path.exists()
                && std::fs::read(&path)
                    .map(|existing| sha256_hex(&existing) == expected)
                    .unwrap_or(false);

            if !deja_valide {
                std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
                        .map_err(|e| e.to_string())?;
                }
                crate::log_info(&format!("binaries::extraire_deps | extrait {}", name));
            } else {
                crate::log_info(&format!("binaries::extraire_deps | déjà présent et vérifié {}", name));
            }

            // Vérification finale : ce qui est sur le disque doit correspondre
            // exactement à ce qui est embarqué dans le binaire, sans quoi on
            // refuse d'utiliser ce fichier.
            let on_disk = std::fs::read(&path).map_err(|e| e.to_string())?;
            if sha256_hex(&on_disk) != expected {
                return Err(format!("binaries::extraire_deps | vérification d'intégrité échouée pour {}", name));
            }
            Ok(())
        };
        f(&format!("ffmpeg{EXT}"),      embedded::FFMPEG)?;
        f(&format!("ffprobe{EXT}"),     embedded::FFPROBE)?;
        f(&format!("mkvpropedit{EXT}"), embedded::MKVPROPEDIT)?;
        TOOLS_DIR.set(Some(temp_dir)).ok();
        return Ok(());
    }
    #[allow(unreachable_code)]
    {
        crate::log_info("binaries::extraire_deps | mode système (pas de bundled), binaires depuis PATH ou /app/bin/");
        TOOLS_DIR.set(None).ok();
        Ok(())
    }
}

// ════════════════════════════════════════════════════════════════════════
//  HELPERS
// ════════════════════════════════════════════════════════════════════════
pub fn silent_cmd(program: PathBuf) -> Command {
    #[cfg_attr(not(target_os = "windows"), allow(unused_mut))]
    let mut cmd = Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

fn get_tool(name: &str) -> PathBuf {
    match TOOLS_DIR.get() {
        Some(Some(dir)) => dir.join(format!("{name}{EXT}")),
        // Mode non-bundled : cherche d'abord sur le PATH système, et ne se
        // rabat sur le chemin Flatpak que si rien n'a été trouvé.
        _ => which::which(name).unwrap_or_else(|_| PathBuf::from(format!("/app/bin/{name}"))),
    }
}

pub fn get_ffmpeg()      -> PathBuf { get_tool("ffmpeg") }
pub fn get_ffprobe()     -> PathBuf { get_tool("ffprobe") }
pub fn get_mkvpropedit() -> PathBuf { get_tool("mkvpropedit") }

pub fn cleanup() {
    if let Some(Some(dir)) = TOOLS_DIR.get() {
        let _ = std::fs::remove_dir_all(dir);
    }
}
