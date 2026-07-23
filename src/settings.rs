// ═══════════════════════════════════════════════════════════════
//  OXYTOOLS — Chargement et sauvegarde de la configuration (oxytools.toml)
// ═══════════════════════════════════════════════════════════════

use crate::app_state::{OxytoolsApp, ModuleType};
use crate::modules;
use std::path::PathBuf;

impl OxytoolsApp {
    /// Chemin du dossier config/exe — toujours à côté de l'exe, sert de bootstrap.
    pub(crate) fn exe_config_dir() -> std::path::PathBuf {
        let dir = std::env::current_exe()
            .unwrap_or_default()
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .to_path_buf()
            .join("config");
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    /// Dossier config effectif : custom si défini, sinon exe_config_dir.
    pub(crate) fn config_dir(&self) -> std::path::PathBuf {
        if let Some(ref p) = self.custom_config_dir {
            let _ = std::fs::create_dir_all(p);
            p.clone()
        } else {
            Self::exe_config_dir()
        }
    }
    pub(crate) fn load_config(&mut self) {
        // Bootstrap : lire custom_config_dir depuis le oxytools.toml à côté de l'exe
        if self.custom_config_dir.is_none() {
            let bootstrap = Self::exe_config_dir().join("oxytools.toml");
            if let Ok(c) = std::fs::read_to_string(&bootstrap)
                && let Ok(parsed) = c.parse::<toml::Table>()
                    && let Some(p) = parsed.get("app").and_then(|a| a.get("config_dir")).and_then(|v| v.as_str()) {
                        let pb = std::path::PathBuf::from(p);
                        if pb.exists() || std::fs::create_dir_all(&pb).is_ok() {
                            self.custom_config_dir = Some(pb);
                        }
                    }
        }
        match self.module_actif {
            ModuleType::Image => self.format_choisi = String::new(),
            ModuleType::Doc => self.format_choisi = String::new(),
            ModuleType::Rename => {},
            #[cfg(feature = "api")]
            ModuleType::Video => self.format_choisi = String::new(),
            #[cfg(feature = "api")]
            ModuleType::Audio => self.format_choisi = String::new(),
            ModuleType::Archive => self.format_choisi = String::new(),
            _ => (),
        }
        if let Ok(c) = std::fs::read_to_string(self.config_dir().join("oxytools.toml"))
            && let Ok(parsed) = c.parse::<toml::Table>() {
                if let Some(theme) = parsed.get("display").and_then(|d| d.get("theme")).and_then(|t| t.as_str()) {
                    self.current_theme = theme.to_string();
                }
                if let Some(max_jobs) = parsed.get("performance").and_then(|p| p.get("max_parallel_jobs")).and_then(|j| j.as_integer()) {
                    self.max_parallel_jobs = max_jobs as usize;
                }
                if let Some(lang_str) = parsed.get("app").and_then(|a| a.get("lang")).and_then(|l| l.as_str()) {
                    self.lang = match lang_str { "fr" => &crate::lang::FR, _ => &crate::lang::EN };
                    self.lang_id = match lang_str { "fr" => "fr", _ => "en" };
                }
                if let Some(doc) = parsed.get("doc")
                    && let Some(fmt) = doc.get("format").and_then(|f| f.as_str())
                        && self.module_actif == ModuleType::Doc {
                            self.format_choisi = fmt.to_string();
                        }
                if let Some(img) = parsed.get("image") {
                    if let Some(fmt) = img.get("format").and_then(|f| f.as_str())
                        && self.module_actif == ModuleType::Image {
                            self.format_choisi = fmt.to_string();
                        }
                    if let Some(ratio) = img.get("ratio_img").and_then(|r| r.as_integer()) {
                        self.ratio_img = ratio as u32;
                    }
                }
                if let Some(arc) = parsed.get("archive") {
                    if let Some(fmt) = arc.get("format").and_then(|f| f.as_str())
                        && self.module_actif == ModuleType::Archive {
                            self.format_choisi = fmt.to_string();
                        }
                    if let Some(n) = arc.get("niveau").and_then(|v| v.as_integer()) {
                        self.archive_niveau = n as u32;
                    }
                    if let Some(s) = arc.get("backup_source").and_then(|v| v.as_str()) {
                        self.archive_backup_source = s.to_string();
                    }
                    if let Some(s) = arc.get("backup_dest").and_then(|v| v.as_str()) {
                        self.archive_backup_dest = s.to_string();
                    }
                    if let Some(s) = arc.get("backup_exclusions").and_then(|v| v.as_str()) {
                        self.archive_backup_exclusions = s.to_string();
                    }
                    if let Some(s) = arc.get("multi_source").and_then(|v| v.as_str()) {
                        self.archive_multi_source = s.to_string();
                    }
                }
                #[cfg(feature = "api")]
                if let Some(aud) = parsed.get("audio") {
                    if let Some(fmt) = aud.get("format").and_then(|f| f.as_str())
                        && self.module_actif == ModuleType::Audio {
                            self.format_choisi = fmt.to_string();
                        }
                    if let Some(q) = aud.get("qualite").and_then(|v| v.as_integer()) {
                        self.audio_qualite = q as u32;
                    }
                }
                #[cfg(feature = "api")]
                if let Some(vid) = parsed.get("video") {
                    if let Some(fmt) = vid.get("format").and_then(|f| f.as_str())
                        && self.module_actif == ModuleType::Video {
                            self.format_choisi = fmt.to_string();
                        }
                    if let Some(copie) = vid.get("copie_flux").and_then(|c| c.as_bool()) {
                        self.copie_flux = copie;
                    }
                    if let Some(speed) = vid.get("speed").and_then(|s| s.as_integer()) {
                        self.video_speed = speed as u32;
                    }
                }
            }
        // ── Charger le dernier profil multi-replace ──────────────
        if let Ok(c) = std::fs::read_to_string(self.config_dir().join("oxytools.toml"))
            && let Ok(parsed) = c.parse::<toml::Table>()
                && let Some(rn) = parsed.get("rename")
                    && let Some(p) = rn.get("last_list_path").and_then(|v| v.as_str()) {
                        let path = std::path::PathBuf::from(p);
                        if path.exists()
                            && let Ok(list) = modules::rename::ReplaceList::load(&path) {
                                self.rename_cfg.replace_list = list;
                                self.rename_cfg.multi_replace = true;
                                self.rename_last_list_path = Some(path);
                            }
                    }
        // Charger le path custom de .env si enregistré dans oxytools.toml
        let custom_env_path: Option<PathBuf> = {
            std::fs::read_to_string(self.config_dir().join("oxytools.toml"))
                .ok()
                .and_then(|c| c.parse::<toml::Table>().ok())
                .and_then(|t| t.get("scrapper")?.as_table()?.get("keys_path")?.as_str().map(PathBuf::from))
        };
        let env_path = custom_env_path.clone().unwrap_or_else(|| self.config_dir().join(".env"));
        if let Some(ref p) = custom_env_path { self.keys_env_path = Some(p.clone()); }
        dotenvy::from_path(&env_path).ok();
        if let Ok(k) = std::env::var("TMDB_API_KEY") { self.tmdb_api_key = k; }
        if let Ok(k) = std::env::var("FANART_API_KEY") { self.fanart_api_key = k; }
    }
    pub(crate) fn save_config(&self) {
        let mut parsed = if let Ok(c) = std::fs::read_to_string(self.config_dir().join("oxytools.toml")) {
            c.parse::<toml::Table>().unwrap_or_else(|_| toml::Table::new())
        } else {
            toml::Table::new()
        };
        let display = parsed.entry("display").or_insert(toml::Value::Table(toml::Table::new()));
        if let Some(display_table) = display.as_table_mut() {
            display_table.insert("theme".to_string(), toml::Value::String(self.current_theme.clone()));
        }
        let perf = parsed.entry("performance").or_insert(toml::Value::Table(toml::Table::new()));
        if let Some(perf_table) = perf.as_table_mut() {
            perf_table.insert("max_parallel_jobs".to_string(), toml::Value::Integer(self.max_parallel_jobs as i64));
        }
        let app = parsed.entry("app").or_insert(toml::Value::Table(toml::Table::new()));
        if let Some(app_table) = app.as_table_mut() {
            app_table.insert("lang".to_string(), toml::Value::String(self.lang_id.to_string()));
        }
        // Persister le chemin custom du fichier .env de clés
        let scrapper = parsed.entry("scrapper").or_insert(toml::Value::Table(toml::Table::new()));
        if let Some(scrap_table) = scrapper.as_table_mut() {
            if let Some(ref p) = self.keys_env_path {
                scrap_table.insert("keys_path".to_string(), toml::Value::String(p.to_string_lossy().into_owned()));
            } else {
                scrap_table.remove("keys_path");
            }
        }
        if self.save_doc_format && !self.format_choisi.is_empty() && self.module_actif == ModuleType::Doc {
            let doc = parsed.entry("doc").or_insert(toml::Value::Table(toml::Table::new()));
            if let Some(doc_table) = doc.as_table_mut() {
                doc_table.insert("format".to_string(), toml::Value::String(self.format_choisi.clone()));
            }
        } else if !self.save_doc_format && self.module_actif == ModuleType::Doc
            && let Some(doc_table) = parsed.get_mut("doc").and_then(|v| v.as_table_mut()) {
                doc_table.remove("format");
            }
        if self.save_image_format && !self.format_choisi.is_empty() && self.module_actif == ModuleType::Image {
            let image = parsed.entry("image").or_insert(toml::Value::Table(toml::Table::new()));
            if let Some(img_table) = image.as_table_mut() {
                img_table.insert("format".to_string(), toml::Value::String(self.format_choisi.clone()));
                img_table.insert("ratio_img".to_string(), toml::Value::Integer(self.ratio_img as i64));
            }
        } else if !self.save_image_format && self.module_actif == ModuleType::Image
            && let Some(img_table) = parsed.get_mut("image").and_then(|v| v.as_table_mut()) {
                img_table.remove("format");
            }
        if self.module_actif == ModuleType::Image {
            let image = parsed.entry("image").or_insert(toml::Value::Table(toml::Table::new()));
            if let Some(img_table) = image.as_table_mut() {
                img_table.insert("ratio_img".to_string(), toml::Value::Integer(self.ratio_img as i64));
            }
        }
        if self.save_archive_format && !self.format_choisi.is_empty() && self.module_actif == ModuleType::Archive {
                let archive = parsed.entry("archive").or_insert(toml::Value::Table(toml::Table::new()));
                if let Some(arc_table) = archive.as_table_mut() {
                    arc_table.insert("format".to_string(), toml::Value::String(self.format_choisi.clone()));
                }
            } else if !self.save_archive_format && self.module_actif == ModuleType::Archive
                && let Some(arc_table) = parsed.get_mut("archive").and_then(|v| v.as_table_mut()) {
                    arc_table.remove("format");
                }
            if self.module_actif == ModuleType::Archive {
                let archive = parsed.entry("archive").or_insert(toml::Value::Table(toml::Table::new()));
                if let Some(arc_table) = archive.as_table_mut() {
                    arc_table.insert("niveau".to_string(), toml::Value::Integer(self.archive_niveau as i64));
                    arc_table.insert("backup_source".to_string(), toml::Value::String(self.archive_backup_source.clone()));
                    arc_table.insert("backup_dest".to_string(), toml::Value::String(self.archive_backup_dest.clone()));
                    arc_table.insert("backup_exclusions".to_string(), toml::Value::String(self.archive_backup_exclusions.clone()));
                    arc_table.insert("multi_source".to_string(), toml::Value::String(self.archive_multi_source.clone()));
                }
            }
        #[cfg(feature = "api")]
        {
            if self.save_audio_format && !self.format_choisi.is_empty() && self.module_actif == ModuleType::Audio {
                let audio = parsed.entry("audio").or_insert(toml::Value::Table(toml::Table::new()));
                if let Some(aud_table) = audio.as_table_mut() {
                    aud_table.insert("format".to_string(), toml::Value::String(self.format_choisi.clone()));
                }
            } else if !self.save_audio_format && self.module_actif == ModuleType::Audio
                && let Some(aud_table) = parsed.get_mut("audio").and_then(|v| v.as_table_mut()) {
                    aud_table.remove("format");
                }
            if self.module_actif == ModuleType::Audio {
                let audio = parsed.entry("audio").or_insert(toml::Value::Table(toml::Table::new()));
                if let Some(aud_table) = audio.as_table_mut() {
                    aud_table.insert("qualite".to_string(), toml::Value::Integer(self.audio_qualite as i64));
                }
            }
            if self.save_video_format && !self.format_choisi.is_empty() && self.module_actif == ModuleType::Video {
                let video = parsed.entry("video").or_insert(toml::Value::Table(toml::Table::new()));
                if let Some(vid_table) = video.as_table_mut() {
                    vid_table.insert("format".to_string(), toml::Value::String(self.format_choisi.clone()));
                }
            } else if !self.save_video_format && self.module_actif == ModuleType::Video
                && let Some(vid_table) = parsed.get_mut("video").and_then(|v| v.as_table_mut()) {
                    vid_table.remove("format");
                }
            if self.module_actif == ModuleType::Video {
                let video = parsed.entry("video").or_insert(toml::Value::Table(toml::Table::new()));
                if let Some(vid_table) = video.as_table_mut() {
                    vid_table.insert("copie_flux".to_string(), toml::Value::Boolean(self.copie_flux));
                    vid_table.insert("speed".to_string(), toml::Value::Integer(self.video_speed as i64));
                }
            }
        }
        // ── Persister le dernier profil multi-replace ────────────
        {
            let rename_table = parsed.entry("rename").or_insert(toml::Value::Table(toml::Table::new()));
            if let Some(rt) = rename_table.as_table_mut() {
                if let Some(ref p) = self.rename_last_list_path {
                    rt.insert("last_list_path".to_string(), toml::Value::String(p.to_string_lossy().to_string()));
                } else {
                    rt.remove("last_list_path");
                }
            }
        }
        let toml_str = toml::to_string(&parsed).unwrap_or_default();
        // Sauver dans le config_dir effectif
        let _ = std::fs::write(self.config_dir().join("oxytools.toml"), &toml_str);
        // Sauver aussi dans le bootstrap exe pour que custom_config_dir survive au redémarrage
        if self.custom_config_dir.is_some() {
            let bootstrap_dir = Self::exe_config_dir();
            let mut bootstrap_parsed = std::fs::read_to_string(bootstrap_dir.join("oxytools.toml"))
                .ok()
                .and_then(|c| c.parse::<toml::Table>().ok())
                .unwrap_or_default();
            let app = bootstrap_parsed.entry("app").or_insert(toml::Value::Table(toml::Table::new()));
            if let Some(t) = app.as_table_mut() {
                if let Some(ref p) = self.custom_config_dir {
                    t.insert("config_dir".to_string(), toml::Value::String(p.to_string_lossy().into_owned()));
                } else {
                    t.remove("config_dir");
                }
            }
            let _ = std::fs::write(bootstrap_dir.join("oxytools.toml"), toml::to_string(&bootstrap_parsed).unwrap_or_default());
        }
    }
}
