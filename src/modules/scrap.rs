#![allow(dead_code)]
use chrono::Local;
use regex::Regex;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
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

pub fn save_metadata(input_path: PathBuf, data: ScrapeResult) {
    let _ = dotenvy::dotenv();
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let year = data.release_date.split('-').next().unwrap_or("").to_string();
    let tag = if data.is_series { "tvshow" } else { "movie" };
    let filename = input_path.file_name().unwrap_or_default().to_string_lossy();
    let base_name = input_path.file_stem().unwrap_or_default().to_string_lossy().to_string();
    let parent_dir = input_path.parent().unwrap_or(&input_path);

    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n");
    xml.push_str(&format!("<!--created on {} by oxyon for KODI-->\n", now));
    xml.push_str(&format!("<{}>\n", tag));
    xml.push_str(&format!("  <title>{}</title>\n", escape_xml(&data.title)));
    xml.push_str(&format!("  <originaltitle>{}</originaltitle>\n", escape_xml(&data.original_title)));
    xml.push_str("  <sorttitle/>\n");
    xml.push_str("  <epbookmark/>\n");
    xml.push_str(&format!("  <year>{}</year>\n", year));

    // Ratings — arrondi à 1 décimale
    let tmdb_rating = (data.vote_average * 10.0).round() / 10.0;
    xml.push_str("  <ratings>\n    <rating default=\"false\" max=\"10\" name=\"themoviedb\">\n");
    xml.push_str(&format!("      <value>{}</value>\n", tmdb_rating));
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

    // Clearlogo (référence locale, sera téléchargé séparément)
    // On ajoute la référence seulement si fanart.tv a un clearlogo
    // (sera rempli par le thread de téléchargement)

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
        xml.push_str(&format!("    <n>{}</n>\n", escape_xml(&actor.name)));
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
        xml.push_str(&format!("    <n>{}</n>\n", escape_xml(&p.name)));
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

    let _ = fs::write(input_path.with_extension("nfo"), xml);

    // Téléchargement images en parallèle
    let client = Client::new();

    if let Some(ref path) = data.poster_path {
        let poster_url = format!("https://image.tmdb.org/t/p/original{}", path);
        let out_path = parent_dir.join(format!("{}-poster.jpg", base_name));
        let client_clone = client.clone();
        std::thread::spawn(move || {
            if let Ok(res) = client_clone.get(poster_url).send() {
                if let Ok(bytes) = res.bytes() {
                    let _ = fs::write(out_path, bytes);
                }
            }
        });
    }

    if let Some(ref path) = data.backdrop_path {
        let fanart_url = format!("https://image.tmdb.org/t/p/original{}", path);
        let out_path = parent_dir.join(format!("{}-fanart.jpg", base_name));
        let client_clone = client.clone();
        std::thread::spawn(move || {
            if let Ok(res) = client_clone.get(fanart_url).send() {
                if let Ok(bytes) = res.bytes() {
                    let _ = fs::write(out_path, bytes);
                }
            }
        });
    }

    // Clearlogo via Fanart.tv
    let fanart_api_key = match std::env::var("FANART_API_KEY") {
        Ok(k) => k,
        Err(_) => return,
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

    let out_logo_path = parent_dir.join(format!("{}-clearlogo.png", base_name));
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

pub fn search_tmdb(query: &str, is_series: bool) -> Result<Vec<ScrapeResult>, String> {
    let _ = dotenvy::dotenv();
    let api_key = std::env::var("TMDB_API_KEY").map_err(|_| "TMDB_API_KEY manquante".to_string())?;

    let client = Client::builder()
        .user_agent("OXYON/2.1")
        .build()
        .map_err(|e| e.to_string())?;

    let re_year = Regex::new(r"\b(19|20)\d{2}\b").unwrap();
    let clean_query = re_year
        .replace_all(query, "")
        .to_string()
        .replace(".", " ")
        .replace("_", " ")
        .replace("-", " ");

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
                // External IDs
                let imdb_id = d["external_ids"]["imdb_id"].as_str().map(|s| s.to_string());
                let wikidata_id = d["external_ids"]["wikidata_id"].as_str().map(|s| s.to_string());
                let tvdb_id = d["external_ids"]["tvdb_id"].as_i64();

                // Actors — tous, pas de limite
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

                // Director
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

                // Writers
                let mut writers = Vec::new();
                if let Some(crew) = d["credits"]["crew"].as_array() {
                    for c in crew {
                        let job = c["job"].as_str().unwrap_or("");
                        if job == "Screenplay" || job == "Writer" || job == "Story" {
                            let w = Writer {
                                name: c["name"].as_str().unwrap_or("").to_string(),
                                tmdbid: c["id"].as_i64().unwrap_or(0),
                            };
                            if !writers.iter().any(|existing: &Writer| existing.tmdbid == w.tmdbid) {
                                writers.push(w);
                            }
                        }
                    }
                }

                // Producers
                let mut producers = Vec::new();
                if let Some(crew) = d["credits"]["crew"].as_array() {
                    for c in crew {
                        let job = c["job"].as_str().unwrap_or("");
                        if job == "Producer" || job == "Executive Producer"
                            || job == "Co-Producer" || job == "Associate Producer"
                        {
                            producers.push(Producer {
                                name: c["name"].as_str().unwrap_or("").to_string(),
                                role: job.to_string(),
                                thumb: c["profile_path"].as_str()
                                    .map(|s| format!("https://image.tmdb.org/t/p/h632{}", s)),
                                profile: format!("https://www.themoviedb.org/person/{}", c["id"].as_i64().unwrap_or(0)),
                                tmdbid: c["id"].as_i64().unwrap_or(0),
                            });
                        }
                    }
                }

                // Runtime
                let runtime = if is_series {
                    d["episode_run_time"].as_array()
                        .and_then(|a| a.first())
                        .and_then(|v| v.as_u64())
                        .unwrap_or(45)
                } else {
                    d["runtime"].as_u64().unwrap_or(0)
                };

                // Country
                let country = d["production_countries"].as_array()
                    .map(|arr| arr.iter()
                        .filter_map(|c| c["iso_3166_1"].as_str())
                        .collect::<Vec<_>>()
                        .join(", "))
                    .unwrap_or_default();

                // Certification
                let certification = if is_series {
                    d["content_ratings"]["results"].as_array()
                        .and_then(|arr| arr.iter().find(|c| c["iso_3166_1"].as_str() == Some("FR")))
                        .and_then(|c| c["rating"].as_str())
                        .map(|r| format!("FR:{}", r))
                } else {
                    d["release_dates"]["results"].as_array()
                        .and_then(|arr| arr.iter().find(|c| c["iso_3166_1"].as_str() == Some("FR")))
                        .and_then(|c| c["release_dates"].as_array())
                        .and_then(|releases| releases.iter()
                            .find(|r| r["certification"].as_str().map_or(false, |s| !s.is_empty())))
                        .and_then(|r| r["certification"].as_str())
                        .map(|r| format!("FR:{}", r))
                };

                // Tags (keywords)
                let keywords_key = if is_series { "results" } else { "keywords" };
                let tags = d["keywords"][keywords_key].as_array()
                    .map(|arr| arr.iter()
                        .filter_map(|kw| kw["name"].as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>())
                    .unwrap_or_default();

                // Trailer YouTube
                let trailer_key = d["videos"]["results"].as_array()
                    .and_then(|arr| arr.iter()
                        .find(|v| v["site"].as_str() == Some("YouTube") && v["type"].as_str() == Some("Trailer")))
                    .and_then(|v| v["key"].as_str())
                    .map(|s| s.to_string());

                // Languages
                let languages = d["spoken_languages"].as_array()
                    .map(|arr| arr.iter()
                        .filter_map(|l| l["english_name"].as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>())
                    .unwrap_or_default();

                // Studios — networks pour séries, production_companies pour films
                let studios = if is_series {
                    d["networks"].as_array()
                        .map(|arr| arr.iter()
                            .filter_map(|s| s["name"].as_str().map(|n| n.to_string()))
                            .collect::<Vec<_>>())
                        .unwrap_or_default()
                } else {
                    d["production_companies"].as_array()
                        .map(|arr| arr.iter()
                            .filter_map(|s| s["name"].as_str().map(|n| n.to_string()))
                            .collect::<Vec<_>>())
                        .unwrap_or_default()
                };

                list.push(ScrapeResult {
                    id,
                    title: d[if is_series { "name" } else { "title" }]
                        .as_str().unwrap_or("Inconnu").to_string(),
                    original_title: d[if is_series { "original_name" } else { "original_title" }]
                        .as_str().unwrap_or("").to_string(),
                    overview: d["overview"].as_str().unwrap_or("").to_string(),
                    poster_path: d["poster_path"].as_str().map(|s| s.to_string()),
                    backdrop_path: d["backdrop_path"].as_str().map(|s| s.to_string()),
                    release_date: d[if is_series { "first_air_date" } else { "release_date" }]
                        .as_str().unwrap_or("").to_string(),
                    vote_average: d["vote_average"].as_f64().unwrap_or(0.0),
                    vote_count: d["vote_count"].as_u64().unwrap_or(0),
                    runtime: runtime as u32,
                    tagline: d["tagline"].as_str().unwrap_or("").to_string(),
                    genres: d["genres"].as_array()
                        .unwrap_or(&vec![]).iter()
                        .map(|g| g["name"].as_str().unwrap_or("").to_string())
                        .collect(),
                    studios,
                    actors,
                    director,
                    director_tmdbid,
                    writers,
                    producers,
                    imdb_id,
                    wikidata_id,
                    tvdb_id,
                    country,
                    certification,
                    tags,
                    trailer_key,
                    languages,
                    is_series,
                });
            }
        }
    }
    Ok(list)
}