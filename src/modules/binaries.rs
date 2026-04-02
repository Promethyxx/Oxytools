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
//  EXTRACTION
// ════════════════════════════════════════════════════════════════════════
pub fn extraire_deps() -> Result<(), String> {
    #[cfg(feature = "bundled")]
    {
        let temp_dir = std::env::temp_dir().join("oxyon_tools");
        crate::log_info(&format!("binaries::extraire_deps | dossier temp={:?}", temp_dir));
        if !temp_dir.exists() {
            std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
        }
        let f = |name: &str, bytes: &[u8]| -> Result<(), String> {
            let path = temp_dir.join(name);
            if !path.exists() {
                std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
                    .map_err(|e| e.to_string())?;
                }
                crate::log_info(&format!("binaries::extraire_deps | extrait {}", name));
            } else {
                crate::log_info(&format!("binaries::extraire_deps | déjà présent {}", name));
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
        _ => PathBuf::from(format!("/app/bin/{name}")),
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