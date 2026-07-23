// ═══════════════════════════════════════════════════════════════
//  OXYTOOLS — Orchestration des jobs de conversion (logique métier)
// ═══════════════════════════════════════════════════════════════

use crate::app_state::{OxytoolsApp, ModuleType};
use crate::{modules, utils, log_info, log_warn, log_error};
use eframe::egui;
use std::path::{Path, PathBuf};
use std::sync::Arc;

impl OxytoolsApp {
    pub(crate) fn verifier_deps(&mut self) {
        self.deps_manquantes = Vec::new();
    }
    pub(crate) fn lancer_batch(&mut self, ctx: egui::Context) {
        log_info(&format!(
            "=== BATCH START | {} fichier(s) | {} workers max | module={:?} | action={} ===",
            self.current_files.len(),
            self.max_parallel_jobs,
            self.module_actif,
            match self.module_actif {
                ModuleType::Doc   => self.doc_action.as_str(),
                ModuleType::Image => self.image_action.as_str(),
                _                 => "-",
            }
        ));
        for f in &self.current_files {
            log_info(&format!("  Fichier en queue: {:?}", f));
        }

        *self.completed_jobs.lock().unwrap_or_else(|e| e.into_inner()) = 0;
        *self.total_jobs.lock().unwrap_or_else(|e| e.into_inner()) = self.current_files.len();
        *self.active_jobs.lock().unwrap_or_else(|e| e.into_inner()) = 0;
        let mut queue = self.job_queue.lock().unwrap_or_else(|e| e.into_inner());
        queue.clear();
        queue.extend(self.current_files.clone());
        drop(queue);
        *self.status.lock().unwrap_or_else(|e| e.into_inner()) = self.lang.starting_tasks.replace("{}", &self.current_files.len().to_string());
        for _ in 0..self.max_parallel_jobs.min(self.current_files.len()) {
            self.spawn_worker(ctx.clone());
        }
    }
    pub(crate) fn spawn_worker(&mut self, ctx: egui::Context) {
        let queue = Arc::clone(&self.job_queue);
        let active = Arc::clone(&self.active_jobs);
        let completed = Arc::clone(&self.completed_jobs);
        let total = Arc::clone(&self.total_jobs);
        let status_arc = Arc::clone(&self.status);
        let conv_progress = Arc::clone(&self.conv_progress);
        let active_pids = Arc::clone(&self.active_pids);
        let lang = self.lang;
        let module = self.module_actif;
        let fmt = self.format_choisi.clone();
        let ratio = self.ratio_img;
        #[cfg(feature = "api")]
        let copie = self.copie_flux;
        #[cfg(feature = "api")]
        let video_speed = self.video_speed;
        #[cfg(feature = "api")]
        let audio_action = self.audio_action.clone();
        #[cfg(feature = "api")]
        let audio_qualite = self.audio_qualite;
        let archive_niveau = self.archive_niveau;
        let archive_action = self.archive_action.clone();
        let img_action = self.image_action.clone();
        let jxl_mode = self.jxl_mode.clone();
        let angle = self.rotation_angle;
        let crop_x = self.crop_x;
        let crop_y = self.crop_y;
        let crop_w = self.crop_width;
        let crop_h = self.crop_height;
        let resize_w = self.resize_width.parse::<u32>().unwrap_or(0);
        let resize_h = self.resize_height.parse::<u32>().unwrap_or(0);
        let resize_kb = self.resize_max_kb.parse::<u32>().unwrap_or(0);
        let doc_action = self.doc_action.clone();
        let pdf_angle = self.pdf_rotation_angle;
        let pdf_pages = self.pdf_pages_spec.clone();
        let pdf_merge_list = self.current_files.clone();
        let pdf_crop_x = self.pdf_crop_x;
        let pdf_crop_y = self.pdf_crop_y;
        let pdf_crop_w = self.pdf_crop_w;
        let pdf_crop_h = self.pdf_crop_h;
        let pdf_num_debut = self.pdf_num_debut;
        let pdf_num_position = self.pdf_num_position.clone();
        let pdf_num_taille = self.pdf_num_taille;
        let pdf_owner_pass = self.pdf_owner_pass.clone();
        let pdf_user_pass = self.pdf_user_pass.clone();
        let pdf_allow_print = self.pdf_allow_print;
        let pdf_allow_copy = self.pdf_allow_copy;
        let pdf_unlock_pass = self.pdf_unlock_pass.clone();
        let pdf_wm_texte = self.pdf_wm_texte.clone();
        let pdf_wm_taille = self.pdf_wm_taille;
        let pdf_wm_opacite = self.pdf_wm_opacite;
        let pdf_nouvel_ordre = self.pdf_nouvel_ordre.clone();
        let pdf_annot_texte = self.pdf_annot_texte.clone();
        let pdf_annot_x = self.pdf_annot_x;
        let pdf_annot_y = self.pdf_annot_y;
        let pdf_annot_w = self.pdf_annot_w;
        let pdf_annot_h = self.pdf_annot_h;
        let pdf_sign_nom = self.pdf_sign_nom.clone();
        let pdf_sign_position = self.pdf_sign_position.clone();
        let pdf_sign_taille = self.pdf_sign_taille;
        let img_wm_texte = self.img_wm_texte.clone();
        let img_wm_taille = self.img_wm_taille;
        let img_wm_opacite = self.img_wm_opacite;
        let img_meme_top = self.img_meme_top.clone();
        let img_meme_bottom = self.img_meme_bottom.clone();
        let img_upscale_factor = self.img_upscale_factor;
        let ico_sizes: Vec<u32> = {
            let mut s = Vec::new();
            if self.ico_size_16 { s.push(16); }
            if self.ico_size_32 { s.push(32); }
            if self.ico_size_64 { s.push(64); }
            if self.ico_size_128 { s.push(128); }
            if self.ico_size_256 { s.push(256); }
            if self.ico_size_512 { s.push(512); }
            if self.ico_size_custom
                && let Ok(w) = self.ico_custom_w.parse::<u32>()
                    && w > 0 { s.push(w); }
            if s.is_empty() { s.push(256); }
            s
        };
        let convert_resize_w = self.resize_width.parse::<u32>().unwrap_or(0);
        let convert_resize_h = self.resize_height.parse::<u32>().unwrap_or(0);
        std::thread::spawn(move || {
            loop {
                let job = {
                    let mut q = queue.lock().unwrap_or_else(|e| e.into_inner());
                    q.pop()
                };
                let input = match job {
                    Some(path) => path,
                    None => break,
                };
                *active.lock().unwrap_or_else(|e| e.into_inner()) += 1;
                let effective_fmt = if module == ModuleType::Doc && doc_action != "Convert" {
                    "pdf".to_string()
                } else {
                    fmt.to_lowercase()
                };
                let output = match input.parent() {
                    Some(parent) => parent.join(format!(
                        "{}_oxytools.{}",
                        input.file_stem().unwrap_or_default().to_string_lossy(),
                        effective_fmt
                    )),
                    None => {
                        log_error(&format!("FAILED | file={:?} | reason=fichier sans dossier parent", input));
                        *active.lock().unwrap_or_else(|e| e.into_inner()) -= 1;
                        *completed.lock().unwrap_or_else(|e| e.into_inner()) += 1;
                        let done = *completed.lock().unwrap_or_else(|e| e.into_inner());
                        let total_count = *total.lock().unwrap_or_else(|e| e.into_inner());
                        *status_arc.lock().unwrap_or_else(|e| e.into_inner()) = crate::lang::fmt2(lang.processing_files, &done.to_string(), &total_count.to_string());
                        ctx.request_repaint();
                        continue;
                    }
                };

                // ── Timing ──────────────────────────────────────────────
                let start = std::time::Instant::now();
                log_info(&format!(
                    "START | module={:?} | fichier={:?} | sortie={:?}",
                    module, input, output
                ));

                let current = *completed.lock().unwrap_or_else(|e| e.into_inner()) + *active.lock().unwrap_or_else(|e| e.into_inner());
                let total_count = *total.lock().unwrap_or_else(|e| e.into_inner());
                *status_arc.lock().unwrap_or_else(|e| e.into_inner()) = crate::lang::fmt2(lang.processing_files, &current.to_string(), &total_count.to_string());
                ctx.request_repaint();

                // ── Exécution avec résultat détaillé ────────────────────
                let result: Result<(), String> = match module {
                    ModuleType::Archive => {
                        match archive_action.as_str() {
                            "extract" => {
                                match input.parent() {
                                    None => Err(format!("extract: fichier sans dossier parent | {:?}", input)),
                                    Some(parent) => {
                                        let dest = parent.join(
                                            input.file_stem().unwrap_or_default().to_string_lossy().to_string()
                                        );
                                        log_info(&format!("Archive: extraction | {:?} -> {:?}", input, dest));
                                        if modules::archive::extraire(&input, &dest) {
                                            Ok(())
                                        } else {
                                            Err(format!("extraire() failed | file={:?}", input))
                                        }
                                    }
                                }
                            },
                            "convert" => {
                                log_info(&format!("Archive: convert fmt={} | {:?}", fmt, input));
                                if modules::archive::convertir(&input, &fmt) {
                                    Ok(())
                                } else {
                                    Err(format!("convertir() failed | fmt={} | file={:?}", fmt, input))
                                }
                            },
                            "multi" => {
                                // input = un sous-dossier, output = dossier.{fmt} à côté
                                match input.parent() {
                                    None => Err(format!("multi: fichier sans dossier parent | {:?}", input)),
                                    Some(parent) => {
                                        let name = input.file_name().unwrap_or_default().to_string_lossy().to_string();
                                        let multi_out = parent.join(format!("{}.{}", name, fmt));
                                        log_info(&format!("Archive multi: {} -> {:?}", name, multi_out));
                                        if modules::archive::compresser(&input, &multi_out, &fmt, archive_niveau) {
                                            Ok(())
                                        } else {
                                            Err(format!("multi compresser() failed | {} | fmt={}", name, fmt))
                                        }
                                    }
                                }
                            },
                            _ => {
                                log_info(&format!("Archive: compression fmt={} niveau={} | {:?}", fmt, archive_niveau, input));
                                if modules::archive::compresser(&input, &output, &fmt, archive_niveau) {
                                    Ok(())
                                } else {
                                    Err(format!("compresser() returned false | fmt={} | file={:?}", fmt, input))
                                }
                            },
                        }
                    },
                    #[cfg(feature = "api")]
                    ModuleType::Audio => {
                        match audio_action.as_str() {
                            "extract" => {
                                log_info(&format!("Audio: extraction | {:?}", input));
                                match input.parent() {
                                    None => Err(format!("Audio extract: fichier sans dossier parent | {:?}", input)),
                                    Some(parent) => {
                                        let ext = modules::audio::detecter_extension(&input);
                                        let extract_out = parent.join(format!(
                                            "{}_oxytools.{}",
                                            input.file_stem().unwrap_or_default().to_string_lossy(),
                                            if ext.is_empty() { "mka".to_string() } else { ext }
                                        ));
                                        match modules::audio::extraire(&input, &extract_out) {
                                            Ok(child) => {
                                                let dur = utils::get_duration_secs(&input);
                                                utils::wait_ffmpeg_with_progress(child, dur, &conv_progress, &active_pids)
                                            },
                                            Err(e) => Err(format!("impossible de lancer ffmpeg extraction: {}", e)),
                                        }
                                    }
                                }
                            },
                            _ => {
                                log_info(&format!("Audio: conversion | {:?}", input));
                                match modules::audio::convertir(&input, &output, audio_qualite) {
                                    Ok(child) => {
                                        let dur = utils::get_duration_secs(&input);
                                        utils::wait_ffmpeg_with_progress(child, dur, &conv_progress, &active_pids)
                                    },
                                    Err(e) => Err(format!("impossible de lancer ffmpeg audio: {}", e)),
                                }
                            },
                        }
                    },
                    #[cfg(feature = "api")]
                    ModuleType::Video => {
                        log_info(&format!("Video: copie_flux={} speed={} | {:?}", copie, video_speed, input));
                        match modules::video::traiter_video(&input, &output, copie, false, video_speed) {
                            Ok(child) => {
                                let dur = utils::get_duration_secs(&input);
                                utils::wait_ffmpeg_with_progress(child, dur, &conv_progress, &active_pids)
                            },
                            Err(e) => Err(format!("failed to start ffmpeg video: {}", e)),
                        }
                    },
                    ModuleType::Doc => {
                        log_info(&format!("Doc: action={} | {:?}", doc_action, input));
                        match doc_action.as_str() {
                            "Convert" => {
                                let format_entree = modules::doc::detecter_format_entree(&input);
                                let format_sortie = modules::doc::detecter_format_sortie(&output);
                                log_info(&format!("Doc Convert: entree={:?} sortie={:?}", format_entree, format_sortie));
                                if modules::doc::convertir_avec_formats(&input, &output, format_entree, format_sortie) {
                                    Ok(())
                                } else {
                                    Err(format!("Convert_avec_formats failed | input={:?} output={:?} | file={:?}", format_entree, format_sortie, input))
                                }
                            },
                            "pdf_split" => {
                                match input.parent() {
                                    None => Err(format!("pdf_split: fichier sans dossier parent | {:?}", input)),
                                    Some(parent) => {
                                        let output_dir = parent.join(format!(
                                            "{}_pages",
                                            input.file_stem().unwrap_or_default().to_string_lossy()
                                        ));
                                        std::fs::create_dir_all(&output_dir).ok();
                                        log_info(&format!("Doc pdf_split: output_dir={:?}", output_dir));
                                        modules::doc::pdf_split(&input, &output_dir)
                                            .map(|_| ())
                                            .map_err(|e| format!("pdf_split failed: {}", e))
                                    }
                                }
                            },
                            "pdf_merge" => {
                                match input.parent() {
                                    None => Err(format!("pdf_merge: fichier sans dossier parent | {:?}", input)),
                                    Some(parent) => {
                                        let paths: Vec<&Path> = pdf_merge_list.iter().map(|p| p.as_path()).collect();
                                        let output_merge = parent.join("merged_oxytools.pdf");
                                        log_info(&format!("Doc pdf_merge: {} fichiers -> {:?}", paths.len(), output_merge));
                                        modules::doc::pdf_merge(&paths, &output_merge)
                                            .map_err(|e| format!("pdf_merge failed: {}", e))
                                    }
                                }
                            },
                            "pdf_rotate" => {
                                let pages_opt = utils::parse_pages_spec(&pdf_pages);
                                log_info(&format!("Doc pdf_rotate: angle={} pages={:?}", pdf_angle, pages_opt));
                                modules::doc::pdf_rotate(&input, &output, pdf_angle, pages_opt.as_deref())
                                    .map_err(|e| format!("pdf_rotate failed: {}", e))
                            },
                            "pdf_compress" => {
                                log_info(&format!("Doc pdf_compress: {:?}", input));
                                modules::doc::pdf_compresser(&input, &output)
                                    .map(|_| ())
                                    .map_err(|e| format!("pdf_compress failed: {}", e))
                            },
                            "pdf_crop" => {
                                let pages_opt = utils::parse_pages_spec(&pdf_pages);
                                log_info(&format!("Doc pdf_crop: x={} y={} w={} h={} pages={:?}", pdf_crop_x, pdf_crop_y, pdf_crop_w, pdf_crop_h, pages_opt));
                                modules::doc::pdf_crop(&input, &output, pdf_crop_x, pdf_crop_y, pdf_crop_w, pdf_crop_h, pages_opt.as_deref())
                                    .map_err(|e| format!("pdf_crop failed: {}", e))
                            },
                            "pdf_organize" => {
                                let ordre: Vec<u32> = pdf_nouvel_ordre.split(',')
                                    .filter_map(|s| s.trim().parse::<u32>().ok())
                                    .collect();
                                log_info(&format!("Doc pdf_organize: ordre={:?}", ordre));
                                if ordre.is_empty() {
                                    Err("pdf_organize: ordre vide ou invalide".to_string())
                                } else {
                                    modules::doc::pdf_organiser(&input, &output, &ordre)
                                        .map_err(|e| format!("pdf_organize failed: {}", e))
                                }
                            },
                            "pdf_delete_pages" => {
                                let pages_a_sup: Vec<u32> = pdf_pages.split(',')
                                    .filter_map(|s| s.trim().parse::<u32>().ok())
                                    .collect();
                                log_info(&format!("Doc pdf_delete_pages: pages={:?}", pages_a_sup));
                                if pages_a_sup.is_empty() {
                                    Err("pdf_delete_pages: liste de pages vide ou invalide".to_string())
                                } else {
                                    modules::doc::pdf_supprimer_pages(&input, &output, &pages_a_sup)
                                        .map_err(|e| format!("pdf_delete_pages failed: {}", e))
                                }
                            },
                            "pdf_numbers" => {
                                let position = match pdf_num_position.as_str() {
                                    "BasGauche"  => modules::doc::PositionNumero::BasGauche,
                                    "BasDroite"  => modules::doc::PositionNumero::BasDroite,
                                    "HautCentre" => modules::doc::PositionNumero::HautCentre,
                                    "HautGauche" => modules::doc::PositionNumero::HautGauche,
                                    "HautDroite" => modules::doc::PositionNumero::HautDroite,
                                    _            => modules::doc::PositionNumero::BasCentre,
                                };
                                log_info(&format!("Doc pdf_numbers: debut={} position={} taille={}", pdf_num_debut, pdf_num_position, pdf_num_taille));
                                modules::doc::pdf_numeroter(&input, &output, pdf_num_debut, position, pdf_num_taille)
                                    .map_err(|e| format!("pdf_number_pages failed: {}", e))
                            },
                            "pdf_protect" => {
                                log_info(&format!("Doc pdf_protect: print={} copy={}", pdf_allow_print, pdf_allow_copy));
                                modules::doc::pdf_proteger(&input, &output, &pdf_owner_pass, &pdf_user_pass, pdf_allow_print, pdf_allow_copy)
                                    .map_err(|e| format!("pdf_protect failed: {}", e))
                            },
                            "pdf_unlock" => {
                                log_info("Doc pdf_unlock");
                                modules::doc::pdf_dechiffrer(&input, &output, &pdf_unlock_pass)
                                    .map_err(|e| format!("pdf_unlock failed: {}", e))
                            },
                            "pdf_repair" => {
                                log_info(&format!("Doc pdf_repair: {:?}", input));
                                modules::doc::pdf_reparer(&input, &output)
                                    .map_err(|e| format!("pdf_repair failed: {}", e))
                            },
                            "pdf_watermark" => {
                                let pages_opt = utils::parse_pages_spec(&pdf_pages);
                                log_info(&format!("Doc pdf_watermark: texte='{}' taille={} opacite={}", pdf_wm_texte, pdf_wm_taille, pdf_wm_opacite));
                                modules::doc::pdf_watermark(&input, &output, &pdf_wm_texte, pdf_wm_taille, pdf_wm_opacite, pages_opt.as_deref())
                                    .map_err(|e| format!("pdf_watermark failed: {}", e))
                            },
                            "pdf_annotate" => {
                                let pages_opt = utils::parse_pages_spec(&pdf_pages);
                                log_info(&format!("Doc pdf_annotate: texte='{}' x={} y={} w={} h={}", pdf_annot_texte, pdf_annot_x, pdf_annot_y, pdf_annot_w, pdf_annot_h));
                                modules::doc::pdf_annoter(&input, &output, &pdf_annot_texte, modules::doc::RectPct { x: pdf_annot_x, y: pdf_annot_y, largeur: pdf_annot_w, hauteur: pdf_annot_h }, pages_opt.as_deref())
                                    .map_err(|e| format!("pdf_annotate failed: {}", e))
                            },
                            "pdf_sign" => {
                                let pages_opt = utils::parse_pages_spec(&pdf_pages);
                                let sign_pos = match pdf_sign_position.as_str() {
                                    "BasGauche"  => modules::doc::PositionNumero::BasGauche,
                                    "BasDroite"  => modules::doc::PositionNumero::BasDroite,
                                    "HautCentre" => modules::doc::PositionNumero::HautCentre,
                                    "HautGauche" => modules::doc::PositionNumero::HautGauche,
                                    "HautDroite" => modules::doc::PositionNumero::HautDroite,
                                    _            => modules::doc::PositionNumero::BasDroite,
                                };
                                log_info(&format!("Doc pdf_sign: nom='{}' position={} taille={}", pdf_sign_nom, pdf_sign_position, pdf_sign_taille));
                                modules::doc::pdf_signer(&input, &output, &pdf_sign_nom, sign_pos, pdf_sign_taille, pages_opt.as_deref())
                                    .map_err(|e| format!("pdf_sign failed: {}", e))
                            },
                            autre => {
                                log_warn(&format!("Doc: action inconnue '{}', fallback Convert()", autre));
                                if modules::doc::convertir(&input, &output) {
                                    Ok(())
                                } else {
                                    Err(format!("Convert() fallback failed for {:?}", input))
                                }
                            },
                        }
                    },
                    ModuleType::Image => {
                        log_info(&format!("Image: action={} fmt={} ratio={} | {:?}", img_action, fmt, ratio, input));
                        match img_action.as_str() {
                            "Convert" => {
                                // Si format JXL, dispatcher selon jxl_mode
                                if fmt.to_uppercase() == "JXL" {
                                    match jxl_mode.as_str() {
                                        "folder" => modules::pic::convertir_jxl_dossier(&input),
                                        "pivot" => modules::pic::convertir_jxl_pivot(&input),
                                        _ => modules::pic::convertir_jxl_lossless(&input),
                                    }
                                } else if fmt.to_uppercase() == "ICO" {
                                    // ICO : un fichier par taille
                                    log_info(&format!("Image ICO: sizes={:?}", ico_sizes));
                                    match input.parent() {
                                        None => Err(format!("ICO: fichier sans dossier parent | {:?}", input)),
                                        Some(parent) => {
                                            let stem = input.file_stem().unwrap_or_default().to_string_lossy();
                                            let mut all_ok = true;
                                            for &sz in &ico_sizes {
                                                let ico_out = parent.join(format!("{}_{sz}x{sz}.ico", stem));
                                                log_info(&format!("ICO entry: {}x{} -> {:?}", sz, sz, ico_out));
                                                if !modules::pic::generer_ico_multi(&input, &ico_out, &[sz]) {
                                                    log_error(&format!("pic::generer_ico_multi failed | {}x{} | {:?}", sz, sz, input));
                                                    all_ok = false;
                                                }
                                            }
                                            if all_ok { Ok(()) }
                                            else { Err(format!("ICO: some sizes failed | {:?}", input)) }
                                        }
                                    }
                                } else if convert_resize_w > 0 && convert_resize_h > 0 {
                                    // Resize before converting
                                    log_info(&format!("Image Convert+resize: {}x{} fmt={}", convert_resize_w, convert_resize_h, fmt));
                                    let mut temp_name = output.as_os_str().to_os_string();
                                    temp_name.push("_temp_cvt.png");
                                    let temp = PathBuf::from(temp_name);
                                    if modules::pic::redimensionner_pixels(&input, &temp, convert_resize_w, convert_resize_h) {
                                        let result = if modules::pic::compresser(&temp, &output, ratio) { Ok(()) }
                                        else { Err(format!("pic::compresser after resize failed | {:?}", input)) };
                                        let _ = std::fs::remove_file(&temp);
                                        result
                                    } else {
                                        Err(format!("pic::resize for convert failed | {}x{} | {:?}", convert_resize_w, convert_resize_h, input))
                                    }
                                } else {
                                    if modules::pic::compresser(&input, &output, ratio) { Ok(()) }
                                    else { Err(format!("pic::compresser failed | fmt={} ratio={} | {:?}", fmt, ratio, input)) }
                                }
                            },
                            "resize" => {
                                log_info(&format!("Image resize: w={} h={} kb={}", resize_w, resize_h, resize_kb));
                                if resize_w > 0 && resize_h > 0 {
                                    if resize_kb > 0 {
                                        let mut temp_name = output.as_os_str().to_os_string();
                                        temp_name.push(format!("_temp.{}", fmt));
                                        let temp = PathBuf::from(temp_name);
                                        if modules::pic::redimensionner_pixels(&input, &temp, resize_w, resize_h) {
                                            if modules::pic::redimensionner_poids(&temp, &output, resize_kb) {
                                                Ok(())
                                            } else {
                                                Err(format!("resize by size failed | max_kb={} | file={:?}", resize_kb, input))
                                            }
                                        } else {
                                            Err(format!("resize by pixels failed | w={} h={} | file={:?}", resize_w, resize_h, input))
                                        }
                                    } else {
                                        if modules::pic::redimensionner_pixels(&input, &output, resize_w, resize_h) { Ok(()) }
                                        else { Err(format!("resize by pixels failed | w={} h={} | file={:?}", resize_w, resize_h, input)) }
                                    }
                                } else if resize_kb > 0 {
                                    if modules::pic::redimensionner_poids(&input, &output, resize_kb) { Ok(()) }
                                    else { Err(format!("resize by size only failed | max_kb={} | file={:?}", resize_kb, input)) }
                                } else {
                                    log_warn("Image resize: no w/h or kb specified, fallback to compress");
                                    if modules::pic::compresser(&input, &output, 1) { Ok(()) }
                                    else { Err(format!("pic::compresser fallback failed for {:?}", input)) }
                                }
                            },
                            "rotate" => {
                                log_info(&format!("Image rotate: angle={}", angle));
                                if modules::pic::pivoter(&input, &output, angle) { Ok(()) }
                                else { Err(format!("pic::rotate failed | angle={} | file={:?}", angle, input)) }
                            },
                            "crop" => {
                                log_info(&format!("Image crop: x={} y={} w={} h={}", crop_x, crop_y, crop_w, crop_h));
                                if modules::pic::recadrer(&input, &output, crop_x, crop_y, crop_w, crop_h) { Ok(()) }
                                else { Err(format!("pic::crop failed | x={} y={} w={} h={} | file={:?}", crop_x, crop_y, crop_w, crop_h, input)) }
                            },
                            "watermark" => {
                                log_info(&format!("Image watermark: texte='{}' taille={} opacite={}", img_wm_texte, img_wm_taille, img_wm_opacite));
                                if modules::pic::watermark(&input, &output, &img_wm_texte, img_wm_taille, img_wm_opacite) { Ok(()) }
                                else { Err(format!("pic::watermark failed for {:?}", input)) }
                            },
                            "meme" => {
                                log_info(&format!("Image meme: top='{}' bottom='{}'", img_meme_top, img_meme_bottom));
                                if modules::pic::meme(&input, &output, &img_meme_top, &img_meme_bottom) { Ok(()) }
                                else { Err(format!("pic::meme failed for {:?}", input)) }
                            },
                            "upscale" => {
                                log_info(&format!("Image upscale: factor={}x", img_upscale_factor));
                                if modules::pic::upscale(&input, &output, img_upscale_factor) { Ok(()) }
                                else { Err(format!("pic::upscale failed for {:?}", input)) }
                            },
                            "html_to_image" => {
                                match input.parent() {
                                    None => Err(format!("html_to_image: fichier sans dossier parent | {:?}", input)),
                                    Some(parent) => {
                                        let png_out = parent.join(format!(
                                            "{}_oxytools.png",
                                            input.file_stem().unwrap_or_default().to_string_lossy()
                                        ));
                                        log_info(&format!("Image html_to_image: {:?} -> {:?}", input, png_out));
                                        if modules::pic::html_to_image(&input, &png_out, 1024) { Ok(()) }
                                        else { Err(format!("pic::html_to_image failed for {:?}", input)) }
                                    }
                                }
                            },
                            autre => {
                                log_warn(&format!("Image: action inconnue '{}', fallback compresser", autre));
                                if modules::pic::compresser(&input, &output, ratio) { Ok(()) }
                                else { Err(format!("pic::compresser fallback failed for {:?}", input)) }
                            },
                        }
                    },
                    _ => Ok(()),
                };

                // ── Résultat + timing ────────────────────────────────────
                let elapsed = start.elapsed();
                match &result {
                    Ok(()) => {
                        if elapsed.as_secs() > 30 {
                            log_warn(&format!(
                                "OK mais LENT ({:.1}s) | module={:?} | {:?}",
                                elapsed.as_secs_f32(), module, input
                            ));
                        } else {
                            log_info(&format!(
                                "OK ({:.2}s) | module={:?} | {:?}",
                                elapsed.as_secs_f32(), module, input
                            ));
                        }
                    },
                    Err(raison) => {
                        log_error(&format!(
                            "FAILED ({:.2}s) | module={:?} | file={:?} | reason={}",
                            elapsed.as_secs_f32(), module, input, raison
                        ));
                    }
                }

                *active.lock().unwrap_or_else(|e| e.into_inner()) -= 1;
                *completed.lock().unwrap_or_else(|e| e.into_inner()) += 1;
                let done = *completed.lock().unwrap_or_else(|e| e.into_inner());
                let total_count = *total.lock().unwrap_or_else(|e| e.into_inner());
                if done >= total_count {
                    log_info(&format!("=== BATCH END | {}/{} files processed ===", done, total_count));
                    *status_arc.lock().unwrap_or_else(|e| e.into_inner()) = crate::lang::fmt2(lang.done_files, &done.to_string(), &total_count.to_string());
                } else {
                    *status_arc.lock().unwrap_or_else(|e| e.into_inner()) = crate::lang::fmt2(lang.processing_files, &done.to_string(), &total_count.to_string());
                }
                ctx.request_repaint();
            }
        });
    }
}
