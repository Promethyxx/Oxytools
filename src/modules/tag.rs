use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use quick_xml::Reader;
use quick_xml::events::Event;
use crate::modules::binaries;
use crate::modules::scrap::extract_series_info;
/// Lit un fichier NFO et retourne un HashMap.
/// Les tags dupliqués (genre, tag, studio...) sont joints par " / ".
pub fn lire_nfo(nfo_path: &Path) -> Result<HashMap<String, String>, String> {
    let file = File::open(nfo_path).map_err(|e| e.to_string())?;
    let mut reader = Reader::from_reader(BufReader::new(file));
    let mut buf = Vec::new();
    let mut data: HashMap<String, Vec<String>> = HashMap::new();
    let mut current_tag = String::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                current_tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
            }
            Ok(Event::Text(e)) => {
                let text = e.decode().unwrap_or_default().trim().to_string();
                if !current_tag.is_empty() && !text.is_empty() {
                    data.entry(current_tag.clone()).or_default().push(text);
                }
            }
            Ok(Event::End(_)) => {
                current_tag.clear();
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("Erreur lecture NFO: {}", e)),
            _ => {}
        }
        buf.clear();
    }
    // Joindre les valeurs multiples avec " / "
    Ok(data.into_iter().map(|(k, v)| (k, v.join(" / "))).collect())
}
/// Crée le fichier XML temporaire pour mkvpropedit
fn creer_xml_tags(tags: &HashMap<String, String>) -> String {
    let mut xml = String::from("<?xml version=\"1.0\"?>\n<Tags>\n  <Tag>\n    <Targets><TargetTypeValue>50</TargetTypeValue></Targets>\n");
    for (cle, valeur) in tags {
        if !valeur.is_empty() {
            xml.push_str("    <Simple>\n");
            xml.push_str(&format!("      <Name>{}</Name>\n", cle.to_uppercase()));
            xml.push_str(&format!("      <String>{}</String>\n", valeur.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")));
            xml.push_str("    </Simple>\n");
        }
    }
    xml.push_str("  </Tag>\n</Tags>");
    xml
}
/// Lit les tags Matroska existants d'un MKV via ffprobe
fn lire_tags_mkv(mkv_path: &Path) -> HashMap<String, String> {
    let mut tags = HashMap::new();
    let output = binaries::silent_cmd(binaries::get_ffprobe())
        .args([
            "-v", "quiet",
            "-print_format", "json",
            "-show_entries", "format_tags",
            mkv_path.to_str().unwrap(),
        ])
        .output();
    if let Ok(out) = output {
        let json_str = String::from_utf8_lossy(&out.stdout);
        // Parse basique des paires "KEY": "VALUE" dans le bloc tags
        // Format attendu: "format": { "tags": { "KEY": "VALUE", ... } }
        if let Some(tags_start) = json_str.find("\"tags\"") {
            let bloc = &json_str[tags_start..];
            if let Some(brace) = bloc.find('{') {
                let inner = &bloc[brace + 1..];
                if let Some(end) = inner.find('}') {
                    let content = &inner[..end];
                    // Extraire chaque paire "key": "value"
                    let mut rest = content;
                    while let Some(q1) = rest.find('"') {
                        rest = &rest[q1 + 1..];
                        if let Some(q2) = rest.find('"') {
                            let key = rest[..q2].to_string();
                            rest = &rest[q2 + 1..];
                            // Chercher la valeur après le ":"
                            if let Some(colon) = rest.find(':') {
                                rest = &rest[colon + 1..];
                                if let Some(v1) = rest.find('"') {
                                    rest = &rest[v1 + 1..];
                                    if let Some(v2) = rest.find('"') {
                                        let val = rest[..v2].to_string();
                                        tags.insert(key.to_uppercase(), val);
                                        rest = &rest[v2 + 1..];
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    tags
}

/// 1. Marquer une vidéo comme 'VU' (cumul avec les tags existants, ne touche pas au NFO)
pub fn marquer_vu(mkv_path: &Path, _nfo_path: &Path, lang_id: &str) -> Result<(), String> {
    // Lire les tags existants du MKV via ffprobe
    let mut tags = lire_tags_mkv(mkv_path);

    // Incrémenter playcount (cumul)
    let pc = tags.get("PLAYCOUNT")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0) + 1;
    tags.insert("PLAYCOUNT".to_string(), pc.to_string());
    tags.insert("WATCHED".to_string(), "true".to_string());
    let watched_label = if lang_id == "fr" { "VU" } else { "WATCHED" };
    tags.insert("KEYWORDS".to_string(), watched_label.to_string());

    // Réinjecter tous les tags (existants + VU) dans le MKV
    let xml_content = creer_xml_tags(&tags);
    let temp_xml = "temp_vu.xml";
    std::fs::write(temp_xml, xml_content).map_err(|e| e.to_string())?;
    let status = binaries::silent_cmd(binaries::get_mkvpropedit())
        .args([mkv_path.to_str().unwrap(), "--tags", &format!("global:{}", temp_xml)])
        .status().map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(temp_xml);
    if status.success() { Ok(()) } else { Err("Erreur marquage VU complet".into()) }
}
/// 2. Modification directe
pub fn modifier_tag(mkv_path: &Path, tag: &str, valeur: &str) -> Result<(), String> {
    let status = binaries::silent_cmd(binaries::get_mkvpropedit())
        .args([
            mkv_path.to_str().unwrap(),
            "--edit", "info",
            "--set", &format!("{}={}", tag, valeur),
        ])
        .status().map_err(|e| e.to_string())?;
    if status.success() { Ok(()) } else { Err("Erreur modification tag".into()) }
}
/// 3. Injection complète depuis NFO
/// Pour une série, cherche seasonXX.nfo dans le dossier parent.
/// Pour un film, cherche le NFO du même nom que le fichier.
pub fn appliquer_tags(mkv_path: &Path, nfo_path: &Path) -> Result<(), String> {
    // Résoudre le bon NFO selon film ou série
    let resolved_nfo = resolve_nfo(mkv_path).unwrap_or_else(|| nfo_path.to_path_buf());
    let mut tags = lire_nfo(&resolved_nfo)?;
    // On supprime le statut de lecture pour ne pas l'écraser
    tags.remove("playcount");
    tags.remove("watched");
    tags.remove("watchedstatus");
    tags.remove("KEYWORDS"); // Statut VU géré séparément
    // Rating dans COMMENT
    if let Some(rating) = tags.get("value").cloned() {
        if let Ok(r) = rating.parse::<f64>() {
            let rounded = (r * 10.0).round() / 10.0;
            tags.insert("COMMENT".to_string(), format!("{} / 10", rounded));
        }
    }
    // Year → RELEASETIME
    if let Some(year) = tags.get("year").cloned() {
        if !year.is_empty() {
            tags.insert("RELEASETIME".to_string(), year);
        }
    }
    // premiered → DATE_RELEASED (année seulement)
    if let Some(premiered) = tags.remove("premiered") {
        if !premiered.is_empty() {
            let annee = premiered.split('-').next().unwrap_or(&premiered).to_string();
            tags.insert("DATE_RELEASED".to_string(), annee);
        }
    }
    let xml_content = creer_xml_tags(&tags);
    let temp_xml = "temp_meta.xml";
    std::fs::write(temp_xml, xml_content).map_err(|e| e.to_string())?;
    let status = binaries::silent_cmd(binaries::get_mkvpropedit())
        .args([mkv_path.to_str().unwrap(), "--tags", &format!("global:{}", temp_xml)])
        .status().map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(temp_xml);
    if status.success() { Ok(()) } else { Err("Erreur injection métadonnées".into()) }
}

/// Résout le chemin du NFO à utiliser pour un fichier donné.
/// Cherche d'abord dans le dossier parent, puis dans parent/Extras/.
/// - Série  → {titre}_season01.nfo puis season01.nfo (compat)
/// - Film   → stem.nfo
fn resolve_nfo(mkv_path: &Path) -> Option<std::path::PathBuf> {
    let parent = mkv_path.parent()?;
    let stem = mkv_path.file_stem()?.to_string_lossy();
    let extras_dir = parent.join("Extras");

    let find = |filename: &str| -> Option<std::path::PathBuf> {
        let p = parent.join(filename);
        if p.exists() { return Some(p); }
        if extras_dir.exists() {
            let p2 = extras_dir.join(filename);
            if p2.exists() { return Some(p2); }
        }
        None
    };

    if let Some((series_title, season_num)) = extract_series_info(&stem) {
        if let Some(p) = find(&format!("{}_season{:02}.nfo", series_title, season_num)) { return Some(p); }
        if let Some(p) = find(&format!("season{:02}.nfo", season_num)) { return Some(p); }
    }
    // Film
    find(&format!("{}.nfo", stem))
}

/// Résout le chemin du poster à utiliser pour un fichier donné :
/// Cherche d'abord dans le dossier parent, puis dans parent/Extras/ si il existe.
/// - Série → {titre}_seasonXX_poster.jpg puis compat season01_poster / season01-poster
/// - Film  → stem_poster.jpg, stem-poster.jpg, poster.jpg
fn resolve_poster(mkv_path: &Path) -> Option<std::path::PathBuf> {
    let parent = mkv_path.parent()?;
    let stem = mkv_path.file_stem()?.to_string_lossy();
    let posters_dir = parent.join("Extras");

    // Closure qui cherche un fichier d'abord dans parent, puis dans Extras/
    let find = |filename: &str| -> Option<std::path::PathBuf> {
        let p = parent.join(filename);
        if p.exists() { return Some(p); }
        if posters_dir.exists() {
            let p2 = posters_dir.join(filename);
            if p2.exists() { return Some(p2); }
        }
        None
    };

    if let Some((series_title, season_num)) = extract_series_info(&stem) {
        // Priorité : {titre}_seasonXX_poster.jpg
        if let Some(p) = find(&format!("{}_season{:02}_poster.jpg", series_title, season_num)) { return Some(p); }
        // Compat : seasonXX_poster.jpg
        if let Some(p) = find(&format!("season{:02}_poster.jpg", season_num)) { return Some(p); }
        // Compat ancienne : seasonXX-poster.jpg
        if let Some(p) = find(&format!("season{:02}-poster.jpg", season_num)) { return Some(p); }
        return None;
    }

    // Film
    if let Some(p) = find(&format!("{}_poster.jpg", stem)) { return Some(p); }
    if let Some(p) = find(&format!("{}-poster.jpg", stem)) { return Some(p); }
    if let Some(p) = find("poster.jpg") { return Some(p); }
    None
}

/// 4. Injection Poster uniquement (film ou série)
/// Pour une série : injecte seasonXX-poster.jpg
/// Pour un film   : injecte stem-poster.jpg / poster.jpg
/// Pas de fanart ni clearlogo
pub fn ajouter_images_mkv(mkv_path: &Path) -> Result<(), String> {
    let poster_path = match resolve_poster(mkv_path) {
        Some(p) => p,
        None => return Ok(()), // Pas de poster trouvé, rien à faire
    };
    let mime = if poster_path.extension().map_or(false, |e| e == "png") {
        "image/png"
    } else {
        "image/jpeg"
    };
    let status = binaries::silent_cmd(binaries::get_mkvpropedit())
        .arg(mkv_path)
        .args(["--attachment-name", "cover",
               "--attachment-mime-type", mime,
               "--add-attachment", poster_path.to_str().unwrap()])
        .status().map_err(|e| e.to_string())?;
    if status.success() { Ok(()) } else { Err("Erreur injection poster".into()) }
}

/// 5a. Injection NFO + Poster (orchestrateur — remplace l'ancien tag_inject_nfo_and_poster)
pub fn injecter_nfo_et_poster(mkv_path: &Path) -> Result<(), String> {
    // NFO
    if let Some(nfo) = resolve_nfo(mkv_path) {
        appliquer_tags(mkv_path, &nfo)?;
    }
    // Poster uniquement
    ajouter_images_mkv(mkv_path)?;
    Ok(())
}
/// 5b. Supprimer TOUS les tags et TOUTES les pièces jointes (Reset total)
pub fn supprimer_tous_tags(mkv_path: &Path) -> Result<(), String> {
    let xml_vide = "<?xml version=\"1.0\"?>\n<Tags>\n</Tags>";
    let temp_xml = "temp_reset.xml";
    std::fs::write(temp_xml, xml_vide).map_err(|e| e.to_string())?;
    let status = binaries::silent_cmd(binaries::get_mkvpropedit())
        .args([
            mkv_path.to_str().unwrap(),
            "--tags", &format!("global:{}", temp_xml),
            "--edit", "info", "--set", "title=",
            "--delete-attachment", "mime-type:image/jpeg",
            "--delete-attachment", "mime-type:image/png",
        ])
        .status()
        .map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(temp_xml);
    if status.success() { Ok(()) } else { Err("Erreur reset".into()) }
}