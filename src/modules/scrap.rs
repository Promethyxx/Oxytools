use chrono::Local;
use regex::Regex;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

// Helper pour l'échappement XML
fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ScrapeResult {
    pub id: i64,
    pub title: String,
    pub original_title: String,
    pub overview: String,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub release_date: String,
    pub vote_average: f64,
    pub vote_count: u64,
    pub genres: Vec<String>,
    pub studios: Vec<String>,
    pub actors: Vec<Actor>,
    pub director: Option<String>,
    pub director_tmdbid: Option<i64>,
    pub writers: Vec<Writer>,
    pub producers: Vec<Producer>,
    pub runtime: u32,
    pub tagline: String,
    pub imdb_id: Option<String>,
    pub wikidata_id: Option<String>,
    pub tvdb_id: Option<i64>,
    pub country: String,
    pub certification: Option<String>,
    pub tags: Vec<String>,
    pub trailer_key: Option<String>,
    pub languages: Vec<String>,
    pub is_series: bool,
    pub seasons: Vec<Season>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Season {
    pub number: u32,
    pub name: String,
    pub overview: String,
    pub poster_path: Option<String>,
    pub air_date: String,
    pub episode_count: u32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Actor {
    pub name: String,
    pub role: String,
    pub thumb: Option<String>,
    pub profile: String,
    pub id: i64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Writer {
    pub name: String,
    pub tmdbid: i64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Producer {
    pub name: String,
    pub role: String,
    pub thumb: Option<String>,
    pub profile: String,
    pub tmdbid: i64,
}

pub fn download_image_bytes(poster_path: &str) -> Option<Vec<u8>> {
    let url = format!("https://image.tmdb.org/t/p/w92{}", poster_path);
    let client = Client::new();
    if let Ok(res) = client.get(url).send() {
        if let Ok(bytes) = res.bytes() {
            return Some(bytes.to_vec());
        }
    }
    None
}

/// Parse le nom d'un fichier et retourne (titre_nettoyé, numéro_de_saison)
/// Supporte S01E01, s01e01, 01x01, 1x01
/// Retourne None si aucun pattern série détecté.
pub fn extract_series_info(filename: &str) -> Option<(String, u32)> {
    let re = Regex::new(
        r"(?i)^(.+?)[\s.\-_]+(?:S(\d{1,2})E\d{1,2}|(\d{1,2})x\d{1,2})"
    ).unwrap();
    if let Some(caps) = re.captures(filename) {
        let raw_title = caps.get(1)?.as_str();
        let season_num: u32 = if let Some(s) = caps.get(2) {
            s.as_str().parse().unwrap_or(1)
        } else if let Some(s) = caps.get(3) {
            s.as_str().parse().unwrap_or(1)
        } else {
            1
        };
        // Nettoyer le titre : remplacer . _ - par espaces, trim
        let title = raw_title
            .replace('.', " ")
            .replace('_', " ")
            .replace('-', " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        Some((title, season_num))
    } else {
        None
    }
}

/// Collecte les numéros de saison uniques présents dans une liste de fichiers.
/// Si `paths` contient un dossier, le scanne récursivement.
pub fn collect_season_numbers(paths: &[PathBuf]) -> BTreeSet<u32> {
    let mut seasons = BTreeSet::new();
    for path in paths {
        if path.is_dir() {
            if let Ok(entries) = fs::read_dir(path) {
                let subpaths: Vec<PathBuf> = entries
                    .filter_map(|e| e.ok().map(|e| e.path()))
                    .collect();
                seasons.extend(collect_season_numbers(&subpaths));
            }
        } else {
            let stem = path.file_stem().unwrap_or_default().to_string_lossy();
            if let Some((_, n)) = extract_series_info(&stem) {
                seasons.insert(n);
            }
        }
    }
    seasons
}

/// Scanne récursivement un dossier et retourne tous les fichiers.
fn walk_dir(dir: &PathBuf) -> Vec<PathBuf> {
    let mut result = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                result.extend(walk_dir(&path));
            } else {
                result.push(path);
            }
        }
    }
    result
}

/// Détecte si une liste de fichiers/dossiers correspond à une série.
/// Retourne (titre_nettoyé, saisons_présentes) ou None si pas de pattern série.
pub fn detect_series(paths: &[PathBuf]) -> Option<(String, BTreeSet<u32>)> {
    // Aplatir : dossiers → tous leurs fichiers récursivement
    let mut all_files: Vec<PathBuf> = Vec::new();
    for path in paths {
        if path.is_dir() {
            all_files.extend(walk_dir(path));
        } else {
            all_files.push(path.clone());
        }
    }

    // Chercher le premier fichier qui matche et collecter toutes les saisons
    let mut title_found: Option<String> = None;
    let mut seasons = BTreeSet::new();
    for file in &all_files {
        let stem = file.file_stem().unwrap_or_default().to_string_lossy();
        if let Some((title, season_num)) = extract_series_info(&stem) {
            if title_found.is_none() {
                title_found = Some(title);
            }
            seasons.insert(season_num);
        }
    }

    if let Some(title) = title_found {
        if !seasons.is_empty() {
            return Some((title, seasons));
        }
    }
    None
}

pub fn save_metadata(input_path: PathBuf, data: ScrapeResult, fanart_key: &str, fetch_fanart: bool, fetch_clearlogo: bool) {
    let _ = dotenvy::dotenv();
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let year = data.release_date.split('-').next().unwrap_or("").to_string();
    let tag = if data.is_series { "tvshow" } else { "movie" };
    let filename = input_path.file_name().unwrap_or_default().to_string_lossy();
    let base_name = input_path.file_stem().unwrap_or_default().to_string_lossy().to_string();
    let parent_dir = input_path.parent().unwrap_or(&input_path);

    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n");
    xml.push_str(&format!("<!--created on {} by oxytools for KODI-->\n", now));
    xml.push_str(&format!("<{}>\n", tag));
    xml.push_str(&format!("  <title>{}</title>\n", escape_xml(&data.title)));
    xml.push_str(&format!("  <originaltitle>{}</originaltitle>\n", escape_xml(&data.original_title)));
    xml.push_str("  <sorttitle/>\n");
    xml.push_str("  <epbookmark/>\n");
    xml.push_str(&format!("  <year>{}</year>\n", year));

    // Ratings — arrondi à 1 décimale
    let tmdb_rating = (data.vote_average * 10.0).round() / 10.0;
    xml.push_str("  <ratings>\n    <rating default=\"false\" max=\"10\" name=\"themoviedb\">\n");
    xml.push_str(&format!("      <value>{:.1}</value>\n", tmdb_rating));
    xml.push_str(&format!("      <votes>{}</votes>\n", data.vote_count));
    xml.push_str("    </rating>\n  </ratings>\n");

    xml.push_str("  <userrating>0</userrating>\n  <top250>0</top250>\n");
    xml.push_str("  <set/>\n");
    xml.push_str(&format!("  <plot>{}</plot>\n", escape_xml(&data.overview)));
    xml.push_str(&format!("  <outline>{}</outline>\n", escape_xml(&data.overview)));
    xml.push_str(&format!("  <tagline>{}</tagline>\n", escape_xml(&data.tagline)));
    xml.push_str(&format!("  <runtime>{}</runtime>\n", data.runtime));

    // Poster
    if let Some(poster) = &data.poster_path {
        xml.push_str(&format!(
            "  <thumb aspect=\"poster\">https://image.tmdb.org/t/p/original{}</thumb>\n", poster
        ));
    }

    // Clearlogo local (téléchargé via fanart.tv)
    xml.push_str(&format!("  <thumb aspect=\"clearlogo\">{}_clearlogo.png</thumb>\n", base_name));

    // Fanart
    if let Some(backdrop) = &data.backdrop_path {
        xml.push_str("  <fanart>\n");
        xml.push_str(&format!(
            "    <thumb>https://image.tmdb.org/t/p/original{}</thumb>\n", backdrop
        ));
        xml.push_str("  </fanart>\n");
    }

    // Certification
    if let Some(cert) = &data.certification {
        if cert != "N/A" {
            xml.push_str(&format!("  <mpaa>{}</mpaa>\n", escape_xml(cert)));
            xml.push_str(&format!("  <certification>{}</certification>\n", escape_xml(cert)));
        }
    }

    // IDs
    if let Some(imdb) = &data.imdb_id {
        xml.push_str(&format!("  <id>{}</id>\n", escape_xml(imdb)));
    }
    xml.push_str(&format!("  <tmdbid>{}</tmdbid>\n", data.id));

    // UniqueIDs
    xml.push_str(&format!("  <uniqueid default=\"false\" type=\"tmdb\">{}</uniqueid>\n", data.id));
    if let Some(imdb) = &data.imdb_id {
        xml.push_str(&format!("  <uniqueid default=\"true\" type=\"imdb\">{}</uniqueid>\n", escape_xml(imdb)));
    }
    if let Some(tvdb) = data.tvdb_id {
        xml.push_str(&format!("  <uniqueid default=\"false\" type=\"tvdb\">{}</uniqueid>\n", tvdb));
    }
    if let Some(wiki) = &data.wikidata_id {
        xml.push_str(&format!("  <uniqueid default=\"false\" type=\"wikidata\">{}</uniqueid>\n", escape_xml(wiki)));
    }

    // Country
    if !data.country.is_empty() {
        xml.push_str(&format!("  <country>{}</country>\n", escape_xml(&data.country)));
    }
    xml.push_str("  <status/>\n  <code/>\n");
    xml.push_str(&format!("  <premiered>{}</premiered>\n", data.release_date));
    xml.push_str("  <watched>false</watched>\n  <playcount>0</playcount>\n");

    // Genres
    for genre in &data.genres {
        xml.push_str(&format!("  <genre>{}</genre>\n", escape_xml(genre)));
    }
    // Studios
    for studio in &data.studios {
        xml.push_str(&format!("  <studio>{}</studio>\n", escape_xml(studio)));
    }
    // Writers (credits)
    for w in &data.writers {
        xml.push_str(&format!("  <credits tmdbid=\"{}\">{}</credits>\n", w.tmdbid, escape_xml(&w.name)));
    }
    // Director
    if let Some(d) = &data.director {
        if let Some(did) = data.director_tmdbid {
            xml.push_str(&format!("  <director tmdbid=\"{}\">{}</director>\n", did, escape_xml(d)));
        } else {
            xml.push_str(&format!("  <director>{}</director>\n", escape_xml(d)));
        }
    }
    // Tags (keywords)
    for t in &data.tags {
        xml.push_str(&format!("  <tag>{}</tag>\n", escape_xml(t)));
    }
    // Actors — tous, pas limité à 15
    for actor in &data.actors {
        xml.push_str("  <actor>\n");
        xml.push_str(&format!("    <name>{}</name>\n", escape_xml(&actor.name)));
        xml.push_str(&format!("    <role>{}</role>\n", escape_xml(&actor.role)));
        if let Some(t) = &actor.thumb {
            xml.push_str(&format!("    <thumb>{}</thumb>\n", t));
        }
        xml.push_str(&format!("    <profile>{}</profile>\n", escape_xml(&actor.profile)));
        xml.push_str(&format!("    <tmdbid>{}</tmdbid>\n", actor.id));
        xml.push_str("  </actor>\n");
    }
    // Producers
    for p in &data.producers {
        xml.push_str(&format!("  <producer tmdbid=\"{}\">\n", p.tmdbid));
        xml.push_str(&format!("    <name>{}</name>\n", escape_xml(&p.name)));
        xml.push_str(&format!("    <role>{}</role>\n", escape_xml(&p.role)));
        if let Some(t) = &p.thumb {
            xml.push_str(&format!("    <thumb>{}</thumb>\n", t));
        }
        xml.push_str(&format!("    <profile>{}</profile>\n", escape_xml(&p.profile)));
        xml.push_str("  </producer>\n");
    }
    // Trailer
    if let Some(key) = &data.trailer_key {
        xml.push_str(&format!("  <trailer>plugin://plugin.video.youtube/play/?video_id={}</trailer>\n", key));
    }
    // Languages
    if !data.languages.is_empty() {
        xml.push_str(&format!("  <languages>{}</languages>\n", escape_xml(&data.languages.join(", "))));
    }
    // Date added
    xml.push_str(&format!("  <dateadded>{}</dateadded>\n", now));
    // Fileinfo (vide pour l'instant — pourrait être rempli par ffprobe)
    xml.push_str("  <fileinfo>\n    <streamdetails>\n    </streamdetails>\n  </fileinfo>\n");
    xml.push_str(&format!("  <original_filename>{}</original_filename>\n", escape_xml(&filename)));
    xml.push_str(&format!("</{}>\n", tag));

    // Tout dans Extras/
    let client = Client::new();
    let posters_dir = parent_dir.join("Extras");
    let _ = fs::create_dir_all(&posters_dir);

    let _ = fs::write(posters_dir.join(format!("{}.nfo", base_name)), xml);

    if let Some(ref path) = data.poster_path {
        let poster_url = format!("https://image.tmdb.org/t/p/original{}", path);
        let out_path = posters_dir.join(format!("{}_poster.jpg", base_name));
        let client_clone = client.clone();
        std::thread::spawn(move || {
            if let Ok(res) = client_clone.get(poster_url).send() {
                if let Ok(bytes) = res.bytes() {
                    let _ = fs::write(out_path, bytes);
                }
            }
        });
    }

    if fetch_fanart {
        if let Some(ref path) = data.backdrop_path {
            let fanart_url = format!("https://image.tmdb.org/t/p/original{}", path);
            let out_path = posters_dir.join(format!("{}_fanart.jpg", base_name));
            let client_clone = client.clone();
            std::thread::spawn(move || {
                if let Ok(res) = client_clone.get(fanart_url).send() {
                    if let Ok(bytes) = res.bytes() {
                        let _ = fs::write(out_path, bytes);
                    }
                }
            });
        }
    }

    if !fetch_clearlogo { return; }
    let fanart_api_key = if fanart_key.is_empty() {
        match std::env::var("FANART_API_KEY") {
            Ok(k) => k,
            Err(_) => return,
        }
    } else {
        fanart_key.to_string()
    };

    let tmdb_id = data.id;
    let tvdb_id = data.tvdb_id;
    let is_series = data.is_series;
    let logo_url = if is_series {
        if let Some(tvdb) = tvdb_id {
            format!("https://webservice.fanart.tv/v3/tv/{}?api_key={}", tvdb, fanart_api_key)
        } else {
            format!("https://webservice.fanart.tv/v3/tv/{}?api_key={}", tmdb_id, fanart_api_key)
        }
    } else {
        format!("https://webservice.fanart.tv/v3/movies/{}?api_key={}", tmdb_id, fanart_api_key)
    };

    let out_logo_path = posters_dir.join(format!("{}_clearlogo.png", base_name));
    std::thread::spawn(move || {
        if let Ok(res) = client.get(&logo_url).send() {
            if let Ok(json) = res.json::<Value>() {
                let logo_path = if is_series {
                    json["hdtvlogo"]
                        .as_array()
                        .or_else(|| json["clearlogo"].as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|v| v["url"].as_str())
                } else {
                    json["hdmovielogo"]
                        .as_array()
                        .or_else(|| json["hdclearlogo"].as_array())
                        .or_else(|| json["movielogo"].as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|v| v["url"].as_str())
                };

                if let Some(url) = logo_path {
                    if let Ok(img_res) = Client::new().get(url).send() {
                        if let Ok(bytes) = img_res.bytes() {
                            let _ = fs::write(out_logo_path, bytes);
                        }
                    }
                }
            }
        }
    });
}

/// Sauvegarde tvshow.nfo + un seasonXX.nfo par saison détectée + les images.
/// `series_dir`      : dossier racine de la série
/// `detected_seasons`: numéros de saison détectés dans les fichiers déposés
pub fn save_series_metadata(
    series_dir: PathBuf,
    series_name: &str,
    data: ScrapeResult,
    detected_seasons: &BTreeSet<u32>,
    fanart_key: &str,
    fetch_fanart: bool,
    fetch_clearlogo: bool,
) {
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let year = data.release_date.split('-').next().unwrap_or("").to_string();
    let base_name = series_name.to_string();
    let tmdb_rating = (data.vote_average * 10.0).round() / 10.0;

    // ── Macro-like : construit le bloc commun header+ids+body dans un String ──
    macro_rules! push_common {
        ($xml:ident) => {
            $xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n");
            $xml.push_str(&format!("<!--created on {} by oxytools for KODI-->\n", now));
            $xml.push_str("<tvshow>\n");
            $xml.push_str(&format!("  <title>{}</title>\n", escape_xml(&data.title)));
            $xml.push_str(&format!("  <originaltitle>{}</originaltitle>\n", escape_xml(&data.original_title)));
            $xml.push_str("  <sorttitle/>\n");
            $xml.push_str(&format!("  <year>{}</year>\n", year));
            $xml.push_str("  <ratings>\n    <rating default=\"false\" max=\"10\" name=\"themoviedb\">\n");
            $xml.push_str(&format!("      <value>{:.1}</value>\n", tmdb_rating));
            $xml.push_str(&format!("      <votes>{}</votes>\n", data.vote_count));
            $xml.push_str("    </rating>\n  </ratings>\n");
            $xml.push_str("  <userrating>0</userrating>\n  <top250>0</top250>\n  <set/>\n");
            $xml.push_str(&format!("  <plot>{}</plot>\n", escape_xml(&data.overview)));
            $xml.push_str(&format!("  <outline>{}</outline>\n", escape_xml(&data.overview)));
            $xml.push_str(&format!("  <tagline>{}</tagline>\n", escape_xml(&data.tagline)));
            $xml.push_str(&format!("  <runtime>{}</runtime>\n", data.runtime));
        };
    }

    macro_rules! push_ids {
        ($xml:ident) => {
            if let Some(ref imdb) = data.imdb_id {
                $xml.push_str(&format!("  <id>{}</id>\n", escape_xml(imdb)));
            }
            $xml.push_str(&format!("  <tmdbid>{}</tmdbid>\n", data.id));
            $xml.push_str(&format!("  <uniqueid default=\"false\" type=\"tmdb\">{}</uniqueid>\n", data.id));
            if let Some(ref imdb) = data.imdb_id {
                $xml.push_str(&format!("  <uniqueid default=\"true\" type=\"imdb\">{}</uniqueid>\n", escape_xml(imdb)));
            }
            if let Some(tvdb) = data.tvdb_id {
                $xml.push_str(&format!("  <uniqueid default=\"false\" type=\"tvdb\">{}</uniqueid>\n", tvdb));
            }
            if let Some(ref wiki) = data.wikidata_id {
                $xml.push_str(&format!("  <uniqueid default=\"false\" type=\"wikidata\">{}</uniqueid>\n", escape_xml(wiki)));
            }
        };
    }

    macro_rules! push_body {
        ($xml:ident) => {
            if let Some(ref cert) = data.certification {
                if cert != "N/A" {
                    $xml.push_str(&format!("  <mpaa>{}</mpaa>\n", escape_xml(cert)));
                    $xml.push_str(&format!("  <certification>{}</certification>\n", escape_xml(cert)));
                }
            }
            if !data.country.is_empty() {
                $xml.push_str(&format!("  <country>{}</country>\n", escape_xml(&data.country)));
            }
            $xml.push_str("  <status/>\n  <code/>\n");
            $xml.push_str(&format!("  <premiered>{}</premiered>\n", data.release_date));
            $xml.push_str("  <watched>false</watched>\n  <playcount>0</playcount>\n");
            for genre in &data.genres {
                $xml.push_str(&format!("  <genre>{}</genre>\n", escape_xml(genre)));
            }
            for studio in &data.studios {
                $xml.push_str(&format!("  <studio>{}</studio>\n", escape_xml(studio)));
            }
            for w in &data.writers {
                $xml.push_str(&format!("  <credits tmdbid=\"{}\">{}</credits>\n", w.tmdbid, escape_xml(&w.name)));
            }
            if let Some(ref d) = data.director {
                if let Some(did) = data.director_tmdbid {
                    $xml.push_str(&format!("  <director tmdbid=\"{}\">{}</director>\n", did, escape_xml(d)));
                } else {
                    $xml.push_str(&format!("  <director>{}</director>\n", escape_xml(d)));
                }
            }
            for t in &data.tags {
                $xml.push_str(&format!("  <tag>{}</tag>\n", escape_xml(t)));
            }
            for actor in &data.actors {
                $xml.push_str("  <actor>\n");
                $xml.push_str(&format!("    <name>{}</name>\n", escape_xml(&actor.name)));
                $xml.push_str(&format!("    <role>{}</role>\n", escape_xml(&actor.role)));
                if let Some(ref t) = actor.thumb {
                    $xml.push_str(&format!("    <thumb>{}</thumb>\n", t));
                }
                $xml.push_str(&format!("    <profile>{}</profile>\n", escape_xml(&actor.profile)));
                $xml.push_str(&format!("    <tmdbid>{}</tmdbid>\n", actor.id));
                $xml.push_str("  </actor>\n");
            }
            for p in &data.producers {
                $xml.push_str(&format!("  <producer tmdbid=\"{}\">\n", p.tmdbid));
                $xml.push_str(&format!("    <name>{}</name>\n", escape_xml(&p.name)));
                $xml.push_str(&format!("    <role>{}</role>\n", escape_xml(&p.role)));
                if let Some(ref t) = p.thumb {
                    $xml.push_str(&format!("    <thumb>{}</thumb>\n", t));
                }
                $xml.push_str(&format!("    <profile>{}</profile>\n", escape_xml(&p.profile)));
                $xml.push_str("  </producer>\n");
            }
            if let Some(ref key) = data.trailer_key {
                $xml.push_str(&format!("  <trailer>plugin://plugin.video.youtube/play/?video_id={}</trailer>\n", key));
            }
            if !data.languages.is_empty() {
                $xml.push_str(&format!("  <languages>{}</languages>\n", escape_xml(&data.languages.join(", "))));
            }
            $xml.push_str(&format!("  <dateadded>{}</dateadded>\n", now));
            $xml.push_str("  <fileinfo>\n    <streamdetails>\n    </streamdetails>\n  </fileinfo>\n");
            $xml.push_str("</tvshow>\n");
        };
    }

    // ── Dossier Extras/ — créé en premier pour NFO et images ────────────────
    let extras_dir = series_dir.join("Extras");
    let _ = fs::create_dir_all(&extras_dir);

    // ── tvshow.nfo ───────────────────────────────────────────────────────────
    let mut xml = String::new();
    push_common!(xml);
    if let Some(ref poster) = data.poster_path {
        xml.push_str(&format!("  <thumb aspect=\"poster\">https://image.tmdb.org/t/p/original{}</thumb>\n", poster));
    }
    for season in data.seasons.iter().filter(|s| s.number > 0 && detected_seasons.contains(&s.number)) {
        if let Some(ref p) = season.poster_path {
            xml.push_str(&format!(
                "  <thumb aspect=\"poster\" type=\"season\" season=\"{}\">https://image.tmdb.org/t/p/original{}</thumb>\n",
                season.number, p
            ));
        }
    }
    xml.push_str(&format!("  <thumb aspect=\"clearlogo\">{}_clearlogo.png</thumb>\n", base_name));
    if let Some(ref backdrop) = data.backdrop_path {
        xml.push_str("  <fanart>\n");
        xml.push_str(&format!("    <thumb>https://image.tmdb.org/t/p/original{}</thumb>\n", backdrop));
        xml.push_str("  </fanart>\n");
    }
    push_ids!(xml);
    push_body!(xml);
    let _ = fs::write(extras_dir.join(format!("{}.nfo", base_name)), &xml);

    // ── seasonXX.nfo — un par saison détectée ────────────────────────────────
    for &season_num in detected_seasons {
        let season_data = data.seasons.iter().find(|s| s.number == season_num);
        let mut sxml = String::new();
        push_common!(sxml);
        if let Some(s) = season_data {
            if let Some(ref p) = s.poster_path {
                sxml.push_str(&format!(
                    "  <thumb aspect=\"poster\" type=\"season\" season=\"{}\">https://image.tmdb.org/t/p/original{}</thumb>\n",
                    season_num, p
                ));
            }
            if !s.overview.is_empty() {
                sxml.push_str(&format!("  <seasondesc>{}</seasondesc>\n", escape_xml(&s.overview)));
            }
            sxml.push_str(&format!("  <season>{}</season>\n", season_num));
            if !s.air_date.is_empty() {
                sxml.push_str(&format!("  <premiered>{}</premiered>\n", s.air_date));
            }
            sxml.push_str(&format!("  <episodecount>{}</episodecount>\n", s.episode_count));
        } else {
            sxml.push_str(&format!("  <season>{}</season>\n", season_num));
        }
        push_ids!(sxml);
        push_body!(sxml);
        let _ = fs::write(extras_dir.join(format!("{}_season{:02}.nfo", base_name, season_num)), &sxml);
    }

    // ── Images en parallèle → dossier Extras/ ──────────────────────────────
    let client = Client::new();
    // extras_dir déjà créé plus haut

    if let Some(ref path) = data.poster_path {
        let url = format!("https://image.tmdb.org/t/p/original{}", path);
        let out = extras_dir.join(format!("{}_poster.jpg", base_name));
        let c = client.clone();
        std::thread::spawn(move || {
            if let Ok(res) = c.get(url).send() {
                if let Ok(bytes) = res.bytes() { let _ = fs::write(out, bytes); }
            }
        });
    }
    if fetch_fanart {
        if let Some(ref path) = data.backdrop_path {
            let url = format!("https://image.tmdb.org/t/p/original{}", path);
            let out = extras_dir.join(format!("{}_fanart.jpg", base_name));
            let c = client.clone();
            std::thread::spawn(move || {
                if let Ok(res) = c.get(url).send() {
                    if let Ok(bytes) = res.bytes() { let _ = fs::write(out, bytes); }
                }
            });
        }
    }
    for season in data.seasons.iter().filter(|s| s.number > 0 && detected_seasons.contains(&s.number)) {
        if let Some(ref pp) = season.poster_path {
            let url = format!("https://image.tmdb.org/t/p/original{}", pp);
            let out = extras_dir.join(format!("{}_season{:02}_poster.jpg", base_name, season.number));
            let c = client.clone();
            std::thread::spawn(move || {
                if let Ok(res) = c.get(url).send() {
                    if let Ok(bytes) = res.bytes() { let _ = fs::write(out, bytes); }
                }
            });
        }
    }

    if !fetch_clearlogo { return; }
    let fanart_api_key = if fanart_key.is_empty() {
        match std::env::var("FANART_API_KEY") {
            Ok(k) => k,
            Err(_) => return,
        }
    } else {
        fanart_key.to_string()
    };
    let tmdb_id = data.id;
    let tvdb_id = data.tvdb_id;
    let logo_url = if let Some(tvdb) = tvdb_id {
        format!("https://webservice.fanart.tv/v3/tv/{}?api_key={}", tvdb, fanart_api_key)
    } else {
        format!("https://webservice.fanart.tv/v3/tv/{}?api_key={}", tmdb_id, fanart_api_key)
    };
    let out_logo = extras_dir.join(format!("{}_clearlogo.png", base_name));
    std::thread::spawn(move || {
        if let Ok(res) = client.get(&logo_url).send() {
            if let Ok(json) = res.json::<Value>() {
                let url = json["hdtvlogo"]
                    .as_array()
                    .or_else(|| json["clearlogo"].as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|v| v["url"].as_str());
                if let Some(u) = url {
                    if let Ok(img) = Client::new().get(u).send() {
                        if let Ok(bytes) = img.bytes() { let _ = fs::write(out_logo, bytes); }
                    }
                }
            }
        }
    });
}



/// Parse un objet JSON de détail TMDB (film ou série) en ScrapeResult.
fn parse_detail(d: &Value, id: i64, is_series: bool) -> Option<ScrapeResult> {
    let imdb_id = d["external_ids"]["imdb_id"].as_str().map(|s| s.to_string());
    let wikidata_id = d["external_ids"]["wikidata_id"].as_str().map(|s| s.to_string());
    let tvdb_id = d["external_ids"]["tvdb_id"].as_i64();

    let mut actors = Vec::new();
    if let Some(cast) = d["credits"]["cast"].as_array() {
        for a in cast.iter() {
            actors.push(Actor {
                name: a["name"].as_str().unwrap_or("").to_string(),
                role: a["character"].as_str().unwrap_or("").to_string(),
                thumb: a["profile_path"].as_str()
                    .map(|s| format!("https://image.tmdb.org/t/p/h632{}", s)),
                profile: format!("https://www.themoviedb.org/person/{}", a["id"].as_i64().unwrap_or(0)),
                id: a["id"].as_i64().unwrap_or(0),
            });
        }
    }

    let (director, director_tmdbid) = if !is_series {
        let dir = d["credits"]["crew"].as_array()
            .and_then(|crew| crew.iter().find(|m| m["job"] == "Director"));
        (
            dir.and_then(|m| m["name"].as_str()).map(|s| s.to_string()),
            dir.and_then(|m| m["id"].as_i64()),
        )
    } else {
        let creator = d["created_by"].as_array().and_then(|c| c.first());
        (
            creator.and_then(|m| m["name"].as_str()).map(|s| s.to_string()),
            creator.and_then(|m| m["id"].as_i64()),
        )
    };

    let mut writers = Vec::new();
    if let Some(crew) = d["credits"]["crew"].as_array() {
        for c in crew {
            let job = c["job"].as_str().unwrap_or("");
            if job == "Screenplay" || job == "Writer" || job == "Story" {
                let w = Writer { name: c["name"].as_str().unwrap_or("").to_string(), tmdbid: c["id"].as_i64().unwrap_or(0) };
                if !writers.iter().any(|existing: &Writer| existing.tmdbid == w.tmdbid) {
                    writers.push(w);
                }
            }
        }
    }

    let mut producers = Vec::new();
    if let Some(crew) = d["credits"]["crew"].as_array() {
        for c in crew {
            let job = c["job"].as_str().unwrap_or("");
            if job == "Producer" || job == "Executive Producer" || job == "Co-Producer" || job == "Associate Producer" {
                producers.push(Producer {
                    name: c["name"].as_str().unwrap_or("").to_string(),
                    role: job.to_string(),
                    thumb: c["profile_path"].as_str().map(|s| format!("https://image.tmdb.org/t/p/h632{}", s)),
                    profile: format!("https://www.themoviedb.org/person/{}", c["id"].as_i64().unwrap_or(0)),
                    tmdbid: c["id"].as_i64().unwrap_or(0),
                });
            }
        }
    }

    let runtime = if is_series {
        d["episode_run_time"].as_array().and_then(|a| a.first()).and_then(|v| v.as_u64()).unwrap_or(45)
    } else {
        d["runtime"].as_u64().unwrap_or(0)
    };

    let country = d["production_countries"].as_array()
        .map(|arr| arr.iter().filter_map(|c| c["iso_3166_1"].as_str()).collect::<Vec<_>>().join(", "))
        .unwrap_or_default();

    let certification = if is_series {
        d["content_ratings"]["results"].as_array()
            .and_then(|arr| arr.iter().find(|c| c["iso_3166_1"].as_str() == Some("FR")))
            .and_then(|c| c["rating"].as_str()).map(|r| format!("FR:{}", r))
    } else {
        d["release_dates"]["results"].as_array()
            .and_then(|arr| arr.iter().find(|c| c["iso_3166_1"].as_str() == Some("FR")))
            .and_then(|c| c["release_dates"].as_array())
            .and_then(|releases| releases.iter().find(|r| r["certification"].as_str().map_or(false, |s| !s.is_empty())))
            .and_then(|r| r["certification"].as_str()).map(|r| format!("FR:{}", r))
    };

    let keywords_key = if is_series { "results" } else { "keywords" };
    let tags = d["keywords"][keywords_key].as_array()
        .map(|arr| arr.iter().filter_map(|kw| kw["name"].as_str().map(|s| s.to_string())).collect::<Vec<_>>())
        .unwrap_or_default();

    let trailer_key = d["videos"]["results"].as_array()
        .and_then(|arr| arr.iter().find(|v| v["site"].as_str() == Some("YouTube") && v["type"].as_str() == Some("Trailer")))
        .and_then(|v| v["key"].as_str()).map(|s| s.to_string());

    let languages = d["spoken_languages"].as_array()
        .map(|arr| arr.iter().filter_map(|l| l["english_name"].as_str().map(|s| s.to_string())).collect::<Vec<_>>())
        .unwrap_or_default();

    let studios = if is_series {
        d["networks"].as_array()
            .map(|arr| arr.iter().filter_map(|s| s["name"].as_str().map(|n| n.to_string())).collect::<Vec<_>>())
            .unwrap_or_default()
    } else {
        d["production_companies"].as_array()
            .map(|arr| arr.iter().filter_map(|s| s["name"].as_str().map(|n| n.to_string())).collect::<Vec<_>>())
            .unwrap_or_default()
    };

    let seasons = if is_series {
        d["seasons"].as_array()
            .map(|arr| arr.iter().filter_map(|s| {
                let number = s["season_number"].as_u64()? as u32;
                Some(Season {
                    number,
                    name: s["name"].as_str().unwrap_or("").to_string(),
                    overview: s["overview"].as_str().unwrap_or("").to_string(),
                    poster_path: s["poster_path"].as_str().map(|p| p.to_string()),
                    air_date: s["air_date"].as_str().unwrap_or("").to_string(),
                    episode_count: s["episode_count"].as_u64().unwrap_or(0) as u32,
                })
            }).collect())
            .unwrap_or_default()
    } else { vec![] };

    Some(ScrapeResult {
        id,
        title: d[if is_series { "name" } else { "title" }].as_str().unwrap_or("Inconnu").to_string(),
        original_title: d[if is_series { "original_name" } else { "original_title" }].as_str().unwrap_or("").to_string(),
        overview: d["overview"].as_str().unwrap_or("").to_string(),
        poster_path: d["poster_path"].as_str().map(|s| s.to_string()),
        backdrop_path: d["backdrop_path"].as_str().map(|s| s.to_string()),
        release_date: d[if is_series { "first_air_date" } else { "release_date" }].as_str().unwrap_or("").to_string(),
        vote_average: d["vote_average"].as_f64().unwrap_or(0.0),
        vote_count: d["vote_count"].as_u64().unwrap_or(0),
        runtime: runtime as u32,
        tagline: d["tagline"].as_str().unwrap_or("").to_string(),
        genres: d["genres"].as_array().unwrap_or(&vec![]).iter()
            .map(|g| g["name"].as_str().unwrap_or("").to_string()).collect(),
        studios, actors, director, director_tmdbid, writers, producers,
        imdb_id, wikidata_id, tvdb_id, country, certification, tags, trailer_key, languages,
        is_series, seasons,
    })
}

/// Récupère une fiche TMDB directement par ID.
/// Utilisé quand l'utilisateur entre un ID numérique ou une URL TMDB dans la recherche alternative.
pub fn fetch_by_tmdb_id(id: i64, is_series: bool, tmdb_key: &str) -> Result<Vec<ScrapeResult>, String> {
    let api_key = if tmdb_key.is_empty() {
        std::env::var("TMDB_API_KEY").map_err(|_| "TMDB_API_KEY manquante".to_string())?
    } else {
        tmdb_key.to_string()
    };
    let client = Client::builder().user_agent("OXYON/2.1").build().map_err(|e| e.to_string())?;
    let append = "credits,external_ids,keywords,videos,release_dates,content_ratings";
    let url = format!(
        "https://api.themoviedb.org/3/{}?api_key={}&language=fr-FR&append_to_response={}",
        if is_series { format!("tv/{}", id) } else { format!("movie/{}", id) },
        api_key, append
    );
    let d = client.get(&url).send().map_err(|e| e.to_string())?.json::<Value>().map_err(|e| e.to_string())?;
    if d["success"] == false {
        return Err(format!("ID {} introuvable", id));
    }
    match parse_detail(&d, id, is_series) {
        Some(r) => Ok(vec![r]),
        None => Ok(vec![]),
    }
}

pub fn search_tmdb(query: &str, is_series: bool, tmdb_key: &str) -> Result<Vec<ScrapeResult>, String> {
    let api_key = if tmdb_key.is_empty() {
        std::env::var("TMDB_API_KEY").map_err(|_| "TMDB_API_KEY manquante".to_string())?
    } else {
        tmdb_key.to_string()
    };

    let client = Client::builder()
        .user_agent("OXYON/2.1")
        .build()
        .map_err(|e| e.to_string())?;

    // Ne strip l'année que si elle est précédée d'un séparateur (espace, point, tiret, parenthèse)
    // pour ne pas écraser un titre comme "1917" ou "2001"
    let re_year = Regex::new(r"[\s.\-_\(](19|20)\d{2}[\s.\-_\)]*$").unwrap();
    let clean_query = re_year
        .replace(query, "")
        .to_string()
        .replace('.', " ")
        .replace('_', " ")
        .replace('-', " ");

    let url = if is_series {
        "https://api.themoviedb.org/3/search/tv"
    } else {
        "https://api.themoviedb.org/3/search/movie"
    };

    let res = client
        .get(url)
        .query(&[
            ("api_key", api_key.as_str()),
            ("language", "fr-FR"),
            ("query", clean_query.trim()),
        ])
        .send()
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .map_err(|e| e.to_string())?;

    let mut list = Vec::new();
    if let Some(results) = res["results"].as_array() {
        for r in results {
            let id = r["id"].as_i64().unwrap_or(0);

            // append_to_response étendu pour récupérer toutes les données
            let append = "credits,external_ids,keywords,videos,release_dates,content_ratings";
            let detail_url = format!(
                "https://api.themoviedb.org/3/{}?api_key={}&language=fr-FR&append_to_response={}",
                if is_series { format!("tv/{}", id) } else { format!("movie/{}", id) },
                api_key,
                append
            );

            if let Ok(d) = client
                .get(detail_url)
                .send()
                .and_then(|resp| resp.json::<Value>())
            {
                if let Some(result) = parse_detail(&d, id, is_series) {
                    list.push(result);
                }
            }
        }
    }
    Ok(list)
}
