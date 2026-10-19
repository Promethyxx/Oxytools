use crate::app_state::{ModuleType, OxytoolsApp};
use crate::modules;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShellAction {
    Scrap,
    TagVu,
    TagNfo,
    TagPoster,
    TagFull,
    TagReset,
}

impl ShellAction {
    pub fn arg(self) -> &'static str {
        match self {
            ShellAction::Scrap => "--scrap",
            ShellAction::TagVu => "--tag-vu",
            ShellAction::TagNfo => "--tag-nfo",
            ShellAction::TagPoster => "--tag-poster",
            ShellAction::TagFull => "--tag-full",
            ShellAction::TagReset => "--tag-reset",
        }
    }

    fn from_arg(s: &str) -> Option<Self> {
        match s {
            "--scrap" => Some(ShellAction::Scrap),
            "--tag-vu" => Some(ShellAction::TagVu),
            "--tag-nfo" => Some(ShellAction::TagNfo),
            "--tag-poster" => Some(ShellAction::TagPoster),
            "--tag-full" => Some(ShellAction::TagFull),
            "--tag-reset" => Some(ShellAction::TagReset),
            _ => None,
        }
    }

    fn queue_name(self) -> &'static str {
        match self {
            ShellAction::Scrap => "scrap",
            ShellAction::TagVu => "tag_vu",
            ShellAction::TagNfo => "tag_nfo",
            ShellAction::TagPoster => "tag_poster",
            ShellAction::TagFull => "tag_full",
            ShellAction::TagReset => "tag_reset",
        }
    }
}

pub enum Launch {
    Normal,
    Exit,
    Scrap(Vec<PathBuf>),
    Tag(ShellAction, Vec<PathBuf>),
}

pub fn launch_from_args() -> Launch {
    let mut args = std::env::args_os().skip(1);
    let Some(first) = args.next() else { return Launch::Normal };
    let Some(action) = first.to_str().and_then(ShellAction::from_arg) else { return Launch::Normal };
    let paths: Vec<PathBuf> = args.map(PathBuf::from).collect();
    if paths.is_empty() {
        return Launch::Normal;
    }
    match gather(action, &paths) {
        Some(files) if !files.is_empty() => {
            if action == ShellAction::Scrap {
                Launch::Scrap(files)
            } else {
                Launch::Tag(action, files)
            }
        }
        _ => Launch::Exit,
    }
}

fn spool_dir() -> PathBuf {
    std::env::temp_dir().join("oxytools_shell")
}

pub(crate) fn gather(action: ShellAction, paths: &[PathBuf]) -> Option<Vec<PathBuf>> {
    match try_gather(action, paths) {
        Ok(result) => result,
        Err(_) => Some(expand(paths.to_vec())),
    }
}

fn try_gather(action: ShellAction, paths: &[PathBuf]) -> std::io::Result<Option<Vec<PathBuf>>> {
    let dir = spool_dir();
    std::fs::create_dir_all(&dir)?;
    let queue = dir.join(format!("{}.queue", action.queue_name()));
    let lock = dir.join(format!("{}.lock", action.queue_name()));
    let mut chunk = String::new();
    for p in paths {
        chunk.push_str(&p.to_string_lossy());
        chunk.push('\n');
    }
    let mut f = OpenOptions::new().create(true).append(true).open(&queue)?;
    f.write_all(chunk.as_bytes())?;
    drop(f);
    if !acquire(&lock) {
        return Ok(None);
    }
    let mut all: Vec<PathBuf> = Vec::new();
    loop {
        wait_quiet(&queue);
        all.extend(take(&queue, &dir));
        let _ = std::fs::remove_file(&lock);
        if !queue.exists() || !acquire(&lock) {
            break;
        }
    }
    Ok(Some(expand(all)))
}

fn acquire(lock: &Path) -> bool {
    for _ in 0..2 {
        if OpenOptions::new().write(true).create_new(true).open(lock).is_ok() {
            return true;
        }
        let stale = std::fs::metadata(lock)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| SystemTime::now().duration_since(t).ok())
            .is_some_and(|d| d > Duration::from_secs(120));
        if !stale {
            return false;
        }
        let _ = std::fs::remove_file(lock);
    }
    false
}

fn queue_len(queue: &Path) -> u64 {
    std::fs::metadata(queue).map(|m| m.len()).unwrap_or(0)
}

fn wait_quiet(queue: &Path) {
    let start = Instant::now();
    let mut last = queue_len(queue);
    let mut stable_since = Instant::now();
    while start.elapsed() < Duration::from_secs(60) {
        std::thread::sleep(Duration::from_millis(100));
        let now = queue_len(queue);
        if now != last {
            last = now;
            stable_since = Instant::now();
        } else if stable_since.elapsed() >= Duration::from_millis(1000) {
            break;
        }
    }
}

fn take(queue: &Path, dir: &Path) -> Vec<PathBuf> {
    let taken = dir.join(format!("{}.taken", std::process::id()));
    if std::fs::rename(queue, &taken).is_err() {
        return Vec::new();
    }
    let text = std::fs::read_to_string(&taken).unwrap_or_default();
    let _ = std::fs::remove_file(&taken);
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(PathBuf::from)
        .collect()
}

fn is_mkv(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("mkv"))
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(rd) = std::fs::read_dir(dir) {
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let is_extras = p
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.eq_ignore_ascii_case("Extras"));
                if !is_extras {
                    walk(&p, out);
                }
            } else if is_mkv(&p) {
                out.push(p);
            }
        }
    }
}

fn expand(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for p in paths {
        if p.is_dir() {
            walk(&p, &mut out);
        } else if is_mkv(&p) && p.exists() {
            out.push(p);
        }
    }
    out.sort();
    out.dedup();
    out
}

impl OxytoolsApp {
    pub(crate) fn open_scrap(&mut self, files: Vec<PathBuf>) {
        self.module_actif = ModuleType::Scrapper;
        self.current_stem = files
            .first()
            .and_then(|p| p.file_stem())
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        *self.status.lock().unwrap_or_else(|e| e.into_inner()) =
            self.lang.files_loaded.replace("{}", &files.len().to_string());
        self.current_files = files;
    }
}

pub fn run_tag(action: ShellAction, files: &[PathBuf], lang: &crate::lang::Lang, lang_id: &str) {
    let (mut ok, mut err) = (0usize, 0usize);
    for path in files {
        let nfo = path.with_extension("nfo");
        let result: Result<(), String> = match action {
            ShellAction::TagVu => modules::tag::marquer_vu(path, &nfo, lang_id),
            ShellAction::TagNfo => modules::tag::appliquer_tags(path, &nfo),
            ShellAction::TagPoster => modules::tag::ajouter_images_mkv(path),
            ShellAction::TagReset => modules::tag::supprimer_tous_tags(path),
            _ => {
                let r_nfo = modules::tag::appliquer_tags(path, &nfo);
                let r_poster = modules::tag::ajouter_images_mkv(path);
                r_nfo.and(r_poster)
            }
        };
        match result {
            Ok(_) => ok += 1,
            Err(e) => {
                crate::log_error(&format!("{:?}: {}", path, e));
                err += 1;
            }
        }
    }
    let msg = crate::lang::fmt2(lang.shell_tag_result, &ok.to_string(), &err.to_string());
    let _ = rfd::MessageDialog::new()
        .set_title("Oxytools")
        .set_description(msg)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}

#[cfg(windows)]
const ROOTS: [&str; 2] = [
    r"Software\Classes\SystemFileAssociations\.mkv\shell\Oxytools",
    r"Software\Classes\Directory\shell\Oxytools",
];

#[cfg(windows)]
pub fn is_installed() -> bool {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};
    RegKey::predef(HKEY_CURRENT_USER).open_subkey(ROOTS[0]).is_ok()
}

#[cfg(windows)]
pub fn install(lang: &crate::lang::Lang) -> Result<(), String> {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe = exe.to_string_lossy().trim_start_matches(r"\\?\").to_string();
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let items = [
        ("1scrap", lang.shell_scrap, ShellAction::Scrap),
        ("2tag_vu", lang.shell_tag_vu, ShellAction::TagVu),
        ("3tag_nfo", lang.shell_tag_nfo, ShellAction::TagNfo),
        ("4tag_poster", lang.shell_tag_poster, ShellAction::TagPoster),
        ("5tag_full", lang.shell_tag_full, ShellAction::TagFull),
        ("6tag_reset", lang.shell_tag_reset, ShellAction::TagReset),
    ];
    for root in ROOTS {
        let _ = hkcu.delete_subkey_all(root);
        let (key, _) = hkcu.create_subkey(root).map_err(|e| e.to_string())?;
        key.set_value("MUIVerb", &"Oxytools").map_err(|e| e.to_string())?;
        key.set_value("Icon", &format!("\"{}\",0", exe)).map_err(|e| e.to_string())?;
        key.set_value("SubCommands", &"").map_err(|e| e.to_string())?;
        key.set_value("MultiSelectModel", &"Player").map_err(|e| e.to_string())?;
        for (id, label, action) in &items {
            let (sub, _) = hkcu
                .create_subkey(format!(r"{}\shell\{}", root, id))
                .map_err(|e| e.to_string())?;
            sub.set_value("MUIVerb", label).map_err(|e| e.to_string())?;
            sub.set_value("MultiSelectModel", &"Player").map_err(|e| e.to_string())?;
            let (cmd, _) = hkcu
                .create_subkey(format!(r"{}\shell\{}\command", root, id))
                .map_err(|e| e.to_string())?;
            cmd.set_value("", &format!("\"{}\" {} \"%1\"", exe, action.arg()))
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[cfg(windows)]
pub fn uninstall() -> Result<(), String> {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    for root in ROOTS {
        match hkcu.delete_subkey_all(root) {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}
