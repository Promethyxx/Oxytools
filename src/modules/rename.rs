use std::path::{Path, PathBuf};
use std::io::{BufRead, BufReader, Write};

#[derive(Debug, Clone, PartialEq)]
pub enum CaseMode { Unchanged, Lower, Upper, Title, Sentence }
impl CaseMode {
    pub fn label(&self) -> &'static str {
        match self {
            CaseMode::Unchanged => "Unchanged", CaseMode::Lower => "lowercase",
            CaseMode::Upper => "UPPERCASE", CaseMode::Title => "Title Case",
            CaseMode::Sentence => "Sentence case",
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum NumPos { Prefix, Suffix }
#[derive(Debug, Clone, PartialEq)]
pub enum ExtMode { Unchanged, Lower, Upper, Replace, Remove }
impl ExtMode {
    pub fn label(&self) -> &'static str {
        match self {
            ExtMode::Unchanged => "Unchanged", ExtMode::Lower => "lowercase",
            ExtMode::Upper => "UPPERCASE", ExtMode::Replace => "Replace", ExtMode::Remove => "Remove",
        }
    }
}

// ── Position origin (début ou fin) ──────────────────────────────
#[derive(Debug, Clone, PartialEq)]
pub enum PosOrigin { FromStart, FromEnd }
impl Default for PosOrigin { fn default() -> Self { PosOrigin::FromStart } }
impl PosOrigin {
    pub fn label(&self) -> &'static str {
        match self { PosOrigin::FromStart => "From start", PosOrigin::FromEnd => "From end" }
    }
}

// ── Remplacement multiple ───────────────────────────────────────
#[derive(Debug, Clone)]
pub struct ReplaceRule {
    pub find: String,
    pub replace: String,
    pub use_regex: bool,
    pub enabled: bool,
}
impl ReplaceRule {
    pub fn new(find: impl Into<String>, replace: impl Into<String>) -> Self {
        Self { find: find.into(), replace: replace.into(), use_regex: false, enabled: true }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ReplaceList {
    pub rules: Vec<ReplaceRule>,
}

impl ReplaceList {
    pub fn new() -> Self { Self { rules: Vec::new() } }

    pub fn add(&mut self, find: impl Into<String>, replace: impl Into<String>) {
        self.rules.push(ReplaceRule::new(find, replace));
    }

    pub fn remove(&mut self, index: usize) {
        if index < self.rules.len() { self.rules.remove(index); }
    }

    pub fn move_up(&mut self, index: usize) {
        if index > 0 && index < self.rules.len() { self.rules.swap(index, index - 1); }
    }

    pub fn move_down(&mut self, index: usize) {
        if index + 1 < self.rules.len() { self.rules.swap(index, index + 1); }
    }

    /// Applique toutes les règles activées séquentiellement sur un nom.
    pub fn apply(&self, name: &str) -> String {
        let mut result = name.to_string();
        for rule in &self.rules {
            if !rule.enabled || rule.find.is_empty() { continue; }
            if rule.use_regex {
                if let Ok(re) = regex::Regex::new(&rule.find) {
                    result = re.replace_all(&result, rule.replace.as_str()).to_string();
                }
            } else {
                result = result.replace(&rule.find, &rule.replace);
            }
        }
        result
    }

    /// Sauvegarde la liste dans un fichier CSV (tab-separated).
    /// Format : find\treplace\tregex\tenabled
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let mut f = std::fs::File::create(path).map_err(|e| e.to_string())?;
        writeln!(f, "#find\treplace\tregex\tenabled").map_err(|e| e.to_string())?;
        for r in &self.rules {
            let find_esc = r.find.replace('\t', "\\t").replace('\n', "\\n");
            let repl_esc = r.replace.replace('\t', "\\t").replace('\n', "\\n");
            writeln!(f, "{}\t{}\t{}\t{}", find_esc, repl_esc, r.use_regex, r.enabled)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Charge une liste depuis un fichier CSV (tab-separated).
    pub fn load(path: &Path) -> Result<Self, String> {
        let f = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let reader = BufReader::new(f);
        let mut rules = Vec::new();
        for line in reader.lines() {
            let line = line.map_err(|e| e.to_string())?;
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') { continue; }
            let parts: Vec<&str> = line.splitn(4, '\t').collect();
            if parts.len() < 2 { continue; }
            let find = parts[0].replace("\\t", "\t").replace("\\n", "\n");
            let replace = parts[1].replace("\\t", "\t").replace("\\n", "\n");
            let use_regex = parts.get(2).map_or(false, |v| v.trim() == "true");
            let enabled = parts.get(3).map_or(true, |v| v.trim() != "false");
            rules.push(ReplaceRule { find, replace, use_regex, enabled });
        }
        Ok(Self { rules })
    }

    /// Charge depuis un fichier XML Ant Renamer.
    /// Parse les blocs <Set> et <CurrentList> dans <MultstrRepl>.
    /// Chaque CDATA contient des lignes "find\treplace".
    /// Si `set_name` est Some, charge le Set correspondant ; sinon charge <CurrentList>.
    pub fn load_ant_renamer_xml(path: &Path, set_name: Option<&str>) -> Result<Self, String> {
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        // Trouver le bon bloc CDATA
        let cdata = if let Some(name) = set_name {
            // Chercher <Set Name="X"> ... <![CDATA[...]]> ... </Set>
            let tag = format!("Set Name=\"{}\"", name);
            let set_start = content.find(&tag).ok_or_else(|| format!("Set '{}' not found", name))?;
            let after = &content[set_start..];
            Self::extract_cdata(after)?
        } else {
            // Chercher <CurrentList> ... <![CDATA[...]]> ... </CurrentList>
            if let Some(pos) = content.find("<CurrentList>") {
                let after = &content[pos..];
                Self::extract_cdata(after)?
            } else {
                // Fallback : premier Set trouvé
                let pos = content.find("<Set ").ok_or("No Set or CurrentList found in XML")?;
                let after = &content[pos..];
                Self::extract_cdata(after)?
            }
        };
        Self::parse_ant_cdata(&cdata)
    }

    /// Liste les noms de Sets disponibles dans un XML Ant Renamer.
    pub fn list_ant_renamer_sets(path: &Path) -> Result<Vec<String>, String> {
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let mut sets = Vec::new();
        let mut search_from = 0;
        while let Some(pos) = content[search_from..].find("Set Name=\"") {
            let abs = search_from + pos + 10; // après 'Set Name="'
            if let Some(end) = content[abs..].find('"') {
                sets.push(content[abs..abs + end].to_string());
            }
            search_from = abs + 1;
        }
        Ok(sets)
    }

    fn extract_cdata(text: &str) -> Result<String, String> {
        let start = text.find("<![CDATA[").ok_or("No CDATA found")? + 9;
        let end = text[start..].find("]]>").ok_or("Malformed CDATA")?;
        Ok(text[start..start + end].to_string())
    }

    fn parse_ant_cdata(cdata: &str) -> Result<Self, String> {
        let mut rules = Vec::new();
        for line in cdata.lines() {
            if line.is_empty() { continue; }
            // Ant Renamer utilise un tab comme séparateur find/replace
            let parts: Vec<&str> = line.splitn(2, '\t').collect();
            if parts.is_empty() { continue; }
            let find = parts[0].to_string();
            let replace = parts.get(1).map_or(String::new(), |s| s.to_string());
            if find.is_empty() { continue; }
            rules.push(ReplaceRule { find, replace, use_regex: false, enabled: true });
        }
        Ok(Self { rules })
    }
}
#[derive(Debug, Clone)]
pub struct RenameConfig {
    pub find: String, pub replace_with: String, pub use_regex: bool,
    pub insert_text: String, pub insert_pos: usize, pub insert_from_end: bool,
    pub delete_from: usize, pub delete_count: usize, pub delete_enabled: bool, pub delete_from_end: bool,
    pub num_enabled: bool, pub num_start: usize, pub num_step: usize,
    pub num_padding: usize, pub num_pos: NumPos, pub num_sep: String,
    pub case_mode: CaseMode,
    pub strip_leading_dots: bool, pub strip_trailing_spaces: bool,
    pub strip_double_spaces: bool, pub strip_chars: String,
    pub ext_mode: ExtMode, pub ext_new: String,
    /// Mode remplacement multiple (si true, utilise replace_list au lieu de find/replace_with)
    pub multi_replace: bool,
    pub replace_list: ReplaceList,
}
impl Default for RenameConfig {
    fn default() -> Self {
        Self {
            find: String::new(), replace_with: String::new(), use_regex: false,
            insert_text: String::new(), insert_pos: 0, insert_from_end: false,
            delete_from: 0, delete_count: 0, delete_enabled: false, delete_from_end: false,
            num_enabled: false, num_start: 1, num_step: 1, num_padding: 2,
            num_pos: NumPos::Suffix, num_sep: " ".into(),
            case_mode: CaseMode::Unchanged,
            strip_leading_dots: false, strip_trailing_spaces: true,
            strip_double_spaces: true, strip_chars: String::new(),
            ext_mode: ExtMode::Unchanged, ext_new: String::new(),
            multi_replace: false,
            replace_list: ReplaceList::new(),
        }
    }
}
pub fn preview(files: &[PathBuf], cfg: &RenameConfig) -> Vec<(PathBuf, String)> {
    files.iter().enumerate().map(|(i, p)| (p.clone(), compute_new_name(p, i, cfg))).collect()
}
fn compute_new_name(path: &Path, index: usize, cfg: &RenameConfig) -> String {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
    let ext_orig = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_string();
    let mut name = stem;

    // ── Remplacement (simple ou multiple) ───────────────────────
    if cfg.multi_replace {
        name = cfg.replace_list.apply(&name);
    } else if !cfg.find.is_empty() {
        name = name.replace(&cfg.find, &cfg.replace_with);
    }

    // ── Insertion (from_start ou from_end) ──────────────────────
    if !cfg.insert_text.is_empty() {
        let len = name.chars().count();
        let pos = if cfg.insert_from_end {
            len.saturating_sub(cfg.insert_pos)
        } else {
            cfg.insert_pos.min(len)
        };
        let mut chars: Vec<char> = name.chars().collect();
        for (j, c) in cfg.insert_text.chars().enumerate() { chars.insert(pos + j, c); }
        name = chars.into_iter().collect();
    }

    // ── Suppression (from_start ou from_end) ────────────────────
    if cfg.delete_enabled && cfg.delete_count > 0 {
        let chars: Vec<char> = name.chars().collect();
        let len = chars.len();
        let from = if cfg.delete_from_end {
            len.saturating_sub(cfg.delete_from + cfg.delete_count)
        } else {
            cfg.delete_from.min(len)
        };
        let to = (from + cfg.delete_count).min(len);
        name = chars[..from].iter().chain(chars[to..].iter()).collect();
    }

    if !cfg.strip_chars.is_empty() { name = name.chars().filter(|c| !cfg.strip_chars.contains(*c)).collect(); }
    if cfg.strip_leading_dots { name = name.trim_start_matches('.').to_string(); }
    if cfg.strip_double_spaces { while name.contains("  ") { name = name.replace("  ", " "); } }
    if cfg.strip_trailing_spaces { name = name.trim().to_string(); }
    name = match &cfg.case_mode {
        CaseMode::Unchanged => name,
        CaseMode::Lower => name.to_lowercase(),
        CaseMode::Upper => name.to_uppercase(),
        CaseMode::Title => name.split_whitespace().map(|w| {
            let mut c = w.chars();
            match c.next() { None => String::new(), Some(f) => f.to_uppercase().to_string() + &c.as_str().to_lowercase() }
        }).collect::<Vec<_>>().join(" "),
        CaseMode::Sentence => {
            let mut c = name.chars();
            match c.next() { None => String::new(), Some(f) => f.to_uppercase().to_string() + &c.as_str().to_lowercase() }
        }
    };
    if cfg.num_enabled {
        let n = cfg.num_start + index * cfg.num_step;
        let num_str = if cfg.num_padding > 0 { format!("{:0>width$}", n, width = cfg.num_padding) } else { n.to_string() };
        name = match cfg.num_pos {
            NumPos::Prefix => format!("{}{}{}", num_str, cfg.num_sep, name),
            NumPos::Suffix => format!("{}{}{}", name, cfg.num_sep, num_str),
        };
    }
    let ext = match &cfg.ext_mode {
        ExtMode::Unchanged => if ext_orig.is_empty() { String::new() } else { format!(".{}", ext_orig) },
        ExtMode::Lower => if ext_orig.is_empty() { String::new() } else { format!(".{}", ext_orig.to_lowercase()) },
        ExtMode::Upper => if ext_orig.is_empty() { String::new() } else { format!(".{}", ext_orig.to_uppercase()) },
        ExtMode::Replace => if cfg.ext_new.is_empty() { String::new() } else { format!(".{}", cfg.ext_new.trim_start_matches('.')) },
        ExtMode::Remove => String::new(),
    };
    format!("{}{}", name, ext)
}
pub struct RenameResult {
    pub original: PathBuf, pub new_name: String, pub success: bool, pub error: Option<String>,
}
pub fn apply_renames(previews: &[(PathBuf, String)]) -> Vec<RenameResult> {
    previews.iter().map(|(original, new_name)| {
        let parent = original.parent().unwrap_or(Path::new(""));
        let dest = parent.join(new_name);
        if dest == *original { return RenameResult { original: original.clone(), new_name: new_name.clone(), success: true, error: None }; }
        if dest.exists() { return RenameResult { original: original.clone(), new_name: new_name.clone(), success: false, error: Some(format!("Already exists: {}", new_name)) }; }
        match std::fs::rename(original, &dest) {
            Ok(_)  => RenameResult { original: original.clone(), new_name: new_name.clone(), success: true,  error: None },
            Err(e) => RenameResult { original: original.clone(), new_name: new_name.clone(), success: false, error: Some(e.to_string()) },
        }
    }).collect()
}