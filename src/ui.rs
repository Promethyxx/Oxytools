// ═══════════════════════════════════════════════════════════════
//  OXYTOOLS — Rendu de l'interface (egui)
// ═══════════════════════════════════════════════════════════════

use crate::app_state::{OxytoolsApp, ModuleType};
#[cfg(feature = "api")]
use crate::app_state::ScrapeEntry;
use crate::{modules, utils, log_error, VERSION};
use eframe::egui;
use std::sync::{Arc, Mutex};

impl eframe::App for OxytoolsApp {
    fn on_exit(&mut self) {
        // Kill tous les process ffmpeg en cours à la fermeture
        let pids = self.active_pids.lock().unwrap_or_else(|e| e.into_inner()).clone();
        for pid in pids {
            #[cfg(unix)]
            { let _ = std::process::Command::new("kill").arg(pid.to_string()).status(); }
            #[cfg(windows)]
            { let _ = std::process::Command::new("taskkill").args(["/PID", &pid.to_string(), "/F"]).status(); }
        }
        self.active_pids.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let ctx = &ctx;
        if ctx.cumulative_pass_nr() == 0 {
            self.load_config();
            self.apply_theme(ctx);
            self.verifier_deps();
        }
        // Si un job en arrière-plan a produit un résultat qui remplace la file
        // d'attente (ex: un merge combinant plusieurs PDF en un seul), on
        // bascule dessus ici — sinon une opération suivante tournerait à tort
        // sur les fichiers d'origine plutôt que sur le résultat produit.
        if let Some(nouveaux) = self.fichiers_produits.lock().unwrap_or_else(|e| e.into_inner()).take() {
            self.current_files = nouveaux;
        }
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                self.current_files = i.raw.dropped_files.iter().filter_map(|f| {
                    // Standard path (works on Windows and macOS)
                    if let Some(ref path) = f.path {
                        return Some(path.clone());
                    }
                    // Linux fallback: some DEs provide the path as bytes (file:// URI)
                    if let Some(ref bytes) = f.bytes {
                        let text = String::from_utf8_lossy(bytes);
                        for line in text.lines() {
                            let line = line.trim();
                            if line.starts_with("file://") {
                                let path_str = line.trim_start_matches("file://");
                                let decoded = utils::percent_decode(path_str);
                                let p = std::path::PathBuf::from(&decoded);
                                if p.exists() {
                                    return Some(p);
                                }
                            }
                        }
                    }
                    // Last resort: try the name field
                    if !f.name.is_empty() {
                        let p = std::path::PathBuf::from(&f.name);
                        if p.exists() { return Some(p); }
                    }
                    None
                }).collect();
                self.current_files.sort();
                if let Some(p) = self.current_files.first() {
                    self.current_stem = p.file_stem().unwrap_or_default().to_string_lossy().to_string();
                }
                #[cfg(feature = "api")]
                self.results_ui.lock().unwrap_or_else(|e| e.into_inner()).clear();
                if !self.current_files.is_empty() {
                    *self.status.lock().unwrap_or_else(|e| e.into_inner()) = self.lang.files_loaded.replace("{}", &self.current_files.len().to_string());
                }
            }
        });
        if let Some(ref mut c) = self.process {
            if let Ok(Some(_)) = c.try_wait() {
                *self.status.lock().unwrap_or_else(|e| e.into_inner()) = self.lang.done.into();
                self.process = None;
            }
            ctx.request_repaint();
        }
        egui::CentralPanel::default().show(ui, |ui| {
            ui.vertical_centered(|ui| ui.heading(format!("OXYTOOLS v{}", VERSION)));
            if !self.deps_manquantes.is_empty() {
                ui.colored_label(egui::Color32::RED, self.lang.missing.replace("{}", &self.deps_manquantes.join(", ")));
            }
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                let mut mods = vec![];
                mods.push((ModuleType::Archive, self.lang.tab_archive));
                #[cfg(feature = "api")] mods.push((ModuleType::Audio, self.lang.tab_audio));
                mods.push((ModuleType::Doc, self.lang.tab_doc));
                mods.push((ModuleType::Image, self.lang.tab_image));
                mods.push((ModuleType::Rename, self.lang.tab_rename));
                #[cfg(feature = "api")] mods.push((ModuleType::Scrapper, self.lang.tab_scrapper));
                #[cfg(feature = "api")] mods.push((ModuleType::Tag, self.lang.tab_tag));
                #[cfg(feature = "api")] mods.push((ModuleType::Video, self.lang.tab_video));
                mods.push((ModuleType::Settings, self.lang.tab_settings));
                for (m, txt) in mods {
                    if ui.selectable_value(&mut self.module_actif, m, txt).clicked() {
                        self.load_config();
                    }
                }
            });
            ui.separator();
            match self.module_actif {
                ModuleType::Archive => {
                    ui.horizontal(|ui| {
                        ui.label(self.lang.action_label);
                        egui::ComboBox::from_id_salt("archive_action").selected_text(
                            match self.archive_action.as_str() {
                                "compress" => "Compress",
                                "extract" => "Extract",
                                "convert" => "Convert",
                                "backup" => "Backup",
                                "multi" => "Multi-compress",
                                _ => "Compress",
                            }
                        ).show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.archive_action, "compress".into(), "Compress");
                            ui.selectable_value(&mut self.archive_action, "extract".into(), "Extract");
                            ui.selectable_value(&mut self.archive_action, "convert".into(), "Convert");
                            ui.selectable_value(&mut self.archive_action, "backup".into(), "Backup");
                            ui.selectable_value(&mut self.archive_action, "multi".into(), "Multi-compress");
                        });
                    });
                    ui.separator();
                    match self.archive_action.as_str() {
                        "compress" => {
                            ui.horizontal(|ui| {
                                ui.label(self.lang.format_label);
                                egui::ComboBox::from_id_salt("arfmt").selected_text(&self.format_choisi).show_ui(ui, |ui| {
                                    for f in ["7z", "tar", "zip"] {
                                        ui.selectable_value(&mut self.format_choisi, f.into(), f);
                                    }
                                });
                            });
                            if ui.add(egui::Slider::new(&mut self.archive_niveau, 1..=9).text(self.lang.compression_slider)).changed() {
                                self.save_config();
                            }
                            if ui.checkbox(&mut self.save_archive_format, self.lang.save_format).changed() {
                                self.save_config();
                            }
                        },
                        "extract" => {
                            ui.label("Drop archive files above, they will be extracted next to the original.");
                        },
                        "convert" => {
                            ui.label("Drop archive files above to convert to another format.");
                            ui.horizontal(|ui| {
                                ui.label(self.lang.format_label);
                                egui::ComboBox::from_id_salt("arfmt_conv").selected_text(&self.format_choisi).show_ui(ui, |ui| {
                                    for f in ["7z", "tar", "zip"] {
                                        ui.selectable_value(&mut self.format_choisi, f.into(), f);
                                    }
                                });
                            });
                        },
                        "backup" => {
                            ui.horizontal(|ui| {
                                ui.label("Source:");
                                ui.add(egui::TextEdit::singleline(&mut self.archive_backup_source).desired_width(250.0));
                                if ui.button("📂").clicked()
                                    && let Some(path) = rfd::FileDialog::new().pick_folder() {
                                        self.archive_backup_source = path.to_string_lossy().to_string();
                                        self.save_config();
                                    }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Destination:");
                                ui.add(egui::TextEdit::singleline(&mut self.archive_backup_dest).desired_width(250.0));
                                if ui.button("📂").clicked()
                                    && let Some(path) = rfd::FileDialog::new().pick_folder() {
                                        self.archive_backup_dest = path.to_string_lossy().to_string();
                                        self.save_config();
                                    }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Exclusions:");
                                if ui.add(egui::TextEdit::singleline(&mut self.archive_backup_exclusions).desired_width(250.0)).lost_focus() {
                                    self.save_config();
                                }
                            });
                            ui.small("Comma-separated folder names to exclude (e.g. .git, target, node_modules)");
                            ui.add_space(5.0);
                            let can_backup = !self.archive_backup_source.is_empty() && !self.archive_backup_dest.is_empty();
                            if ui.add_enabled(can_backup, egui::Button::new("▶ Run Backup")).clicked() {
                                let exclusions: Vec<&str> = self.archive_backup_exclusions
                                    .split(',')
                                    .map(|s| s.trim())
                                    .filter(|s| !s.is_empty())
                                    .collect();
                                let source = std::path::Path::new(&self.archive_backup_source);
                                let dest = std::path::Path::new(&self.archive_backup_dest);
                                match modules::archive::backup_zip(source, dest, &exclusions) {
                                    Ok(path) => {
                                        *self.status.lock().unwrap_or_else(|e| e.into_inner()) = format!("✅ Backup: {}", path);
                                    }
                                    Err(e) => {
                                        *self.status.lock().unwrap_or_else(|e| e.into_inner()) = format!("⚠️ Backup: {}", e);
                                    }
                                }
                            }
                        },
                        "multi" => {
                            ui.horizontal(|ui| {
                                ui.label("Parent folder:");
                                ui.add(egui::TextEdit::singleline(&mut self.archive_multi_source).desired_width(250.0));
                                if ui.button("📂").clicked()
                                    && let Some(path) = rfd::FileDialog::new().pick_folder() {
                                        self.archive_multi_source = path.to_string_lossy().to_string();
                                        self.save_config();
                                    }
                            });
                            ui.horizontal(|ui| {
                                ui.label(self.lang.format_label);
                                egui::ComboBox::from_id_salt("arfmt_multi").selected_text(&self.format_choisi).show_ui(ui, |ui| {
                                    for f in ["7z", "tar", "zip"] {
                                        ui.selectable_value(&mut self.format_choisi, f.into(), f);
                                    }
                                });
                            });
                            if ui.add(egui::Slider::new(&mut self.archive_niveau, 1..=9).text(self.lang.compression_slider)).changed() {
                                self.save_config();
                            }
                            ui.small("Each direct subfolder will be compressed as an individual archive.");
                            ui.add_space(5.0);
                            let can_multi = !self.archive_multi_source.is_empty();
                            if ui.add_enabled(can_multi, egui::Button::new("▶ Compress All Subfolders")).clicked() {
                                let parent = std::path::Path::new(&self.archive_multi_source);
                                match std::fs::read_dir(parent) {
                                    Ok(entries) => {
                                        self.current_files = entries
                                            .filter_map(|e| e.ok())
                                            .map(|e| e.path())
                                            .filter(|p| p.is_dir())
                                            .collect();
                                        if self.current_files.is_empty() {
                                            *self.status.lock().unwrap_or_else(|e| e.into_inner()) = "No subfolders found.".into();
                                        } else {
                                            crate::log_info(&format!(
                                                "Archive multi: {} subfolders found in {:?}",
                                                self.current_files.len(), parent
                                            ));
                                            self.lancer_batch(ctx.clone());
                                        }
                                    }
                                    Err(e) => {
                                        *self.status.lock().unwrap_or_else(|e| e.into_inner()) = format!("⚠️ {}", e);
                                    }
                                }
                            }
                        },
                        _ => {}
                    }
                },
                ModuleType::Doc => {
                    ui.horizontal(|ui| {
                        ui.label(self.lang.action_label);
                        egui::ComboBox::from_id_salt("doc_action").selected_text(&self.doc_action).show_ui(ui, |ui| {
							ui.selectable_value(&mut self.doc_action, "Convert".into(), self.lang.doc_convert);
                            ui.selectable_value(&mut self.doc_action, "pdf_annotate".into(), "PDF Annotate");
							ui.selectable_value(&mut self.doc_action, "pdf_compress".into(), self.lang.doc_pdf_compress);
							ui.selectable_value(&mut self.doc_action, "pdf_crop".into(), self.lang.doc_pdf_crop);
                            ui.selectable_value(&mut self.doc_action, "pdf_delete_pages".into(), self.lang.doc_pdf_delete_pages);
							ui.selectable_value(&mut self.doc_action, "pdf_merge".into(), self.lang.doc_pdf_merge);
                            ui.selectable_value(&mut self.doc_action, "pdf_numbers".into(), self.lang.doc_pdf_number_pages);
                            ui.selectable_value(&mut self.doc_action, "pdf_organize".into(), self.lang.doc_pdf_organize);
							ui.selectable_value(&mut self.doc_action, "pdf_protect".into(), self.lang.doc_pdf_protect);
                            ui.selectable_value(&mut self.doc_action, "pdf_repair".into(), self.lang.doc_pdf_repair);
							ui.selectable_value(&mut self.doc_action, "pdf_rotate".into(), self.lang.doc_pdf_rotate);
                            ui.selectable_value(&mut self.doc_action, "pdf_sign".into(), "PDF Sign");
							ui.selectable_value(&mut self.doc_action, "pdf_split".into(), self.lang.doc_pdf_split);
							ui.selectable_value(&mut self.doc_action, "pdf_unlock".into(), self.lang.doc_pdf_unlock);
                            ui.selectable_value(&mut self.doc_action, "pdf_watermark".into(), self.lang.doc_pdf_watermark);
                        });
                    });
                    ui.separator();
                    match self.doc_action.as_str() {
                        "Convert" => {
                            let input_exts: std::collections::HashSet<String> = self.current_files.iter()
                                .filter_map(|p| p.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()))
                                .collect();
                            let formats_dispo: Vec<&str> = ["docx", "epub", "html", "md", "odt", "pdf", "rtf", "txt"]
                                .into_iter()
                                .filter(|f| !input_exts.contains(*f))
                                .collect();
                            if input_exts.contains(self.format_choisi.as_str())
                                && let Some(&premier) = formats_dispo.first() {
                                    self.format_choisi = premier.to_string();
                                }
                            ui.horizontal(|ui| {
                                ui.label(self.lang.format_label);
                                egui::ComboBox::from_id_salt("dfmt").selected_text(&self.format_choisi).show_ui(ui, |ui| {
                                    for f in &formats_dispo {
                                        ui.selectable_value(&mut self.format_choisi, (*f).into(), *f);
                                    }
                                });
                            });
                            if ui.checkbox(&mut self.save_doc_format, self.lang.save_format).changed() {
                                self.save_config();
                            }
                        },
                        "pdf_split" => {
                            ui.label(self.lang.doc_split_hint1);
                            ui.label(self.lang.doc_split_hint2);
                        },
                        "pdf_merge" => {
                            ui.label(self.lang.doc_merge_hint1);
                            ui.label(self.lang.doc_merge_hint2);
                        },
                        "pdf_rotate" => {
                            ui.horizontal(|ui| {
                                ui.label(self.lang.angle_label);
                                egui::ComboBox::from_id_salt("pdf_rot").selected_text(format!("{}°", self.pdf_rotation_angle)).show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.pdf_rotation_angle, 90, "90°");
                                    ui.selectable_value(&mut self.pdf_rotation_angle, 180, "180°");
                                    ui.selectable_value(&mut self.pdf_rotation_angle, 270, "270°");
                                });
                            });
                            ui.horizontal(|ui| {
                                ui.label(self.lang.pages_hint);
                                ui.text_edit_singleline(&mut self.pdf_pages_spec);
                            });
                        },
                        "pdf_compress" => {
                            ui.label(self.lang.doc_compress_hint);
                            ui.horizontal(|ui| {
                                ui.label(self.lang.doc_compress_niveau);
                                ui.add(egui::Slider::new(&mut self.pdf_compress_niveau, 0..=9));
                            });
                        },
                        "pdf_crop" => {
                            ui.label(self.lang.doc_margins);
                            ui.horizontal(|ui| {
                                ui.label("X:");
                                ui.add(egui::Slider::new(&mut self.pdf_crop_x, 0.0..=100.0).fixed_decimals(1));
                                ui.label("Y:");
                                ui.add(egui::Slider::new(&mut self.pdf_crop_y, 0.0..=100.0).fixed_decimals(1));
                            });
                            ui.horizontal(|ui| {
                                ui.label("W:");
                                ui.add(egui::Slider::new(&mut self.pdf_crop_w, 1.0..=100.0).fixed_decimals(1));
                                ui.label("H:");
                                ui.add(egui::Slider::new(&mut self.pdf_crop_h, 1.0..=100.0).fixed_decimals(1));
                            });
                            ui.horizontal(|ui| {
                                ui.label(self.lang.pages_label);
                                ui.text_edit_singleline(&mut self.pdf_pages_spec);
                                ui.label(self.lang.pages_hint);
                            });
                        },
                        "pdf_organize" => {
                            ui.label(self.lang.doc_new_order);
                            ui.text_edit_singleline(&mut self.pdf_nouvel_ordre);
                        },
                        "pdf_delete_pages" => {
                            ui.label(self.lang.doc_delete_pages);
                            ui.text_edit_singleline(&mut self.pdf_pages_spec);
                        },
                        "pdf_numbers" => {
                            ui.horizontal(|ui| {
                                ui.label(self.lang.doc_start);
                                ui.add(egui::Slider::new(&mut self.pdf_num_debut, 1..=999));
                            });
                            ui.horizontal(|ui| {
                                ui.label(self.lang.doc_position);
                                egui::ComboBox::from_id_salt("pdf_numpos").selected_text(&self.pdf_num_position).show_ui(ui, |ui| {
                                    for pos in ["BasCentre","BasGauche","BasDroite","HautCentre","HautGauche","HautDroite"] {
                                        ui.selectable_value(&mut self.pdf_num_position, pos.into(), pos);
                                    }
                                });
                            });
                            ui.horizontal(|ui| {
                                ui.label(self.lang.doc_size);
                                ui.add(egui::Slider::new(&mut self.pdf_num_taille, 6.0..=36.0).fixed_decimals(0));
                            });
                        },
                        "pdf_protect" => {
                            ui.horizontal(|ui| {
                                ui.label(self.lang.doc_owner_password);
                                ui.add(egui::TextEdit::singleline(&mut self.pdf_owner_pass).password(true));
                            });
                            ui.horizontal(|ui| {
                                ui.label(self.lang.doc_user_password);
                                ui.add(egui::TextEdit::singleline(&mut self.pdf_user_pass).password(true));
                            });
                            ui.checkbox(&mut self.pdf_allow_print, self.lang.doc_allow_print);
                            ui.checkbox(&mut self.pdf_allow_copy, self.lang.doc_allow_copy);
                        },
                        "pdf_unlock" => {
                            ui.horizontal(|ui| {
                                ui.label(self.lang.doc_password);
                                ui.add(egui::TextEdit::singleline(&mut self.pdf_unlock_pass).password(true));
                            });
                        },
                        "pdf_repair" => {
                            ui.label(self.lang.doc_repair_hint1);
                            ui.label(self.lang.doc_repair_hint2);
                            ui.label(self.lang.doc_repair_hint3);
                        },
                        "pdf_watermark" => {
                            ui.label(self.lang.doc_watermark_hint1);
                            ui.label(self.lang.doc_repair_hint3);
                            ui.horizontal(|ui| {
                                ui.label(self.lang.doc_text);
                                ui.text_edit_singleline(&mut self.pdf_wm_texte);
                            });
                            ui.horizontal(|ui| {
                                ui.label(self.lang.doc_size);
                                ui.add(egui::Slider::new(&mut self.pdf_wm_taille, 12.0..=120.0).fixed_decimals(0));
                            });
                            ui.horizontal(|ui| {
                                ui.label(self.lang.doc_opacity);
                                ui.add(egui::Slider::new(&mut self.pdf_wm_opacite, 0.05..=1.0).fixed_decimals(2));
                            });
                            ui.horizontal(|ui| {
                                ui.label(self.lang.pages_label);
                                ui.text_edit_singleline(&mut self.pdf_pages_spec);
                                ui.label(self.lang.pages_hint);
                            });
                        },
                        "pdf_annotate" => {
                            ui.label("Add a text annotation (FreeText) to PDF pages.");
                            ui.horizontal(|ui| {
                                ui.label("Text:");
                                ui.text_edit_singleline(&mut self.pdf_annot_texte);
                            });
                            ui.label("Position & size (% of page):");
                            ui.horizontal(|ui| {
                                ui.label("X:");
                                ui.add(egui::Slider::new(&mut self.pdf_annot_x, 0.0..=100.0).fixed_decimals(1));
                                ui.label("Y:");
                                ui.add(egui::Slider::new(&mut self.pdf_annot_y, 0.0..=100.0).fixed_decimals(1));
                            });
                            ui.horizontal(|ui| {
                                ui.label("W:");
                                ui.add(egui::Slider::new(&mut self.pdf_annot_w, 1.0..=100.0).fixed_decimals(1));
                                ui.label("H:");
                                ui.add(egui::Slider::new(&mut self.pdf_annot_h, 1.0..=100.0).fixed_decimals(1));
                            });
                            ui.horizontal(|ui| {
                                ui.label(self.lang.pages_label);
                                ui.text_edit_singleline(&mut self.pdf_pages_spec);
                                ui.label(self.lang.pages_hint);
                            });
                        },
                        "pdf_sign" => {
                            ui.label("Add a visual signature line with name and date.");
                            ui.horizontal(|ui| {
                                ui.label("Name:");
                                ui.text_edit_singleline(&mut self.pdf_sign_nom);
                            });
                            ui.horizontal(|ui| {
                                ui.label(self.lang.doc_position);
                                egui::ComboBox::from_id_salt("pdf_signpos").selected_text(&self.pdf_sign_position).show_ui(ui, |ui| {
                                    for pos in ["BasCentre","BasGauche","BasDroite","HautCentre","HautGauche","HautDroite"] {
                                        ui.selectable_value(&mut self.pdf_sign_position, pos.into(), pos);
                                    }
                                });
                            });
                            ui.horizontal(|ui| {
                                ui.label(self.lang.doc_size);
                                ui.add(egui::Slider::new(&mut self.pdf_sign_taille, 6.0..=24.0).fixed_decimals(0));
                            });
                            ui.horizontal(|ui| {
                                ui.label(self.lang.pages_label);
                                ui.text_edit_singleline(&mut self.pdf_pages_spec);
                                ui.label(self.lang.pages_hint);
                            });
                        },
                        _ => {}
                    }
                },
                ModuleType::Image => {
                    ui.horizontal(|ui| {
                        ui.label(self.lang.action_label);
                        egui::ComboBox::from_id_salt("img_action").selected_text(&self.image_action).show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.image_action, "Convert".into(), self.lang.doc_convert);
							ui.selectable_value(&mut self.image_action, "crop".into(), self.lang.img_crop);
                            ui.selectable_value(&mut self.image_action, "html_to_image".into(), "HTML to Image");
                            ui.selectable_value(&mut self.image_action, "meme".into(), "Meme Generator");
                            ui.selectable_value(&mut self.image_action, "resize".into(), self.lang.img_resize);
                            ui.selectable_value(&mut self.image_action, "rotate".into(), self.lang.img_rotate);
                            ui.selectable_value(&mut self.image_action, "upscale".into(), "Upscale");
                            ui.selectable_value(&mut self.image_action, "watermark".into(), "Watermark");
                        });
                    });
                    ui.separator();
                    match self.image_action.as_str() {
                        "Convert" => {
                            ui.horizontal(|ui| {
                                ui.label(self.lang.format_label);
                                egui::ComboBox::from_id_salt("ifmt").selected_text(&self.format_choisi).show_ui(ui, |ui| {
                                    for f in ["exr","gif","ico","jpg","jxl","png","tiff","webp"] {
                                        ui.selectable_value(&mut self.format_choisi, f.into(), f);
                                    }
                                });
                            });
                            if ui.checkbox(&mut self.save_image_format, self.lang.save_format).changed() {
                                self.save_config();
                            }
                            if ui.add(egui::Slider::new(&mut self.ratio_img, 1..=10).text(self.lang.img_quality_slider)).changed() {
                                self.save_config();
                            }
                            // Output size (optional, for all formats)
                            if self.format_choisi.to_uppercase() != "ICO" {
                                ui.separator();
                                ui.label("Output size (optional, leave empty for original):");
                                ui.horizontal(|ui| {
                                    ui.label("W:");
                                    ui.add(egui::TextEdit::singleline(&mut self.resize_width).desired_width(60.0).hint_text("px"));
                                    ui.label("H:");
                                    ui.add(egui::TextEdit::singleline(&mut self.resize_height).desired_width(60.0).hint_text("px"));
                                });
                            }
                            // Sous-options JXL
                            if self.format_choisi.to_uppercase() == "JXL" {
                                ui.separator();
                                ui.horizontal(|ui| {
                                    ui.label("JXL mode:");
                                    egui::ComboBox::from_id_salt("jxl_mode").selected_text(
                                        match self.jxl_mode.as_str() {
                                            "folder" => "Folder (separate dir)",
                                            "pivot" => "Pivot (via PNG)",
                                            _ => "Lossless (in-place)",
                                        }
                                    ).show_ui(ui, |ui| {
                                        ui.selectable_value(&mut self.jxl_mode, "lossless".into(), "Lossless (in-place)");
                                        ui.selectable_value(&mut self.jxl_mode, "folder".into(), "Folder (separate dir)");
                                        ui.selectable_value(&mut self.jxl_mode, "pivot".into(), "Pivot (via PNG)");
                                    });
                                });
                                match self.jxl_mode.as_str() {
                                    "folder" => { ui.small("Output in a \"{folder} jxl\" directory next to the source folder."); },
                                    "pivot" => { ui.small("Re-decode via PNG pivot for problematic files, output: {name}_pivot.jxl."); },
                                    _ => { ui.small("Lossless JXL next to the original, skips if .jxl already exists."); },
                                }
                            }
                            // Sous-options ICO
                            if self.format_choisi.to_uppercase() == "ICO" {
                                ui.separator();
                                ui.label("ICO sizes (multi-size icon):");
                                ui.horizontal(|ui| {
                                    ui.checkbox(&mut self.ico_size_16, "16×16");
                                    ui.checkbox(&mut self.ico_size_32, "32×32");
                                    ui.checkbox(&mut self.ico_size_64, "64×64");
                                });
                                ui.horizontal(|ui| {
                                    ui.checkbox(&mut self.ico_size_128, "128×128");
                                    ui.checkbox(&mut self.ico_size_256, "256×256");
                                    ui.checkbox(&mut self.ico_size_512, "512×512");
                                });
                                ui.horizontal(|ui| {
                                    ui.checkbox(&mut self.ico_size_custom, "Custom:");
                                    ui.add_enabled(self.ico_size_custom, egui::TextEdit::singleline(&mut self.ico_custom_w).desired_width(50.0).hint_text("W"));
                                    ui.label("×");
                                    ui.add_enabled(self.ico_size_custom, egui::TextEdit::singleline(&mut self.ico_custom_h).desired_width(50.0).hint_text("H"));
                                });
                                ui.small("Check multiple sizes to generate a multi-resolution .ico file.");
                            }
                        },
                        "resize" => {
                            ui.horizontal(|ui| {
                                ui.label(self.lang.format_label);
                                egui::ComboBox::from_id_salt("ifmt_resize").selected_text(&self.format_choisi).show_ui(ui, |ui| {
                                    for f in ["exr","gif","ico","jpg","jxl","png","tiff","webp"] {
                                        ui.selectable_value(&mut self.format_choisi, f.into(), f);
                                    }
                                });
                            });
                            if ui.checkbox(&mut self.save_image_format, self.lang.save_format).changed() {
                                self.save_config();
                            }
                            ui.separator();
                            ui.label(self.lang.img_resize_px);
                            ui.horizontal(|ui| {
                                ui.label(self.lang.img_width);
                                ui.text_edit_singleline(&mut self.resize_width);
                                ui.label(self.lang.img_height);
                                ui.text_edit_singleline(&mut self.resize_height);
                            });
                            ui.label(self.lang.img_andor);
                            ui.horizontal(|ui| {
                                ui.label(self.lang.img_max_size);
                                ui.text_edit_singleline(&mut self.resize_max_kb);
                            });
                        },
                        "rotate" => {
                            ui.horizontal(|ui| {
                                ui.label(self.lang.angle_label);
                                egui::ComboBox::from_id_salt("rot_angle").selected_text(format!("{}°", self.rotation_angle)).show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.rotation_angle, 90, "90°");
                                    ui.selectable_value(&mut self.rotation_angle, 180, "180°");
                                    ui.selectable_value(&mut self.rotation_angle, 270, "270°");
                                });
                            });
                        },
                        "crop" => {
                            ui.label(self.lang.img_coordinates);
                            ui.horizontal(|ui| {
                                ui.label("X:");
                                ui.add(egui::Slider::new(&mut self.crop_x, 0..=100));
                                ui.label("Y:");
                                ui.add(egui::Slider::new(&mut self.crop_y, 0..=100));
                            });
                            ui.horizontal(|ui| {
                                ui.label(self.lang.img_width);
                                ui.add(egui::Slider::new(&mut self.crop_width, 1..=100));
                                ui.label(self.lang.img_height);
                                ui.add(egui::Slider::new(&mut self.crop_height, 1..=100));
                            });
                        },
                        "watermark" => {
                            ui.label("Add a text watermark over the image.");
                            ui.horizontal(|ui| {
                                ui.label("Text:");
                                ui.text_edit_singleline(&mut self.img_wm_texte);
                            });
                            ui.horizontal(|ui| {
                                ui.label("Size:");
                                ui.add(egui::Slider::new(&mut self.img_wm_taille, 12.0..=120.0).fixed_decimals(0));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Opacity:");
                                ui.add(egui::Slider::new(&mut self.img_wm_opacite, 0.05..=1.0).fixed_decimals(2));
                            });
                        },
                        "meme" => {
                            ui.label("Add meme-style text (white on black bars).");
                            ui.horizontal(|ui| {
                                ui.label("Top text:");
                                ui.text_edit_singleline(&mut self.img_meme_top);
                            });
                            ui.horizontal(|ui| {
                                ui.label("Bottom text:");
                                ui.text_edit_singleline(&mut self.img_meme_bottom);
                            });
                        },
                        "upscale" => {
                            ui.label("Enlarge image using Lanczos interpolation.");
                            ui.horizontal(|ui| {
                                ui.label("Factor:");
                                egui::ComboBox::from_id_salt("upscale_factor").selected_text(format!("{}x", self.img_upscale_factor)).show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.img_upscale_factor, 2, "2x");
                                    ui.selectable_value(&mut self.img_upscale_factor, 3, "3x");
                                    ui.selectable_value(&mut self.img_upscale_factor, 4, "4x");
                                    ui.selectable_value(&mut self.img_upscale_factor, 8, "8x");
                                });
                            });
                        },
                        "html_to_image" => {
                            ui.label("Render an HTML file as a PNG image.");
                            ui.small("Drop an .html file, output will be a PNG snapshot of the text content.");
                        },
                        _ => {}
                    }
                },
                #[cfg(feature = "api")]
                ModuleType::Audio => {
                    ui.horizontal(|ui| {
                        ui.label(self.lang.action_label);
                        egui::ComboBox::from_id_salt("audio_action").selected_text(&self.audio_action).show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.audio_action, "extract".into(), self.lang.audio_extract);
							ui.selectable_value(&mut self.audio_action, "Convert".into(), self.lang.doc_convert);                            
                        });
                    });
                    ui.separator();
                    match self.audio_action.as_str() {
                        "Convert" => {
                            // Détection codec au chargement de fichiers
                            if let Some(f) = self.current_files.first()
                                && ui.button(self.lang.audio_detect_codec).clicked() {
                                    let codec = modules::audio::detecter_extension(f);
                                    let fmts = modules::audio::formats_compatibles(&codec);
                                    self.audio_formats_dispo = fmts.iter().map(|s| s.to_string()).collect();
                                    crate::log_info(&format!("Audio: codec detected='{}' | formats compatibles={:?}", codec, self.audio_formats_dispo));
                                }
                            ui.horizontal(|ui| {
                                ui.label(self.lang.format_label);
                                egui::ComboBox::from_id_salt("afmt").selected_text(&self.format_choisi).show_ui(ui, |ui| {
                                    for f in &self.audio_formats_dispo {
                                        ui.selectable_value(&mut self.format_choisi, f.clone(), f.as_str());
                                    }
                                });
                            });
                            if ui.add(egui::Slider::new(&mut self.audio_qualite, 0..=9).text(self.lang.audio_vbr_slider)).changed() {
                                self.save_config();
                            }
                            if ui.checkbox(&mut self.save_audio_format, self.lang.save_format).changed() {
                                self.save_config();
                            }
                        },
                        "extract" => {
                            ui.label(self.lang.audio_extract_hint1);
                            ui.label(self.lang.audio_extract_hint2);
                        },
                        _ => {}
                    }
                },
                #[cfg(feature = "api")]
                ModuleType::Video => {
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt("vfmt").selected_text(&self.format_choisi).show_ui(ui, |ui| {
                            for f in ["mkv","mp4","webm"] {
                                ui.selectable_value(&mut self.format_choisi, f.into(), f);
                            }
                        });
                        if ui.checkbox(&mut self.copie_flux, self.lang.video_stream_copy).changed() { self.save_config(); }
                    });
                    // Codec/accélération n'ont aucun effet en mode copie de flux
                    // (-c copy conserve le codec source tel quel).
                    ui.add_enabled_ui(!self.copie_flux, |ui| {
                        let dispo = modules::video::codecs_et_accelerations_disponibles();
                        ui.horizontal(|ui| {
                            ui.label(self.lang.video_codec_label);
                            egui::ComboBox::from_id_salt("vcodec").selected_text(&self.video_codec).show_ui(ui, |ui| {
                                if ui.selectable_value(&mut self.video_codec, "auto".into(), "auto").changed() {
                                    self.save_config();
                                }
                                for (codec, _) in &dispo {
                                    if ui.selectable_value(&mut self.video_codec, codec.to_string(), *codec).changed() {
                                        self.save_config();
                                    }
                                }
                            });
                        });
                        // Liste des accélérations réellement utilisables pour le codec choisi
                        // (ou l'union de toutes si "auto") — on ne propose jamais une option
                        // vouée à échouer sur cette machine.
                        let accels_pour_codec: Vec<&str> = if self.video_codec == "auto" {
                            let mut tous: Vec<&str> = dispo.iter().flat_map(|(_, a)| a.iter().copied()).collect();
                            tous.sort();
                            tous.dedup();
                            tous
                        } else {
                            dispo.iter()
                                .find(|(c, _)| *c == self.video_codec)
                                .map(|(_, a)| a.clone())
                                .unwrap_or_else(|| vec!["software"])
                        };
                        ui.horizontal(|ui| {
                            ui.label(self.lang.video_accel_label);
                            egui::ComboBox::from_id_salt("vaccel").selected_text(&self.video_accel).show_ui(ui, |ui| {
                                if ui.selectable_value(&mut self.video_accel, "auto".into(), "auto").changed() {
                                    self.save_config();
                                }
                                for accel in &accels_pour_codec {
                                    if ui.selectable_value(&mut self.video_accel, accel.to_string(), *accel).changed() {
                                        self.save_config();
                                    }
                                }
                            });
                        });
                    });
                    if ui.add(egui::Slider::new(&mut self.video_speed, 0..=8).text(self.lang.video_quality_slider)).changed() {
                        self.save_config();
                    }
                    if ui.checkbox(&mut self.save_video_format, self.lang.save_format).changed() {
                        self.save_config();
                    }
                },
                #[cfg(feature = "api")]
                ModuleType::Scrapper => {
                    // ── Champs clés ──────────────────────────────────────────
                    ui.horizontal(|ui| {
                        ui.label(self.lang.scrap_tmdb_key);
                        ui.add(egui::TextEdit::singleline(&mut self.tmdb_api_key).password(true));
                    });
                    ui.horizontal(|ui| {
                        ui.label(self.lang.scrap_fanart_key);
                        ui.add(egui::TextEdit::singleline(&mut self.fanart_api_key).password(true));
                    });

                    // ── Chemin cible du .env ─────────────────────────────────
                    let default_env = self.config_dir().join(".env");
                    let save_target = self.keys_env_path.clone().unwrap_or_else(|| default_env.clone());
                    ui.horizontal(|ui| {
                        ui.label(self.lang.scrap_keys_path);
                        ui.label(
                            egui::RichText::new(save_target.to_string_lossy())
                                .weak()
                                .italics()
                        );
                        if ui.small_button(self.lang.scrap_browse_keys).clicked()
                            && let Some(p) = rfd::FileDialog::new()
                                .add_filter("env", &["env"])
                                .set_file_name(".env")
                                .save_file()
                            {
                                self.keys_env_path = Some(p);
                                self.save_config();
                            }
                    });

                    // ── Save / Load ──────────────────────────────────────────
                    ui.horizontal(|ui| {
                        if ui.button(self.lang.scrap_save_keys).clicked() {
                            let target = self.keys_env_path.clone().unwrap_or_else(|| default_env.clone());
                            let do_write = if target.exists() {
                                rfd::MessageDialog::new()
                                    .set_title("Oxytools")
                                    .set_description(format!(
                                        "{} — {}",
                                        target.to_string_lossy(),
                                        if self.lang_id == "fr" {
                                            "Ce fichier existe déjà. Écraser ?"
                                        } else {
                                            "This file already exists. Overwrite?"
                                        }
                                    ))
                                    .set_buttons(rfd::MessageButtons::YesNo)
                                    .show() == rfd::MessageDialogResult::Yes
                            } else {
                                true
                            };
                            if do_write {
                                let content = format!(
                                    "TMDB_API_KEY={}\nFANART_API_KEY={}\n",
                                    self.tmdb_api_key, self.fanart_api_key
                                );
                                match std::fs::write(&target, content) {
                                    Ok(_) => self.scrap_status = format!("✅ {}", target.to_string_lossy()),
                                    Err(e) => self.scrap_status = format!("⚠️ {}", e),
                                }
                            }
                        }

                        if ui.button(self.lang.scrap_load_keys).clicked()
                            && let Some(p) = rfd::FileDialog::new()
                                .add_filter("env", &["env", "txt"])
                                .pick_file()
                            {
                                if let Ok(content) = std::fs::read_to_string(&p) {
                                    for line in content.lines() {
                                        let line = line.trim().trim_matches('\r');
                                        if line.starts_with('#') || !line.contains('=') { continue; }
                                        let (raw_key, raw_val) = line.split_once('=').unwrap();
                                        let key = raw_key.trim().trim_matches('\r').replace([' ', '_'], "").to_lowercase();
                                        let val = raw_val.trim().trim_matches('\r').trim_matches('"').trim_matches('\'').trim().to_string();
                                        if key.contains("tmdb") {
                                            self.tmdb_api_key = val;
                                        } else if key.contains("fanart") {
                                            self.fanart_api_key = val;
                                        }
                                    }
                                    self.scrap_status = format!("📂 {}", p.to_string_lossy());
                                } else {
                                    self.scrap_status = format!("⚠️ {}", p.to_string_lossy());
                                }
                            }
                    });

                    // ── Status save/load ─────────────────────────────────────
                    if !self.scrap_status.is_empty() {
                        ui.label(egui::RichText::new(&self.scrap_status).weak().small());
                    }

                    ui.separator();

                    // ── Options fanart/clearlogo (point 2) ───────────────────
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.fetch_fanart, self.lang.scrap_fetch_fanart);
                        ui.checkbox(&mut self.fetch_clearlogo, self.lang.scrap_fetch_clearlogo);
                        ui.separator();
                        let was_enabled = self.scrap_manual_search_enabled;
                        ui.checkbox(&mut self.scrap_manual_search_enabled, self.lang.scrap_manual_search);
                        // Ouvrir la popup immédiatement si on coche la case
                        if self.scrap_manual_search_enabled && !was_enabled {
                            self.scrap_popup_query = Some(self.current_stem.clone());
                        }
                        // Refermer si on décoche
                        if !self.scrap_manual_search_enabled && self.scrap_popup_query.is_some() {
                            self.scrap_popup_query = None;
                        }
                    });

                    ui.separator();

                    // ── Recherche ────────────────────────────────────────────
                    ui.horizontal(|ui| {
                        let status_arc = Arc::clone(&self.status);
                        let lang = self.lang;
                        let tmdb_key = self.tmdb_api_key.clone();
                        let search = |is_series: bool, res_arc: Arc<Mutex<Vec<ScrapeEntry>>>, stem: String, ctx_c: egui::Context, status_c: Arc<Mutex<String>>, tk: String, nrf: Arc<Mutex<bool>>| {
                            res_arc.lock().unwrap_or_else(|e| e.into_inner()).clear();
                            std::thread::spawn(move || {
                                match modules::scrap::search_tmdb(&stem, is_series, &tk) {
                                    Ok(results) if results.is_empty() => {
                                        *status_c.lock().unwrap_or_else(|e| e.into_inner()) = lang.scrap_no_results.into();
                                        *nrf.lock().unwrap_or_else(|e| e.into_inner()) = true;
                                        ctx_c.request_repaint();
                                    }
                                    Ok(results) => {
                                        for r in results {
                                            let tex = r.poster_path.as_ref()
                                                .and_then(|p| modules::scrap::download_image_bytes(p))
                                                .and_then(|b| image::load_from_memory(&b).ok())
                                                .map(|img| {
                                                    let ci = egui::ColorImage::from_rgba_unmultiplied(
                                                        [img.width() as usize, img.height() as usize],
                                                        img.to_rgba8().as_flat_samples().as_slice()
                                                    );
                                                    ctx_c.load_texture(format!("p_{}", r.id), ci, Default::default())
                                                });
                                            res_arc.lock().unwrap_or_else(|e| e.into_inner()).push(ScrapeEntry { data: r, texture: tex });
                                            ctx_c.request_repaint();
                                        }
                                    }
                                    Err(e) => {
                                        *status_c.lock().unwrap_or_else(|e| e.into_inner()) = format!("⚠️ {}", e);
                                        ctx_c.request_repaint();
                                    }
                                }
                            });
                        };
                        let no_result_flag = Arc::clone(&self.no_result_flag);
                        if ui.button(self.lang.scrap_movie).clicked() {
                            if tmdb_key.is_empty() {
                                *self.status.lock().unwrap_or_else(|e| e.into_inner()) = self.lang.scrap_error_no_key.into();
                            } else {
                                self.scrap_last_is_series = false;
                                search(false, Arc::clone(&self.results_ui), self.current_stem.clone(), ctx.clone(), Arc::clone(&status_arc), tmdb_key.clone(), Arc::clone(&no_result_flag));
                            }
                        }
                        if ui.button(self.lang.scrap_series).clicked() {
                            if tmdb_key.is_empty() {
                                *self.status.lock().unwrap_or_else(|e| e.into_inner()) = self.lang.scrap_error_no_key.into();
                            } else {
                                self.scrap_last_is_series = true;
                                let query = if let Some((title, _)) = modules::scrap::detect_series(&self.current_files) {
                                    title
                                } else {
                                    self.current_stem.clone()
                                };
                                search(true, Arc::clone(&self.results_ui), query, ctx.clone(), Arc::clone(&status_arc), tmdb_key.clone(), Arc::clone(&no_result_flag));
                            }
                        }
                    });

                    // Vérifier le flag no_result entre les frames et ouvrir la popup
                    if *self.no_result_flag.lock().unwrap_or_else(|e| e.into_inner()) {
                        *self.no_result_flag.lock().unwrap_or_else(|e| e.into_inner()) = false;
                        self.scrap_popup_query = Some(self.current_stem.clone());
                    }

                    // ── Popup aucun résultat ─────────────────────────────────
                    if self.scrap_popup_query.is_some() {
                        let mut open = true;
                        egui::Window::new(self.lang.scrap_retry_hint)
                            .collapsible(false)
                            .resizable(false)
                            .open(&mut open)
                            .show(ctx, |ui| {
                                let q = self.scrap_popup_query.as_mut().unwrap();
                                ui.text_edit_singleline(q);
                                let query2 = q.clone();
                                ui.horizontal(|ui| {
                                    let status_arc2 = Arc::clone(&self.status);
                                    let tmdb_key2 = self.tmdb_api_key.clone();
                                    let lang2 = self.lang;
                                    let res2 = Arc::clone(&self.results_ui);
                                    let ctx2 = ctx.clone();
                                    let is_series2 = self.scrap_last_is_series;
                                    if ui.button(self.lang.scrap_retry_search).clicked() {
                                        self.scrap_popup_query = None;
                                        res2.lock().unwrap_or_else(|e| e.into_inner()).clear();

                                        // Détecter si c'est un ID numérique ou une URL TMDB
                                        let trimmed = query2.trim().to_string();
                                        let tmdb_id: Option<i64> = trimmed.parse::<i64>().ok().or_else(|| {
                                            // URL : themoviedb.org/tv/95224 ou /movie/12345
                                            let re = regex::Regex::new(r"themoviedb\.org/(?:tv|movie)/(\d+)").unwrap();
                                            re.captures(&trimmed).and_then(|c| c.get(1)).and_then(|m| m.as_str().parse().ok())
                                        });
                                        // Détecter is_series depuis l'URL si présent
                                        let is_series_from_url = if trimmed.contains("themoviedb.org/tv/") { Some(true) }
                                            else if trimmed.contains("themoviedb.org/movie/") { Some(false) }
                                            else { None };
                                        let resolved_is_series = is_series_from_url.unwrap_or(is_series2);

                                        std::thread::spawn(move || {
                                            let result = if let Some(id) = tmdb_id {
                                                modules::scrap::fetch_by_tmdb_id(id, resolved_is_series, &tmdb_key2)
                                            } else {
                                                modules::scrap::search_tmdb(&trimmed, resolved_is_series, &tmdb_key2)
                                            };
                                            match result {
                                                Ok(results) if results.is_empty() => {
                                                    *status_arc2.lock().unwrap_or_else(|e| e.into_inner()) = lang2.scrap_no_results.into();
                                                    ctx2.request_repaint();
                                                }
                                                Ok(results) => {
                                                    for r in results {
                                                        let tex = r.poster_path.as_ref()
                                                            .and_then(|p| modules::scrap::download_image_bytes(p))
                                                            .and_then(|b| image::load_from_memory(&b).ok())
                                                            .map(|img| {
                                                                let ci = egui::ColorImage::from_rgba_unmultiplied(
                                                                    [img.width() as usize, img.height() as usize],
                                                                    img.to_rgba8().as_flat_samples().as_slice()
                                                                );
                                                                ctx2.load_texture(format!("p_{}", r.id), ci, Default::default())
                                                            });
                                                        res2.lock().unwrap_or_else(|e| e.into_inner()).push(ScrapeEntry { data: r, texture: tex });
                                                        ctx2.request_repaint();
                                                    }
                                                }
                                                Err(e) => {
                                                    *status_arc2.lock().unwrap_or_else(|e| e.into_inner()) = format!("⚠️ {}", e);
                                                    ctx2.request_repaint();
                                                }
                                            }
                                        });
                                    }
                                });
                            });
                        if !open {
                            self.scrap_popup_query = None;
                            self.scrap_manual_search_enabled = false;
                        }
                    }

                    // ── Résultats avec année + images plus grandes + plein écran (point 3) ──
                    let entries = self.results_ui.lock().unwrap_or_else(|e| e.into_inner()).clone();
                    egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                        for entry in &entries {
                            ui.horizontal(|ui| {
                                // Image cliquable — taille 90x135
                                if let Some(t) = &entry.texture {
                                    let img = egui::Image::new((t.id(), egui::vec2(90.0, 135.0)))
                                        .sense(egui::Sense::click());
                                    let resp = ui.add(img);
                                    if resp.clicked() {
                                        self.scrap_fullscreen = Some(t.clone());
                                    }
                                    resp.on_hover_text("Cliquer pour agrandir");
                                } else {
                                    ui.allocate_space(egui::vec2(90.0, 135.0));
                                }
                                ui.vertical(|ui| {
                                    // Titre + année (point 3)
                                    let year = entry.data.release_date.split('-').next().unwrap_or("");
                                    ui.strong(format!("{} ({})", entry.data.title, year));
                                    ui.label(egui::RichText::new(&entry.data.overview).small().weak());
                                    if !self.current_files.is_empty()
                                        && ui.button(self.lang.scrap_choose).clicked() {
                                            let fanart = self.fanart_api_key.clone();
                                            let ff = self.fetch_fanart;
                                            let fc = self.fetch_clearlogo;
                                            if entry.data.is_series {
                                                let series_dir = if self.current_files[0].is_dir() {
                                                    self.current_files[0].clone()
                                                } else {
                                                    self.current_files[0].parent().unwrap_or(&self.current_files[0]).to_path_buf()
                                                };
                                                // Nom de la série : si dossier → nom du dossier,
                                                // si fichier → titre extrait par extract_series_info (sans S01E01),
                                                // fallback stem complet
                                                let series_name = if self.current_files[0].is_dir() {
                                                    self.current_files[0].file_name().unwrap_or_default().to_string_lossy().to_string()
                                                } else {
                                                    let stem = self.current_files[0].file_stem().unwrap_or_default().to_string_lossy();
                                                    modules::scrap::extract_series_info(&stem)
                                                        .map(|(title, _)| title)
                                                        .unwrap_or_else(|| stem.to_string())
                                                };
                                                let detected = modules::scrap::collect_season_numbers(&self.current_files);
                                                modules::scrap::save_series_metadata(series_dir, &series_name, entry.data.clone(), &detected, &fanart, ff, fc);
                                            } else {
                                                modules::scrap::save_metadata(self.current_files[0].clone(), entry.data.clone(), &fanart, ff, fc);
                                            }
                                        }
                                });
                            });
                            ui.separator();
                        }
                    });

                    // ── Plein écran image (point 3) ──────────────────────────
                    if self.scrap_fullscreen.is_some() {
                        let mut open = true;
                        egui::Window::new("🖼")
                            .collapsible(false)
                            .resizable(true)
                            .open(&mut open)
                            .show(ctx, |ui| {
                                if let Some(t) = &self.scrap_fullscreen {
                                    let avail = ui.available_size();
                                    ui.image((t.id(), avail));
                                }
                            });
                        if !open { self.scrap_fullscreen = None; }
                    }
                },
                #[cfg(feature = "api")]
                ModuleType::Tag => {
                    let has_files = !self.current_files.is_empty();
                    ui.vertical(|ui| {
                        ui.add_enabled_ui(has_files, |ui| {
                            if ui.button(self.lang.tag_mark_watched).clicked() {
                                let (mut ok, mut err) = (0usize, 0usize);
                                for path in &self.current_files {
                                    match modules::tag::marquer_vu(path, &path.with_extension("nfo"), self.lang_id) {
                                        Ok(_) => ok += 1,
                                        Err(e) => { crate::log_error(&format!("marquer_vu {:?}: {}", path, e)); err += 1; }
                                    }
                                }
                                *self.status.lock().unwrap_or_else(|e| e.into_inner()) = format!("✅ {ok} VU | ⚠️ {err}");
                            }
                            if ui.button(self.lang.tag_inject_nfo).clicked() {
                                let (mut ok, mut err) = (0usize, 0usize);
                                for path in &self.current_files {
                                    let nfo = if self.use_custom_nfo {
                                        self.custom_nfo_path.clone().unwrap_or_else(|| path.with_extension("nfo"))
                                    } else {
                                        path.with_extension("nfo")
                                    };
                                    match modules::tag::appliquer_tags(path, &nfo) {
                                        Ok(_) => ok += 1,
                                        Err(e) => { crate::log_error(&format!("appliquer_tags {:?}: {}", path, e)); err += 1; }
                                    }
                                }
                                *self.status.lock().unwrap_or_else(|e| e.into_inner()) = format!("✅ {ok} NFO | ⚠️ {err}");
                            }
                            if ui.button(self.lang.tag_add_poster).clicked() {
                                let (mut ok, mut err) = (0usize, 0usize);
                                for path in &self.current_files {
                                    let result = if self.use_custom_poster {
                                        if let Some(ref poster) = self.custom_poster_path {
                                            modules::tag::injecter_poster_custom(path, poster)
                                        } else {
                                            modules::tag::ajouter_images_mkv(path)
                                        }
                                    } else {
                                        modules::tag::ajouter_images_mkv(path)
                                    };
                                    match result {
                                        Ok(_) => ok += 1,
                                        Err(e) => { crate::log_error(&format!("ajouter_images {:?}: {}", path, e)); err += 1; }
                                    }
                                }
                                *self.status.lock().unwrap_or_else(|e| e.into_inner()) = format!("✅ {ok} images | ⚠️ {err}");
                            }

                            // ── Inject NFO + Poster avec checkboxes custom ───
                            ui.horizontal(|ui| {
                                if ui.button(self.lang.tag_inject_nfo_and_poster).clicked() {
                                    let (mut ok, mut err) = (0usize, 0usize);
                                    for path in &self.current_files {
                                        let nfo = if self.use_custom_nfo {
                                            self.custom_nfo_path.clone().unwrap_or_else(|| path.with_extension("nfo"))
                                        } else {
                                            path.with_extension("nfo")
                                        };
                                        let r_nfo = modules::tag::appliquer_tags(path, &nfo);
                                        let r_poster = if self.use_custom_poster {
                                            if let Some(ref poster) = self.custom_poster_path {
                                                modules::tag::injecter_poster_custom(path, poster)
                                            } else {
                                                modules::tag::ajouter_images_mkv(path)
                                            }
                                        } else {
                                            modules::tag::ajouter_images_mkv(path)
                                        };
                                        match (r_nfo, r_poster) {
                                            (Ok(_), Ok(_)) => ok += 1,
                                            (Err(e), _) | (_, Err(e)) => { crate::log_error(&format!("{:?}: {}", path, e)); err += 1; }
                                        }
                                    }
                                    *self.status.lock().unwrap_or_else(|e| e.into_inner()) = format!("✅ {ok} NFO+poster | ⚠️ {err}");
                                }
                                // Checkbox NFO custom
                                ui.checkbox(&mut self.use_custom_nfo, self.lang.tag_custom_nfo);
                                if self.use_custom_nfo {
                                    let label = self.custom_nfo_path.as_ref()
                                        .map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string())
                                        .unwrap_or_else(|| "…".into());
                                    if ui.small_button(format!("📄 {}", label)).clicked()
                                        && let Some(p) = rfd::FileDialog::new().add_filter("NFO", &["nfo"]).pick_file() {
                                            self.custom_nfo_path = Some(p);
                                        }
                                }
                                // Checkbox poster custom
                                ui.checkbox(&mut self.use_custom_poster, self.lang.tag_custom_poster);
                                if self.use_custom_poster {
                                    let label = self.custom_poster_path.as_ref()
                                        .map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string())
                                        .unwrap_or_else(|| "…".into());
                                    if ui.small_button(format!("🖼 {}", label)).clicked()
                                        && let Some(p) = rfd::FileDialog::new()
                                            .add_filter("Image", &["jpg", "jpeg", "png"])
                                            .pick_file()
                                        {
                                            self.custom_poster_path = Some(p);
                                        }
                                }
                            });
                            if ui.button(self.lang.tag_reset_tags).clicked() {
                                let (mut ok, mut err) = (0usize, 0usize);
                                let mkv_files = utils::expand_to_mkv(&self.current_files);
                                for path in &mkv_files {
                                    match modules::tag::supprimer_tous_tags(path) {
                                        Ok(_) => ok += 1,
                                        Err(e) => { crate::log_error(&format!("supprimer_tags {:?}: {}", path, e)); err += 1; }
                                    }
                                }
                                *self.status.lock().unwrap_or_else(|e| e.into_inner()) = format!("✅ {ok} reset | ⚠️ {err}");
                            }
                            ui.horizontal(|ui| {
                                ui.text_edit_singleline(&mut self.tag_edit_val);
                                if ui.button(self.lang.tag_edit_title).clicked() {
                                    let (mut ok, mut err) = (0usize, 0usize);
                                    let val = self.tag_edit_val.clone();
                                    for path in &self.current_files {
                                        match modules::tag::modifier_tag(path, "title", &val) {
                                            Ok(_) => ok += 1,
                                            Err(e) => { crate::log_error(&format!("modifier_tag {:?}: {}", path, e)); err += 1; }
                                        }
                                    }
                                    *self.status.lock().unwrap_or_else(|e| e.into_inner()) = format!("✅ {ok} title | ⚠️ {err}");
                                }
                            });
                        });
                    });
                },
                ModuleType::Rename => {
                    ui.vertical(|ui| {
                        self.rename_previews = modules::rename::preview(&self.current_files, &self.rename_cfg);

                        ui.heading(self.lang.tab_rename);
                        ui.separator();

                        // ── Find & Replace ──────────────────────────────────
                        ui.collapsing(self.lang.rename_find_replace, |ui| {
                            ui.horizontal(|ui| {
                                ui.selectable_value(&mut self.rename_cfg.multi_replace, false, "Simple"); // TODO lang
                                ui.selectable_value(&mut self.rename_cfg.multi_replace, true, "Multiple"); // TODO lang
                            });
                            if !self.rename_cfg.multi_replace {
                                // ── Mode simple ──
                                ui.horizontal(|ui| {
                                    ui.label(self.lang.rename_find);
                                    ui.text_edit_singleline(&mut self.rename_cfg.find);
                                });
                                ui.horizontal(|ui| {
                                    ui.label(self.lang.rename_replace_with);
                                    ui.text_edit_singleline(&mut self.rename_cfg.replace_with);
                                });
                            } else {
                                // ── Mode multiple ──
                                ui.horizontal(|ui| {
                                    ui.label(self.lang.rename_find);
                                    ui.add(egui::TextEdit::singleline(&mut self.rename_multi_find).desired_width(150.0));
                                    ui.label(self.lang.rename_replace_with);
                                    ui.add(egui::TextEdit::singleline(&mut self.rename_multi_replace).desired_width(150.0));
                                    if ui.button("➕").on_hover_text("Add rule").clicked() && !self.rename_multi_find.is_empty() { // TODO lang
                                        self.rename_cfg.replace_list.add(
                                            self.rename_multi_find.clone(),
                                            self.rename_multi_replace.clone(),
                                        );
                                        self.rename_multi_find.clear();
                                        self.rename_multi_replace.clear();
                                    }
                                });
                                // Import / Export (toujours visible en haut)
                                ui.horizontal(|ui| {
                                    // ── Profil actif ────────────────────────────────
                                    if let Some(ref p) = self.rename_last_list_path {
                                        ui.small(format!("📌 {}", p.file_name().unwrap_or_default().to_string_lossy()));
                                        if ui.small_button("✖").on_hover_text("Forget profile").clicked() {
                                            self.rename_last_list_path = None;
                                            self.save_config();
                                        }
                                        ui.separator();
                                    }
                                    if ui.button("💾 Save").clicked() { // TODO lang
                                        if let Some(path) = rfd::FileDialog::new()
                                            .add_filter("TSV", &["tsv"])
                                            .set_file_name("replace_rules.tsv")
                                            .save_file()
                                            && let Err(e) = self.rename_cfg.replace_list.save(&path) {
                                                log_error(&format!("Save replace list: {}", e));
                                            }
                                    }
                                    if ui.button("📂 Load").clicked() { // TODO lang
                                        if let Some(path) = rfd::FileDialog::new()
                                            .add_filter("TSV", &["tsv"])
                                            .pick_file()
                                        {
                                            match modules::rename::ReplaceList::load(&path) {
                                                Ok(list) => {
                                                    self.rename_cfg.replace_list = list;
                                                    self.rename_last_list_path = Some(path);
                                                    self.save_config();
                                                },
                                                Err(e) => log_error(&format!("Load replace list: {}", e)),
                                            }
                                        }
                                    }
                                    if ui.button("📂 Ant Renamer").clicked() { // TODO lang
                                        if let Some(path) = rfd::FileDialog::new()
                                            .add_filter("XML", &["xml"])
                                            .pick_file()
                                        {
                                            match modules::rename::ReplaceList::list_ant_renamer_sets(&path) {
                                                Ok(sets) if sets.is_empty() => {
                                                    // Pas de sets nommés, charger CurrentList
                                                    match modules::rename::ReplaceList::load_ant_renamer_xml(&path, None) {
                                                        Ok(list) => self.rename_cfg.replace_list = list,
                                                        Err(e) => log_error(&format!("Import Ant Renamer: {}", e)),
                                                    }
                                                }
                                                Ok(sets) if sets.len() == 1 => {
                                                    // Un seul set, charger directement
                                                    match modules::rename::ReplaceList::load_ant_renamer_xml(&path, Some(&sets[0])) {
                                                        Ok(list) => self.rename_cfg.replace_list = list,
                                                        Err(e) => log_error(&format!("Import Ant Renamer: {}", e)),
                                                    }
                                                }
                                                Ok(sets) => {
                                                    // Plusieurs sets → afficher le sélecteur
                                                    self.rename_ant_sets = sets;
                                                    self.rename_ant_path = Some(path);
                                                }
                                                Err(e) => log_error(&format!("Import Ant Renamer: {}", e)),
                                            }
                                        }
                                    }
                                    if ui.button("🗑").on_hover_text("Clear all rules").clicked() { // TODO lang
                                        self.rename_cfg.replace_list.rules.clear();
                                        self.rename_last_list_path = None;
                                        self.save_config();
                                    }
                                });
                                // Sélecteur de Set Ant Renamer (affiché si plusieurs sets disponibles)
                                if !self.rename_ant_sets.is_empty() {
                                    ui.horizontal(|ui| {
                                        ui.label("Set :"); // TODO lang
                                        for set_name in self.rename_ant_sets.clone() {
                                            if ui.button(&set_name).clicked() {
                                                if let Some(ref path) = self.rename_ant_path {
                                                    match modules::rename::ReplaceList::load_ant_renamer_xml(path, Some(&set_name)) {
                                                        Ok(list) => self.rename_cfg.replace_list = list,
                                                        Err(e) => log_error(&format!("Import Ant Renamer set '{}': {}", set_name, e)),
                                                    }
                                                }
                                                self.rename_ant_sets.clear();
                                                self.rename_ant_path = None;
                                            }
                                        }
                                        // Aussi proposer CurrentList
                                        if ui.button("CurrentList").clicked() {
                                            if let Some(ref path) = self.rename_ant_path {
                                                match modules::rename::ReplaceList::load_ant_renamer_xml(path, None) {
                                                    Ok(list) => self.rename_cfg.replace_list = list,
                                                    Err(e) => log_error(&format!("Import Ant Renamer CurrentList: {}", e)),
                                                }
                                            }
                                            self.rename_ant_sets.clear();
                                            self.rename_ant_path = None;
                                        }
                                        if ui.button("✖").clicked() {
                                            self.rename_ant_sets.clear();
                                            self.rename_ant_path = None;
                                        }
                                    });
                                }
                                // Tableau des règles (scrollable)
                                let mut to_remove: Option<usize> = None;
                                let mut to_move_up: Option<usize> = None;
                                let mut to_move_down: Option<usize> = None;
                                if !self.rename_cfg.replace_list.rules.is_empty() {
                                    egui::ScrollArea::vertical().max_height(200.0).id_salt("multi_replace_scroll").show(ui, |ui| {
                                        egui::Grid::new("multi_replace_grid").striped(true).show(ui, |ui| {
                                            ui.label(""); // checkbox col
                                            ui.strong(self.lang.rename_find);
                                            ui.strong(self.lang.rename_replace_with);
                                            ui.label(""); // actions
                                            ui.end_row();
                                            for (i, rule) in self.rename_cfg.replace_list.rules.iter_mut().enumerate() {
                                                ui.checkbox(&mut rule.enabled, "");
                                                ui.add(egui::TextEdit::singleline(&mut rule.find).desired_width(140.0));
                                                ui.add(egui::TextEdit::singleline(&mut rule.replace).desired_width(140.0));
                                                ui.horizontal(|ui| {
                                                    if ui.small_button("▲").clicked() { to_move_up = Some(i); }
                                                    if ui.small_button("▼").clicked() { to_move_down = Some(i); }
                                                    if ui.small_button("🗑").clicked() { to_remove = Some(i); }
                                                });
                                                ui.end_row();
                                            }
                                        });
                                    });
                                }
                                if let Some(i) = to_move_up { self.rename_cfg.replace_list.move_up(i); }
                                if let Some(i) = to_move_down { self.rename_cfg.replace_list.move_down(i); }
                                if let Some(i) = to_remove { self.rename_cfg.replace_list.remove(i); }
                            }
                        });

                        // ── Insertion ───────────────────────────────────────
                        ui.collapsing(self.lang.rename_insert, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(self.lang.rename_text);
                                ui.text_edit_singleline(&mut self.rename_cfg.insert_text);
                            });
                            ui.horizontal(|ui| {
                                ui.label(self.lang.rename_at_pos);
                                ui.add(egui::DragValue::new(&mut self.rename_cfg.insert_pos).range(0..=999));
                                ui.selectable_value(&mut self.rename_cfg.insert_from_end, false, "↦ From start"); // TODO lang
                                ui.selectable_value(&mut self.rename_cfg.insert_from_end, true, "↤ From end"); // TODO lang
                            });
                        });

                        // ── Suppression de plage ────────────────────────────
                        ui.collapsing(self.lang.rename_delete_range, |ui| {
                            ui.checkbox(&mut self.rename_cfg.delete_enabled, self.lang.rename_enable);
                            ui.horizontal(|ui| {
                                ui.label(self.lang.rename_from);
                                ui.add(egui::DragValue::new(&mut self.rename_cfg.delete_from).range(0..=999));
                                ui.label(self.lang.rename_count);
                                ui.add(egui::DragValue::new(&mut self.rename_cfg.delete_count).range(0..=999));
                                ui.selectable_value(&mut self.rename_cfg.delete_from_end, false, "↦ From start"); // TODO lang
                                ui.selectable_value(&mut self.rename_cfg.delete_from_end, true, "↤ From end"); // TODO lang
                            });
                        });

                        // ── Numérotation ────────────────────────────────────
                        ui.collapsing(self.lang.rename_numbering, |ui| {
                            ui.checkbox(&mut self.rename_cfg.num_enabled, self.lang.rename_enable);
                            if self.rename_cfg.num_enabled {
                                ui.horizontal(|ui| {
                                    ui.label(self.lang.rename_start);
                                    ui.add(egui::DragValue::new(&mut self.rename_cfg.num_start).range(0..=99999));
                                    ui.label(self.lang.rename_step);
                                    ui.add(egui::DragValue::new(&mut self.rename_cfg.num_step).range(1..=100));
                                    ui.label(self.lang.rename_padding);
                                    ui.add(egui::DragValue::new(&mut self.rename_cfg.num_padding).range(0..=10));
                                });
                                ui.horizontal(|ui| {
                                    ui.label(self.lang.rename_separator);
                                    ui.add(egui::TextEdit::singleline(&mut self.rename_cfg.num_sep).desired_width(40.0));
                                    ui.label(self.lang.rename_position);
                                    egui::ComboBox::from_id_salt("num_pos")
                                        .selected_text(match self.rename_cfg.num_pos {
                                            modules::rename::NumPos::Prefix => self.lang.rename_prefix,
                                            modules::rename::NumPos::Suffix => self.lang.rename_suffix,
                                        })
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(&mut self.rename_cfg.num_pos, modules::rename::NumPos::Prefix, self.lang.rename_prefix);
                                            ui.selectable_value(&mut self.rename_cfg.num_pos, modules::rename::NumPos::Suffix, self.lang.rename_suffix);
                                        });
                                });
                            }
                        });

                        // ── Casse ───────────────────────────────────────────
                        ui.collapsing(self.lang.rename_case, |ui| {
                            ui.horizontal(|ui| {
                                for mode in [
                                    modules::rename::CaseMode::Unchanged,
                                    modules::rename::CaseMode::Lower,
                                    modules::rename::CaseMode::Upper,
                                    modules::rename::CaseMode::Title,
                                    modules::rename::CaseMode::Sentence,
                                ] as [modules::rename::CaseMode; 5] {
                                    let label = mode.label();
                                    ui.selectable_value(&mut self.rename_cfg.case_mode, mode, label);
                                }
                            });
                        });

                        // ── Nettoyage ───────────────────────────────────────
                        ui.collapsing(self.lang.rename_clean, |ui| {
                            ui.checkbox(&mut self.rename_cfg.strip_trailing_spaces, self.lang.rename_trim_spaces);
                            ui.checkbox(&mut self.rename_cfg.strip_double_spaces, self.lang.rename_double_spaces);
                            ui.checkbox(&mut self.rename_cfg.strip_leading_dots, self.lang.rename_leading_dots);
                            ui.horizontal(|ui| {
                                ui.label(self.lang.rename_strip_chars);
                                ui.text_edit_singleline(&mut self.rename_cfg.strip_chars);
                            });
                        });

                        // ── Extension ───────────────────────────────────────
                        ui.collapsing(self.lang.rename_extension, |ui| {
                            ui.horizontal(|ui| {
                                for mode in [
                                    modules::rename::ExtMode::Unchanged,
                                    modules::rename::ExtMode::Lower,
                                    modules::rename::ExtMode::Upper,
                                    modules::rename::ExtMode::Replace,
                                    modules::rename::ExtMode::Remove,
                                ] as [modules::rename::ExtMode; 5] {
                                    let label = mode.label();
                                    ui.selectable_value(&mut self.rename_cfg.ext_mode, mode, label);
                                }
                            });
                            if self.rename_cfg.ext_mode == modules::rename::ExtMode::Replace {
                                ui.horizontal(|ui| {
                                    ui.label(self.lang.rename_new_ext);
                                    ui.text_edit_singleline(&mut self.rename_cfg.ext_new);
                                });
                            }
                        });

                        ui.separator();
                        if ui.button(self.lang.rename_reset).clicked() {
                            self.rename_cfg = modules::rename::RenameConfig::default();
                        }
                        ui.separator();

                        // ── Preview ─────────────────────────────────────────
                        if self.current_files.is_empty() {
                            ui.label(self.lang.drop_here);
                        } else {
                            egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                                egui::Grid::new("rename_preview").striped(true).min_col_width(200.0).show(ui, |ui| {
                                    ui.strong(self.lang.rename_original);
                                    ui.strong(self.lang.rename_new_name);
                                    ui.end_row();
                                    for (orig, new_name) in &self.rename_previews {
                                        let orig_str = orig.file_name().and_then(|n| n.to_str()).unwrap_or("");
                                        if orig_str != new_name {
                                            ui.colored_label(egui::Color32::LIGHT_GREEN, orig_str);
                                            ui.colored_label(egui::Color32::LIGHT_GREEN, new_name);
                                        } else {
                                            ui.label(orig_str);
                                            ui.label(new_name);
                                        }
                                        ui.end_row();
                                    }
                                });
                            });
                            ui.separator();
                            if !self.rename_results.is_empty() {
                                let ok  = self.rename_results.iter().filter(|r| r.success).count();
                                let err = self.rename_results.iter().filter(|r| !r.success).count();
                                if err > 0 {
                                    ui.colored_label(egui::Color32::RED, format!("✅ {}  ⚠️ {} errors", ok, err));
                                    for r in self.rename_results.iter().filter(|r| !r.success) {
                                        if let Some(e) = &r.error {
                                            ui.colored_label(egui::Color32::RED, format!("  • {}: {}", r.new_name, e));
                                        }
                                    }
                                } else {
                                    ui.colored_label(egui::Color32::LIGHT_GREEN, format!("✅ {} {}", ok, self.lang.rename_done));
                                }
                            }
                            if ui.button(self.lang.rename_apply).clicked() {
                                self.rename_results = modules::rename::apply_renames(&self.rename_previews);
                                let mut applied: Vec<modules::rename::RenameResult> = Vec::new();
                                for r in &self.rename_results {
                                    if r.success {
                                        if let Some(pos) = self.current_files.iter().position(|f| *f == r.original) {
                                            let parent = r.original.parent().unwrap_or(std::path::Path::new(""));
                                            self.current_files[pos] = parent.join(&r.new_name);
                                        }
                                        applied.push(modules::rename::RenameResult {
                                            original: r.original.clone(),
                                            new_name: r.new_name.clone(),
                                            success: true,
                                            error: None,
                                        });
                                    }
                                }
                                if !applied.is_empty() {
                                    self.rename_undo_stack.push(applied);
                                }
                            }
                            if let Some(last) = self.rename_undo_stack.last() {
                                let count = last.len();
                                if ui.button(format!("↩ Undo ({})", count)).clicked() {
                                    let batch = self.rename_undo_stack.pop().unwrap();
                                    let undo_results = modules::rename::undo_renames(&batch);
                                    for r in &undo_results {
                                        if r.success {
                                            let parent = r.original.parent().unwrap_or(std::path::Path::new(""));
                                            let restored = parent.join(&r.new_name);
                                            if let Some(pos) = self.current_files.iter().position(|f| *f == r.original) {
                                                self.current_files[pos] = restored;
                                            }
                                        }
                                    }
                                    self.rename_results = undo_results;
                                }
                            }
                        }
                    });
                },
                ModuleType::Settings => {
                    ui.vertical(|ui| {
                        ui.heading(self.lang.settings_heading);
                        let old_theme = self.current_theme.clone();
                        ui.horizontal(|ui| {
                            ui.label(self.lang.settings_theme);
                            egui::ComboBox::from_id_salt("theme_sel").selected_text(&self.current_theme).show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.current_theme, "Auto".into(), "Auto");
                                ui.selectable_value(&mut self.current_theme, "Light".into(), "Light");
                                ui.selectable_value(&mut self.current_theme, "Dark".into(), "Dark");
                            });
                        });
                        if self.current_theme != old_theme {
                            self.apply_theme(ctx);
                            self.save_config();
                        }
                        ui.horizontal(|ui| {
                            ui.label("Language:");
                            let lang_label = if self.lang_id == "fr" { "Français" } else { "English" };
                            egui::ComboBox::from_id_salt("lang_sel").selected_text(lang_label).show_ui(ui, |ui| {
                                if ui.selectable_label(self.lang_id == "en", "English").clicked() {
                                    self.lang = &crate::lang::EN; self.lang_id = "en"; self.save_config();
                                }
                                if ui.selectable_label(self.lang_id == "fr", "Français").clicked() {
                                    self.lang = &crate::lang::FR; self.lang_id = "fr"; self.save_config();
                                }
                            });
                        });
                        ui.separator();
                        ui.heading(self.lang.settings_performance);
                        ui.horizontal(|ui| {
                            ui.label(self.lang.settings_max_jobs);
                            if ui.add(egui::Slider::new(&mut self.max_parallel_jobs, 1..=16).text("threads")).changed() {
                                self.save_config();
                            }
                        });
                        ui.label(self.lang.settings_jobs_hint);
                        ui.separator();
                        ui.heading(self.lang.settings_config_dir);
                        ui.horizontal(|ui| {
                            let current = self.config_dir();
                            ui.label(egui::RichText::new(current.to_string_lossy()).weak().italics());
                        });
                        ui.horizontal(|ui| {
                            if ui.button("📁 Browse…").clicked()
                                && let Some(p) = rfd::FileDialog::new().pick_folder() {
                                    self.custom_config_dir = Some(p);
                                    self.save_config();
                                }
                            if self.custom_config_dir.is_some()
                                && ui.button(self.lang.settings_config_dir_reset).clicked() {
                                    self.custom_config_dir = None;
                                    // Supprimer config_dir du bootstrap
                                    let bootstrap_dir = Self::exe_config_dir();
                                    if let Ok(c) = std::fs::read_to_string(bootstrap_dir.join("oxytools.toml"))
                                        && let Ok(mut parsed) = c.parse::<toml::Table>() {
                                            if let Some(app) = parsed.get_mut("app").and_then(|a| a.as_table_mut()) {
                                                app.remove("config_dir");
                                            }
                                            let _ = std::fs::write(bootstrap_dir.join("oxytools.toml"), toml::to_string(&parsed).unwrap_or_default());
                                        }
                                }
                        });
                        ui.label(egui::RichText::new(self.lang.settings_config_dir_hint).small().weak());
                        ui.separator();
                        ui.heading("Logs");
                        if ui.button("📋 Open log file").clicked() {
                            let log_path = self.config_dir().join("oxytools.log");
                            if log_path.exists() {
                                let _ = open::that(&log_path);
                            } else {
                                *self.status.lock().unwrap_or_else(|e| e.into_inner()) = "No log file found.".into();
                            }
                        }
                    });
                },
            }
            let mut hide_exec = self.module_actif == ModuleType::Settings || self.module_actif == ModuleType::Rename;
            #[cfg(feature = "api")]
            { hide_exec = hide_exec || self.module_actif == ModuleType::Scrapper || self.module_actif == ModuleType::Tag; }
            if !self.current_files.is_empty() && !hide_exec {
                ui.separator();
                if ui.button(self.lang.run_all).clicked() {
                    self.lancer_batch(ctx.clone());
                }
            }
            if self.current_files.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(self.lang.drop_here);
                        ui.add_space(5.0);
                        if ui.button(self.lang.browse).clicked()
                            && let Some(paths) = rfd::FileDialog::new().pick_files() {
                                self.current_files = paths;
                                if let Some(p) = self.current_files.first() {
                                    self.current_stem = p.file_stem().unwrap_or_default().to_string_lossy().to_string();
                                }
                                #[cfg(feature = "api")]
                                self.results_ui.lock().unwrap_or_else(|e| e.into_inner()).clear();
                                *self.status.lock().unwrap_or_else(|e| e.into_inner()) = self.lang.files_loaded.replace("{}", &self.current_files.len().to_string());
                            }
                    });
                });
            }
            ui.add_space(10.0);
            ui.vertical_centered(|ui| {
                let completed = *self.completed_jobs.lock().unwrap_or_else(|e| e.into_inner());
                let total = *self.total_jobs.lock().unwrap_or_else(|e| e.into_inner());
                let ff_progress = *self.conv_progress.lock().unwrap_or_else(|e| e.into_inner());

                if ff_progress >= 0.0 {
                    // Conversion ffmpeg en cours — barre de progression du fichier courant
                    let pct = (ff_progress * 100.0).round() as u32;
                    ui.heading(format!("⚙️ {} {}%", completed + 1, pct));
                    ui.add(egui::ProgressBar::new(ff_progress).animate(false));
                } else if total > 0 && completed < total {
                    let active = *self.active_jobs.lock().unwrap_or_else(|e| e.into_inner());
                    let pct = (completed as f32 / total as f32 * 100.0).round() as u32;
                    ui.heading(crate::lang::fmt3(self.lang.processing_pct, &completed.to_string(), &total.to_string(), &pct.to_string()));
                    ui.add(egui::ProgressBar::new(completed as f32 / total as f32).animate(true));
                    ui.small(crate::lang::fmt2(self.lang.active_pending, &active.to_string(), &self.job_queue.lock().unwrap_or_else(|e| e.into_inner()).len().to_string()));
                } else if total > 0 && completed >= total {
                    ui.heading(self.lang.done_processed.replace("{}", &total.to_string()));
                } else {
                    ui.heading(&*self.status.lock().unwrap_or_else(|e| e.into_inner()));
                }
                if ff_progress >= 0.0 {
                    ctx.request_repaint_after(std::time::Duration::from_millis(100));
                }
            });
            if !self.current_files.is_empty() && ui.button(self.lang.clear_all).clicked() { self.current_files.clear(); }
        });
    }
}
