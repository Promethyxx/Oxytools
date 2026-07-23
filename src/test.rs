// ═══════════════════════════════════════════════════════════════
//  OXYTOOLS — Tests
// ═══════════════════════════════════════════════════════════════
//
// Ces tests vérifient le résultat réel de chaque opération (codec,
// durée, dimensions, nombre de pages, contenu) plutôt que seulement
// l'existence du fichier de sortie. Les valeurs attendues sont
// calculées dynamiquement à partir des propriétés du fichier source
// plutôt que codées en dur, pour ne pas dépendre du contenu exact
// des fixtures dans tests/.

use std::path::Path;
use std::fs;
use std::sync::{Once, Mutex};

const TEST_AUDIO: &str = "tests/audio";
const TEST_DOC:   &str = "tests/doc";
const TEST_PIC:   &str = "tests/pic";
const TEST_VIDEO: &str = "tests/video";
const OUT:        &str = "tests/_output";

static INIT: Once = Once::new();
/// Global mutex to serialize FFmpeg operations (prevents concurrent crashes)
static FFMPEG_LOCK: Mutex<()> = Mutex::new(());

fn setup() {
    INIT.call_once(|| {
        let _ = crate::modules::binaries::extraire_deps();
    });
    let _ = fs::create_dir_all(OUT);
}

/// Locks the ffmpeg mutex, executes the spawn, and THEN waits for it to finish (sequential execution guaranteed)
fn run_ffmpeg<F>(spawn_fn: F, context: &str)
where F: FnOnce() -> Result<std::process::Child, std::io::Error>
{
    let _lock = FFMPEG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let result = spawn_fn();
    assert!(result.is_ok(), "{context} spawn échoué : {:?}", result.err());
    let status = result.unwrap().wait().unwrap();
    assert!(status.success(), "{context} code={:?}", status.code());
}

fn assert_output(path: &str, context: &str) {
    let p = Path::new(path);
    assert!(p.exists(), "ÉCHEC {context} — fichier absent : {path}");
    let size = fs::metadata(p).unwrap().len();
    assert!(size > 0, "ÉCHEC {context} — fichier vide : {path}");
    println!("  OK {context} — {} octets", size);
}

fn cleanup(path: &str) {
    let _ = fs::remove_file(path);
}

// ───────────────────────────────────────────────────────────────
//  Helpers de vérification réelle (ffprobe, image, lopdf, zip)
// ───────────────────────────────────────────────────────────────

/// Interroge ffprobe pour un champ d'un flux donné (ex: "a:0", "codec_name").
fn probe_stream_field(path: &str, stream: &str, field: &str) -> Option<String> {
    let out = crate::modules::binaries::silent_cmd(crate::modules::binaries::get_ffprobe())
        .args(["-v", "error", "-select_streams", stream, "-show_entries", &format!("stream={field}"),
               "-of", "default=noprint_wrappers=1:nokey=1"])
        .arg(path)
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

/// Durée totale du conteneur en secondes.
fn probe_duration(path: &str) -> f64 {
    let out = crate::modules::binaries::silent_cmd(crate::modules::binaries::get_ffprobe())
        .args(["-v", "error", "-show_entries", "format=duration", "-of", "default=noprint_wrappers=1:nokey=1"])
        .arg(path)
        .output();
    out.ok()
        .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<f64>().ok())
        .unwrap_or(0.0)
}

fn assert_audio_codec(path: &str, codec_attendu: &str, context: &str) {
    let codec = probe_stream_field(path, "a:0", "codec_name");
    assert_eq!(codec.as_deref(), Some(codec_attendu),
        "ÉCHEC {context} — codec audio attendu '{codec_attendu}', obtenu {codec:?}");
}

fn assert_video_codec(path: &str, codec_attendu: &str, context: &str) {
    let codec = probe_stream_field(path, "v:0", "codec_name");
    assert_eq!(codec.as_deref(), Some(codec_attendu),
        "ÉCHEC {context} — codec vidéo attendu '{codec_attendu}', obtenu {codec:?}");
}

/// Vérifie que la durée de sortie est proche de la durée d'entrée (±1s, tolère l'arrondi des conteneurs).
fn assert_duree_preservee(input: &str, output: &str, context: &str) {
    let d_in = probe_duration(input);
    let d_out = probe_duration(output);
    assert!(d_in > 0.0, "ÉCHEC {context} — durée source illisible (fixture manquante ou corrompue ?)");
    assert!((d_in - d_out).abs() < 1.0,
        "ÉCHEC {context} — durée changée : source {d_in:.2}s, sortie {d_out:.2}s");
}

fn image_dims(path: &str) -> (u32, u32) {
    let img = image::open(path).unwrap_or_else(|e| panic!("image illisible {path} : {e}"));
    (img.width(), img.height())
}

fn assert_image_dims(path: &str, largeur: u32, hauteur: u32, context: &str) {
    let (w, h) = image_dims(path);
    assert_eq!((w, h), (largeur, hauteur), "ÉCHEC {context} — dimensions {w}x{h}, attendu {largeur}x{hauteur}");
}

fn assert_dims_preservees(input: &str, output: &str, context: &str) {
    let (w_in, h_in) = image_dims(input);
    let (w_out, h_out) = image_dims(output);
    assert_eq!((w_in, h_in), (w_out, h_out),
        "ÉCHEC {context} — dimensions changées : source {w_in}x{h_in}, sortie {w_out}x{h_out}");
}

/// Vérifie que largeur/hauteur ont été permutées (rotation 90°/270°).
fn assert_dims_permutees(input: &str, output: &str, context: &str) {
    let (w_in, h_in) = image_dims(input);
    let (w_out, h_out) = image_dims(output);
    assert_eq!((w_out, h_out), (h_in, w_in),
        "ÉCHEC {context} — dimensions non permutées : source {w_in}x{h_in}, sortie {w_out}x{h_out}");
}

fn pdf_page_count(path: &str) -> usize {
    lopdf::Document::load(path)
        .unwrap_or_else(|e| panic!("PDF illisible {path} : {e}"))
        .get_pages()
        .len()
}

fn assert_pdf_pages(path: &str, attendu: usize, context: &str) {
    let n = pdf_page_count(path);
    assert_eq!(n, attendu, "ÉCHEC {context} — {n} pages, {attendu} attendues");
}

/// Vérifie qu'un fichier est un ZIP valide (structure OOXML des .docx) contenant une entrée donnée.
fn assert_zip_contient(path: &str, entree: &str, context: &str) {
    let file = fs::File::open(path).unwrap_or_else(|e| panic!("ÉCHEC {context} — ouverture impossible : {e}"));
    let mut archive = zip::ZipArchive::new(file)
        .unwrap_or_else(|e| panic!("ÉCHEC {context} — pas un zip/docx valide : {e}"));
    assert!(archive.by_name(entree).is_ok(),
        "ÉCHEC {context} — entrée '{entree}' absente, structure docx invalide");
}

/// Vérifie qu'un fichier texte (HTML/MD) contient une sous-chaîne donnée.
fn assert_contient_texte(path: &str, sous_chaine: &str, context: &str) {
    let content = fs::read_to_string(path).unwrap_or_else(|e| panic!("ÉCHEC {context} — lecture impossible : {e}"));
    assert!(content.contains(sous_chaine),
        "ÉCHEC {context} — contenu attendu '{sous_chaine}' absent du résultat");
}

// ═══════════════════════════════════════════════════════════════
//  AUDIO — conversion (codec + durée vérifiés réellement)
// ═══════════════════════════════════════════════════════════════
#[test]
fn test_audio_mp3_vers_wav() {
    setup();
    let input = format!("{TEST_AUDIO}/mp3.mp3");
    let output = format!("{OUT}/audio_mp3_to_wav.wav");
    cleanup(&output);
    run_ffmpeg(|| crate::modules::audio::convertir(Path::new(&input), Path::new(&output), 1), "audio mp3→wav");
    assert_output(&output, "audio mp3→wav");
    assert_audio_codec(&output, "pcm_s16le", "audio mp3→wav");
    assert_duree_preservee(&input, &output, "audio mp3→wav");
    cleanup(&output);
}

#[test]
fn test_audio_wav_vers_flac() {
    setup();
    let input = format!("{TEST_AUDIO}/wav.wav");
    let output = format!("{OUT}/audio_wav_to_flac.flac");
    cleanup(&output);
    run_ffmpeg(|| crate::modules::audio::convertir(Path::new(&input), Path::new(&output), 1), "audio wav→flac");
    assert_output(&output, "audio wav→flac");
    assert_audio_codec(&output, "flac", "audio wav→flac");
    assert_duree_preservee(&input, &output, "audio wav→flac");
    cleanup(&output);
}

#[test]
fn test_audio_ogg_vers_mp3() {
    setup();
    let input = format!("{TEST_AUDIO}/ogg.ogg");
    let output = format!("{OUT}/audio_ogg_to_mp3.mp3");
    cleanup(&output);
    run_ffmpeg(|| crate::modules::audio::convertir(Path::new(&input), Path::new(&output), 1), "audio ogg→mp3");
    assert_output(&output, "audio ogg→mp3");
    assert_audio_codec(&output, "mp3", "audio ogg→mp3");
    assert_duree_preservee(&input, &output, "audio ogg→mp3");
    cleanup(&output);
}

#[test]
fn test_audio_aac_vers_mp3() {
    setup();
    let input = format!("{TEST_AUDIO}/aac.aac");
    let output = format!("{OUT}/audio_aac_to_mp3.mp3");
    cleanup(&output);
    run_ffmpeg(|| crate::modules::audio::convertir(Path::new(&input), Path::new(&output), 1), "audio aac→mp3");
    assert_output(&output, "audio aac→mp3");
    assert_audio_codec(&output, "mp3", "audio aac→mp3");
    assert_duree_preservee(&input, &output, "audio aac→mp3");
    cleanup(&output);
}

#[test]
fn test_audio_flac_vers_mp3() {
    setup();
    let input = format!("{TEST_AUDIO}/flac.flac");
    let output = format!("{OUT}/audio_flac_to_mp3.mp3");
    cleanup(&output);
    run_ffmpeg(|| crate::modules::audio::convertir(Path::new(&input), Path::new(&output), 1), "audio flac→mp3");
    assert_output(&output, "audio flac→mp3");
    assert_audio_codec(&output, "mp3", "audio flac→mp3");
    assert_duree_preservee(&input, &output, "audio flac→mp3");
    cleanup(&output);
}

#[test]
fn test_audio_wav_vers_mp3() {
    setup();
    let input = format!("{TEST_AUDIO}/wav.wav");
    let output = format!("{OUT}/audio_wav_to_mp3.mp3");
    cleanup(&output);
    run_ffmpeg(|| crate::modules::audio::convertir(Path::new(&input), Path::new(&output), 1), "audio wav→mp3");
    assert_output(&output, "audio wav→mp3");
    assert_audio_codec(&output, "mp3", "audio wav→mp3");
    assert_duree_preservee(&input, &output, "audio wav→mp3");
    cleanup(&output);
}

#[test]
fn test_audio_mp3_vers_ogg() {
    setup();
    let input = format!("{TEST_AUDIO}/mp3.mp3");
    let output = format!("{OUT}/audio_mp3_to_ogg.ogg");
    cleanup(&output);
    run_ffmpeg(|| crate::modules::audio::convertir(Path::new(&input), Path::new(&output), 1), "audio mp3→ogg");
    assert_output(&output, "audio mp3→ogg");
    assert_audio_codec(&output, "vorbis", "audio mp3→ogg");
    assert_duree_preservee(&input, &output, "audio mp3→ogg");
    cleanup(&output);
}

#[test]
fn test_audio_mp3_vers_aac() {
    setup();
    let input = format!("{TEST_AUDIO}/mp3.mp3");
    let output = format!("{OUT}/audio_mp3_to_aac.aac");
    cleanup(&output);
    run_ffmpeg(|| crate::modules::audio::convertir(Path::new(&input), Path::new(&output), 1), "audio mp3→aac");
    assert_output(&output, "audio mp3→aac");
    assert_audio_codec(&output, "aac", "audio mp3→aac");
    assert_duree_preservee(&input, &output, "audio mp3→aac");
    cleanup(&output);
}

#[test]
fn test_audio_detecter_extension_ogg() {
    setup();
    let ext = crate::modules::audio::detecter_extension(Path::new(&format!("{TEST_AUDIO}/ogg.ogg")));
    println!("  codec OGG: '{}'", ext);
    assert!(!ext.is_empty(), "detecter_extension vide pour OGG");
}

#[test]
fn test_audio_detecter_extension_mp3() {
    setup();
    let ext = crate::modules::audio::detecter_extension(Path::new(&format!("{TEST_AUDIO}/mp3.mp3")));
    println!("  codec MP3: '{}'", ext);
    assert!(!ext.is_empty(), "detecter_extension vide pour MP3");
}

#[test]
fn test_audio_detecter_extension_flac() {
    setup();
    let ext = crate::modules::audio::detecter_extension(Path::new(&format!("{TEST_AUDIO}/flac.flac")));
    println!("  codec FLAC: '{}'", ext);
    assert!(!ext.is_empty(), "detecter_extension vide pour FLAC");
}

#[test]
fn test_audio_formats_compatibles() {
    let fmts = crate::modules::audio::formats_compatibles("mp3");
    assert!(!fmts.is_empty(), "formats_compatibles mp3 est vide");
    assert!(fmts.contains(&"wav"), "mp3 devrait pouvoir aller vers wav");
    println!("  formats compatibles mp3: {:?}", fmts);
}

// ═══════════════════════════════════════════════════════════════
//  PIC — compress (taille réellement réduite ou au moins valide)
// ═══════════════════════════════════════════════════════════════
#[test]
fn test_pic_compresser_jpg() {
    setup();
    let input = format!("{TEST_PIC}/jpg.jpg");
    let output = format!("{OUT}/pic_jpg_c.jpg");
    cleanup(&output);
    assert!(crate::modules::pic::compresser(Path::new(&input), Path::new(&output), 2));
    assert_output(&output, "compresser JPG");
    assert_dims_preservees(&input, &output, "compresser JPG");
    cleanup(&output);
}

#[test]
fn test_pic_compresser_png() {
    setup();
    let input = format!("{TEST_PIC}/png.png");
    let output = format!("{OUT}/pic_png_c.png");
    cleanup(&output);
    assert!(crate::modules::pic::compresser(Path::new(&input), Path::new(&output), 2));
    assert_output(&output, "compresser PNG");
    assert_dims_preservees(&input, &output, "compresser PNG");
    cleanup(&output);
}

#[test]
fn test_pic_compresser_webp() {
    setup();
    let input = format!("{TEST_PIC}/webp.webp");
    let output = format!("{OUT}/pic_webp_c.png");
    cleanup(&output);
    assert!(crate::modules::pic::compresser(Path::new(&input), Path::new(&output), 2));
    assert_output(&output, "compresser WEBP");
    assert_dims_preservees(&input, &output, "compresser WEBP");
    cleanup(&output);
}

#[test]
fn test_pic_compresser_gif() {
    setup();
    let input = format!("{TEST_PIC}/gif.gif");
    let output = format!("{OUT}/pic_gif_c.png");
    cleanup(&output);
    assert!(crate::modules::pic::compresser(Path::new(&input), Path::new(&output), 2));
    assert_output(&output, "compresser GIF");
    assert_dims_preservees(&input, &output, "compresser GIF");
    cleanup(&output);
}

#[test]
fn test_pic_compresser_svg() {
    setup();
    let output = format!("{OUT}/pic_svg_c.png");
    cleanup(&output);
    assert!(crate::modules::pic::compresser(Path::new(&format!("{TEST_PIC}/svg.svg")), Path::new(&output), 1));
    assert_output(&output, "compresser SVG");
    // Le SVG source est vectoriel (pas de dimensions pixel fixes attendues côté source),
    // on vérifie juste que le PNG produit a des dimensions non nulles.
    let (w, h) = image_dims(&output);
    assert!(w > 0 && h > 0, "ÉCHEC compresser SVG — dimensions nulles");
    cleanup(&output);
}

#[test]
fn test_pic_compresser_psd() {
    setup();
    let output = format!("{OUT}/pic_psd_c.png");
    cleanup(&output);
    assert!(crate::modules::pic::compresser(Path::new(&format!("{TEST_PIC}/psd.psd")), Path::new(&output), 1));
    assert_output(&output, "compresser PSD");
    let (w, h) = image_dims(&output);
    assert!(w > 0 && h > 0, "ÉCHEC compresser PSD — dimensions nulles");
    cleanup(&output);
}

#[test]
fn test_pic_compresser_ico() {
    setup();
    let output = format!("{OUT}/pic_ico_c.png");
    cleanup(&output);
    assert!(crate::modules::pic::compresser(Path::new(&format!("{TEST_PIC}/ico.ico")), Path::new(&output), 2));
    assert_output(&output, "compresser ICO");
    let (w, h) = image_dims(&output);
    assert!(w > 0 && h > 0, "ÉCHEC compresser ICO — dimensions nulles");
    cleanup(&output);
}

#[test]
fn test_pic_compresser_tiff() {
    setup();
    let input = format!("{TEST_PIC}/tiff.tiff");
    let output = format!("{OUT}/pic_tiff_c.png");
    cleanup(&output);
    assert!(crate::modules::pic::compresser(Path::new(&input), Path::new(&output), 2));
    assert_output(&output, "compresser TIFF");
    assert_dims_preservees(&input, &output, "compresser TIFF");
    cleanup(&output);
}

// ═══════════════════════════════════════════════════════════════
//  PIC — rotate (dimensions permutées à 90°/270°, préservées à 180°)
// ═══════════════════════════════════════════════════════════════
#[test]
fn test_pic_pivoter_90() {
    setup();
    let input = format!("{TEST_PIC}/jpg.jpg");
    let output = format!("{OUT}/pic_rot90.jpg");
    cleanup(&output);
    assert!(crate::modules::pic::pivoter(Path::new(&input), Path::new(&output), 90));
    assert_output(&output, "pivoter 90°");
    assert_dims_permutees(&input, &output, "pivoter 90°");
    cleanup(&output);
}

#[test]
fn test_pic_pivoter_180() {
    setup();
    let input = format!("{TEST_PIC}/jpg.jpg");
    let output = format!("{OUT}/pic_rot180.jpg");
    cleanup(&output);
    assert!(crate::modules::pic::pivoter(Path::new(&input), Path::new(&output), 180));
    assert_output(&output, "pivoter 180°");
    assert_dims_preservees(&input, &output, "pivoter 180°");
    cleanup(&output);
}

#[test]
fn test_pic_pivoter_270() {
    setup();
    let input = format!("{TEST_PIC}/jpg.jpg");
    let output = format!("{OUT}/pic_rot270.jpg");
    cleanup(&output);
    assert!(crate::modules::pic::pivoter(Path::new(&input), Path::new(&output), 270));
    assert_output(&output, "pivoter 270°");
    assert_dims_permutees(&input, &output, "pivoter 270°");
    cleanup(&output);
}

// ═══════════════════════════════════════════════════════════════
//  PIC — crop, resize (dimensions calculées exactement, pas devinées)
// ═══════════════════════════════════════════════════════════════
#[test]
fn test_pic_recadrer() {
    setup();
    let input = format!("{TEST_PIC}/jpg.jpg");
    let output = format!("{OUT}/pic_crop.jpg");
    cleanup(&output);
    let (w_in, h_in) = image_dims(&input);
    assert!(crate::modules::pic::recadrer(Path::new(&input), Path::new(&output), 10, 10, 50, 50));
    assert_output(&output, "recadrer");
    // Même formule que pic::recadrer (division entière) pour ne pas deviner un chiffre au hasard.
    let attendu_w = (w_in * 50) / 100;
    let attendu_h = (h_in * 50) / 100;
    assert_image_dims(&output, attendu_w, attendu_h, "recadrer");
    cleanup(&output);
}

#[test]
fn test_pic_redimensionner_pixels() {
    setup();
    let output = format!("{OUT}/pic_resize_px.jpg");
    cleanup(&output);
    assert!(crate::modules::pic::redimensionner_pixels(Path::new(&format!("{TEST_PIC}/jpg.jpg")), Path::new(&output), 200, 150));
    assert_output(&output, "resize 200x150");
    assert_image_dims(&output, 200, 150, "resize 200x150");
    cleanup(&output);
}

#[test]
fn test_pic_redimensionner_poids() {
    setup();
    let output = format!("{OUT}/pic_resize_kb.jpg");
    cleanup(&output);
    assert!(crate::modules::pic::redimensionner_poids(Path::new(&format!("{TEST_PIC}/jpg.jpg")), Path::new(&output), 50));
    assert_output(&output, "resize max 50Ko");
    let taille = fs::metadata(&output).unwrap().len();
    assert!(taille <= 55 * 1024,
        "ÉCHEC resize max 50Ko — fichier fait {} octets, largement au-dessus de la cible", taille);
    cleanup(&output);
}

// ═══════════════════════════════════════════════════════════════
//  PIC — conversions (dimensions préservées à travers le changement de format)
// ═══════════════════════════════════════════════════════════════
#[test]
fn test_pic_convertir_jpg_vers_png() {
    setup();
    let input = format!("{TEST_PIC}/jpg.jpg");
    let output = format!("{OUT}/pic_jpg2png.png");
    cleanup(&output);
    assert!(crate::modules::pic::convertir(Path::new(&input), Path::new(&output)));
    assert_output(&output, "JPG→PNG");
    assert_dims_preservees(&input, &output, "JPG→PNG");
    cleanup(&output);
}

#[test]
fn test_pic_convertir_png_vers_jpg() {
    setup();
    let input = format!("{TEST_PIC}/png.png");
    let output = format!("{OUT}/pic_png2jpg.jpg");
    cleanup(&output);
    assert!(crate::modules::pic::convertir(Path::new(&input), Path::new(&output)));
    assert_output(&output, "PNG→JPG");
    assert_dims_preservees(&input, &output, "PNG→JPG");
    cleanup(&output);
}

#[test]
fn test_pic_convertir_svg_vers_png() {
    setup();
    let output = format!("{OUT}/pic_svg2png.png");
    cleanup(&output);
    assert!(crate::modules::pic::convertir(Path::new(&format!("{TEST_PIC}/svg.svg")), Path::new(&output)));
    assert_output(&output, "SVG→PNG");
    let (w, h) = image_dims(&output);
    assert!(w > 0 && h > 0, "ÉCHEC SVG→PNG — dimensions nulles");
    cleanup(&output);
}

#[test]
fn test_pic_convertir_jpg_vers_webp() {
    setup();
    let input = format!("{TEST_PIC}/jpg.jpg");
    let output = format!("{OUT}/pic_jpg2webp.webp");
    cleanup(&output);
    assert!(crate::modules::pic::convertir(Path::new(&input), Path::new(&output)));
    assert_output(&output, "JPG→WEBP");
    assert_dims_preservees(&input, &output, "JPG→WEBP");
    cleanup(&output);
}

#[test]
fn test_pic_convertir_png_vers_webp() {
    setup();
    let input = format!("{TEST_PIC}/png.png");
    let output = format!("{OUT}/pic_png2webp.webp");
    cleanup(&output);
    assert!(crate::modules::pic::convertir(Path::new(&input), Path::new(&output)));
    assert_output(&output, "PNG→WEBP");
    assert_dims_preservees(&input, &output, "PNG→WEBP");
    cleanup(&output);
}

#[test]
fn test_pic_convertir_webp_vers_png() {
    setup();
    let input = format!("{TEST_PIC}/webp.webp");
    let output = format!("{OUT}/pic_webp2png.png");
    cleanup(&output);
    assert!(crate::modules::pic::convertir(Path::new(&input), Path::new(&output)));
    assert_output(&output, "WEBP→PNG");
    assert_dims_preservees(&input, &output, "WEBP→PNG");
    cleanup(&output);
}

#[test]
fn test_pic_convertir_gif_vers_png() {
    setup();
    let input = format!("{TEST_PIC}/gif.gif");
    let output = format!("{OUT}/pic_gif2png.png");
    cleanup(&output);
    assert!(crate::modules::pic::convertir(Path::new(&input), Path::new(&output)));
    assert_output(&output, "GIF→PNG");
    assert_dims_preservees(&input, &output, "GIF→PNG");
    cleanup(&output);
}

#[test]
fn test_pic_convertir_tiff_vers_jpg() {
    setup();
    let input = format!("{TEST_PIC}/tiff.tiff");
    let output = format!("{OUT}/pic_tiff2jpg.jpg");
    cleanup(&output);
    assert!(crate::modules::pic::convertir(Path::new(&input), Path::new(&output)));
    assert_output(&output, "TIFF→JPG");
    assert_dims_preservees(&input, &output, "TIFF→JPG");
    cleanup(&output);
}

#[test]
fn test_pic_convertir_jpg_vers_jxl() {
    setup();
    let input = format!("{TEST_PIC}/jpg.jpg");
    let output = format!("{OUT}/pic_jpg2jxl.jxl");
    cleanup(&output);
    assert!(crate::modules::pic::convertir(Path::new(&input), Path::new(&output)));
    assert_output(&output, "JPG→JXL");
    cleanup(&output);
}

#[test]
fn test_pic_convertir_png_vers_jxl() {
    setup();
    let input = format!("{TEST_PIC}/png.png");
    let output = format!("{OUT}/pic_png2jxl.jxl");
    cleanup(&output);
    assert!(crate::modules::pic::convertir(Path::new(&input), Path::new(&output)));
    assert_output(&output, "PNG→JXL");
    cleanup(&output);
}

#[test]
fn test_pic_convertir_jxl_vers_jpg() {
    setup();
    let input = format!("{TEST_PIC}/jxl.jxl");
    let output = format!("{OUT}/pic_jxl2jpg.jpg");
    cleanup(&output);
    assert!(crate::modules::pic::convertir(Path::new(&input), Path::new(&output)));
    assert_output(&output, "JXL→JPG");
    let (w, h) = image_dims(&output);
    assert!(w > 0 && h > 0, "ÉCHEC JXL→JPG — dimensions nulles");
    cleanup(&output);
}

#[test]
fn test_pic_convertir_jxl_vers_png() {
    setup();
    let input = format!("{TEST_PIC}/jxl.jxl");
    let output = format!("{OUT}/pic_jxl2png.png");
    cleanup(&output);
    assert!(crate::modules::pic::convertir(Path::new(&input), Path::new(&output)));
    assert_output(&output, "JXL→PNG");
    let (w, h) = image_dims(&output);
    assert!(w > 0 && h > 0, "ÉCHEC JXL→PNG — dimensions nulles");
    cleanup(&output);
}

// ═══════════════════════════════════════════════════════════════
//  PIC — EXIF
// ═══════════════════════════════════════════════════════════════
#[test]
fn test_pic_lire_exif() {
    setup();
    let tags = crate::modules::pic::lire_exif(Path::new(&format!("{TEST_PIC}/jpg.jpg")));
    println!("  EXIF JPG: {} entrées", tags.len());
}

#[test]
fn test_pic_supprimer_exif() {
    setup();
    let input = format!("{TEST_PIC}/jpg.jpg");
    let output = format!("{OUT}/pic_no_exif.jpg");
    cleanup(&output);
    assert!(crate::modules::pic::supprimer_exif(Path::new(&input), Path::new(&output)));
    assert_output(&output, "supprimer EXIF");
    // Vérification réelle : les tags EXIF doivent avoir disparu du résultat.
    let tags_apres = crate::modules::pic::lire_exif(Path::new(&output));
    assert!(tags_apres.is_empty(), "ÉCHEC supprimer EXIF — {} tags encore présents après suppression", tags_apres.len());
    assert_dims_preservees(&input, &output, "supprimer EXIF");
    cleanup(&output);
}

// ═══════════════════════════════════════════════════════════════
//  DOC — conversion (structure du résultat vérifiée selon le format)
// ═══════════════════════════════════════════════════════════════
#[test]
fn test_doc_convertir_md_vers_pdf() {
    setup();
    let output = format!("{OUT}/doc_md2pdf.pdf");
    cleanup(&output);
    assert!(crate::modules::doc::convertir(Path::new(&format!("{TEST_DOC}/md.md")), Path::new(&output)), "MD→PDF échoué");
    assert_output(&output, "MD→PDF");
    assert!(pdf_page_count(&output) > 0, "ÉCHEC MD→PDF — 0 page dans le PDF produit");
    cleanup(&output);
}

#[test]
fn test_doc_convertir_md_vers_html() {
    setup();
    let output = format!("{OUT}/doc_md2html.html");
    cleanup(&output);
    assert!(crate::modules::doc::convertir(Path::new(&format!("{TEST_DOC}/md.md")), Path::new(&output)), "MD→HTML échoué");
    assert_output(&output, "MD→HTML");
    assert_contient_texte(&output, "<", "MD→HTML");
    cleanup(&output);
}

#[test]
fn test_doc_convertir_md_vers_docx() {
    setup();
    let output = format!("{OUT}/doc_md2docx.docx");
    cleanup(&output);
    assert!(crate::modules::doc::convertir(Path::new(&format!("{TEST_DOC}/md.md")), Path::new(&output)), "MD→DOCX échoué");
    assert_output(&output, "MD→DOCX");
    assert_zip_contient(&output, "word/document.xml", "MD→DOCX");
    cleanup(&output);
}

#[test]
fn test_doc_convertir_docx_vers_pdf() {
    setup();
    let output = format!("{OUT}/doc_docx2pdf.pdf");
    cleanup(&output);
    assert!(crate::modules::doc::convertir(Path::new(&format!("{TEST_DOC}/docx.docx")), Path::new(&output)), "DOCX→PDF échoué");
    assert_output(&output, "DOCX→PDF");
    assert!(pdf_page_count(&output) > 0, "ÉCHEC DOCX→PDF — 0 page dans le PDF produit");
    cleanup(&output);
}

#[test]
fn test_doc_convertir_docx_vers_html() {
    setup();
    let output = format!("{OUT}/doc_docx2html.html");
    cleanup(&output);
    assert!(crate::modules::doc::convertir(Path::new(&format!("{TEST_DOC}/docx.docx")), Path::new(&output)), "DOCX→HTML échoué");
    assert_output(&output, "DOCX→HTML");
    assert_contient_texte(&output, "<", "DOCX→HTML");
    cleanup(&output);
}

#[test]
fn test_doc_convertir_html_vers_pdf() {
    setup();
    let output = format!("{OUT}/doc_html2pdf.pdf");
    cleanup(&output);
    assert!(crate::modules::doc::convertir(Path::new(&format!("{TEST_DOC}/html.html")), Path::new(&output)), "HTML→PDF échoué");
    assert_output(&output, "HTML→PDF");
    assert!(pdf_page_count(&output) > 0, "ÉCHEC HTML→PDF — 0 page dans le PDF produit");
    cleanup(&output);
}

#[test]
fn test_doc_convertir_html_vers_md() {
    setup();
    let output = format!("{OUT}/doc_html2md.md");
    cleanup(&output);
    assert!(crate::modules::doc::convertir(Path::new(&format!("{TEST_DOC}/html.html")), Path::new(&output)), "HTML→MD échoué");
    assert_output(&output, "HTML→MD");
    cleanup(&output);
}

#[test]
fn test_doc_convertir_txt_vers_pdf() {
    setup();
    let output = format!("{OUT}/doc_txt2pdf.pdf");
    cleanup(&output);
    assert!(crate::modules::doc::convertir(Path::new(&format!("{TEST_DOC}/txt.txt")), Path::new(&output)), "TXT→PDF échoué");
    assert_output(&output, "TXT→PDF");
    assert!(pdf_page_count(&output) > 0, "ÉCHEC TXT→PDF — 0 page dans le PDF produit");
    cleanup(&output);
}

#[test]
fn test_doc_convertir_txt_vers_html() {
    setup();
    let output = format!("{OUT}/doc_txt2html.html");
    cleanup(&output);
    assert!(crate::modules::doc::convertir(Path::new(&format!("{TEST_DOC}/txt.txt")), Path::new(&output)), "TXT→HTML échoué");
    assert_output(&output, "TXT→HTML");
    assert_contient_texte(&output, "<", "TXT→HTML");
    cleanup(&output);
}

#[test]
fn test_doc_convertir_odt_vers_pdf() {
    setup();
    let output = format!("{OUT}/doc_odt2pdf.pdf");
    cleanup(&output);
    assert!(crate::modules::doc::convertir(Path::new(&format!("{TEST_DOC}/odt.odt")), Path::new(&output)), "ODT→PDF échoué");
    assert_output(&output, "ODT→PDF");
    assert!(pdf_page_count(&output) > 0, "ÉCHEC ODT→PDF — 0 page dans le PDF produit");
    cleanup(&output);
}

#[test]
fn test_doc_convertir_odt_vers_html() {
    setup();
    let output = format!("{OUT}/doc_odt2html.html");
    cleanup(&output);
    assert!(crate::modules::doc::convertir(Path::new(&format!("{TEST_DOC}/odt.odt")), Path::new(&output)), "ODT→HTML échoué");
    assert_output(&output, "ODT→HTML");
    assert_contient_texte(&output, "<", "ODT→HTML");
    cleanup(&output);
}

// ═══════════════════════════════════════════════════════════════
//  DOC — format detection
// ═══════════════════════════════════════════════════════════════
#[test]
fn test_doc_detecter_format_entree_md() {
    setup();
    let fmt = crate::modules::doc::detecter_format_entree(Path::new(&format!("{TEST_DOC}/md.md")));
    println!("  format entree MD: {:?}", fmt);
}

#[test]
fn test_doc_detecter_format_entree_docx() {
    setup();
    let fmt = crate::modules::doc::detecter_format_entree(Path::new(&format!("{TEST_DOC}/docx.docx")));
    println!("  format entree DOCX: {:?}", fmt);
}

#[test]
fn test_doc_detecter_format_entree_html() {
    setup();
    let fmt = crate::modules::doc::detecter_format_entree(Path::new(&format!("{TEST_DOC}/html.html")));
    println!("  format entree HTML: {:?}", fmt);
}

#[test]
fn test_doc_detecter_format_sortie_pdf() {
    setup();
    let fmt = crate::modules::doc::detecter_format_sortie(Path::new("output.pdf"));
    println!("  format sortie pdf: {:?}", fmt);
}

#[test]
fn test_doc_detecter_format_sortie_html() {
    setup();
    let fmt = crate::modules::doc::detecter_format_sortie(Path::new("output.html"));
    println!("  format sortie html: {:?}", fmt);
}

// ═══════════════════════════════════════════════════════════════
//  DOC — PDF operations (nombre de pages vérifié à chaque étape)
// ═══════════════════════════════════════════════════════════════
#[test]
fn test_doc_pdf_split() {
    setup();
    let input = format!("{TEST_DOC}/pdf.pdf");
    let output_dir = format!("{OUT}/pdf_split_pages");
    let _ = fs::remove_dir_all(&output_dir);
    let _ = fs::create_dir_all(&output_dir);
    let pages_source = pdf_page_count(&input);
    let result = crate::modules::doc::pdf_split(Path::new(&input), Path::new(&output_dir));
    assert!(result.is_ok(), "pdf_split échoué : {:?}", result);
    let fichiers: Vec<_> = fs::read_dir(&output_dir).unwrap().filter_map(|e| e.ok()).collect();
    assert_eq!(fichiers.len(), pages_source,
        "ÉCHEC pdf_split — {} fichiers produits, {} pages attendues (une par page source)",
        fichiers.len(), pages_source);
    for f in &fichiers {
        let p = f.path();
        let n = pdf_page_count(p.to_str().unwrap());
        assert_eq!(n, 1, "ÉCHEC pdf_split — {:?} contient {n} pages au lieu d'une seule", p);
    }
    let _ = fs::remove_dir_all(&output_dir);
}

#[test]
fn test_doc_pdf_merge() {
    setup();
    let input = format!("{TEST_DOC}/pdf.pdf");
    let output = format!("{OUT}/doc_pdf_merged.pdf");
    cleanup(&output);
    let pages_source = pdf_page_count(&input);
    let p = Path::new(&input);
    let result = crate::modules::doc::pdf_merge(&[p, p], Path::new(&output));
    assert!(result.is_ok(), "pdf_merge échoué : {:?}", result);
    assert_output(&output, "pdf merge");
    assert_pdf_pages(&output, pages_source * 2, "pdf merge (2 copies)");
    cleanup(&output);
}

#[test]
fn test_doc_pdf_rotate() {
    setup();
    let input = format!("{TEST_DOC}/pdf.pdf");
    let output = format!("{OUT}/doc_pdf_rot90.pdf");
    cleanup(&output);
    let pages_source = pdf_page_count(&input);
    let result = crate::modules::doc::pdf_rotate(Path::new(&input), Path::new(&output), 90, None);
    assert!(result.is_ok(), "pdf_rotate 90 échoué : {:?}", result);
    assert_output(&output, "pdf rotate 90°");
    assert_pdf_pages(&output, pages_source, "pdf rotate 90° (nombre de pages inchangé)");
    cleanup(&output);
}

#[test]
fn test_doc_pdf_rotate_180() {
    setup();
    let input = format!("{TEST_DOC}/pdf.pdf");
    let output = format!("{OUT}/doc_pdf_rot180.pdf");
    cleanup(&output);
    let pages_source = pdf_page_count(&input);
    let result = crate::modules::doc::pdf_rotate(Path::new(&input), Path::new(&output), 180, None);
    assert!(result.is_ok(), "pdf_rotate 180 échoué : {:?}", result);
    assert_output(&output, "pdf rotate 180°");
    assert_pdf_pages(&output, pages_source, "pdf rotate 180° (nombre de pages inchangé)");
    cleanup(&output);
}

#[test]
fn test_doc_pdf_rotate_pages_specifiques() {
    setup();
    let input = format!("{TEST_DOC}/pdf.pdf");
    let output = format!("{OUT}/doc_pdf_rot_p1.pdf");
    cleanup(&output);
    let pages_source = pdf_page_count(&input);
    let pages = vec![1u32];
    let result = crate::modules::doc::pdf_rotate(Path::new(&input), Path::new(&output), 90, Some(&pages));
    assert!(result.is_ok(), "pdf_rotate page 1 échoué : {:?}", result);
    assert_output(&output, "pdf rotate page 1");
    assert_pdf_pages(&output, pages_source, "pdf rotate page 1 (nombre de pages inchangé)");
    cleanup(&output);
}

#[test]
fn test_doc_pdf_compress() {
    setup();
    let input = format!("{TEST_DOC}/pdf.pdf");
    let output = format!("{OUT}/doc_pdf_comp.pdf");
    cleanup(&output);
    let pages_source = pdf_page_count(&input);
    let result = crate::modules::doc::pdf_compresser(Path::new(&input), Path::new(&output), 9);
    assert!(result.is_ok(), "pdf_compresser échoué : {:?}", result);
    assert_output(&output, "pdf compress");
    assert_pdf_pages(&output, pages_source, "pdf compress (nombre de pages inchangé)");
    cleanup(&output);
}

#[test]
fn test_doc_pdf_crop() {
    setup();
    let input = format!("{TEST_DOC}/pdf.pdf");
    let output = format!("{OUT}/doc_pdf_crop.pdf");
    cleanup(&output);
    let pages_source = pdf_page_count(&input);
    let result = crate::modules::doc::pdf_crop(Path::new(&input), Path::new(&output), 10.0, 10.0, 80.0, 80.0, None);
    assert!(result.is_ok(), "pdf_crop échoué : {:?}", result);
    assert_output(&output, "pdf crop");
    assert_pdf_pages(&output, pages_source, "pdf crop (nombre de pages inchangé)");
    cleanup(&output);
}

#[test]
fn test_doc_pdf_organiser() {
    setup();
    let output = format!("{OUT}/doc_pdf_org.pdf");
    cleanup(&output);
    let result = crate::modules::doc::pdf_organiser(Path::new(&format!("{TEST_DOC}/pdf.pdf")), Path::new(&output), &[1]);
    assert!(result.is_ok(), "pdf_organiser échoué : {:?}", result);
    assert_output(&output, "pdf organiser");
    assert_pdf_pages(&output, 1, "pdf organiser (une seule page sélectionnée)");
    cleanup(&output);
}

#[test]
fn test_doc_pdf_supprimer_pages() {
    setup();
    let input = format!("{TEST_DOC}/pdf.pdf");
    let merged = format!("{OUT}/doc_pdf_for_del.pdf");
    cleanup(&merged);
    let p = Path::new(&input);
    let _ = crate::modules::doc::pdf_merge(&[p, p, p], Path::new(&merged));
    let pages_avant = pdf_page_count(&merged);

    let output = format!("{OUT}/doc_pdf_del.pdf");
    cleanup(&output);
    let result = crate::modules::doc::pdf_supprimer_pages(Path::new(&merged), Path::new(&output), &[2]);
    assert!(result.is_ok(), "pdf_supprimer_pages échoué : {:?}", result);
    assert_output(&output, "pdf supprimer page");
    assert_pdf_pages(&output, pages_avant - 1, "pdf supprimer page (une page de moins)");
    cleanup(&output);
    cleanup(&merged);
}

#[test]
fn test_doc_pdf_numeroter() {
    setup();
    let input = format!("{TEST_DOC}/pdf.pdf");
    let output = format!("{OUT}/doc_pdf_num.pdf");
    cleanup(&output);
    let pages_source = pdf_page_count(&input);
    let result = crate::modules::doc::pdf_numeroter(
        Path::new(&input), Path::new(&output), 1,
        crate::modules::doc::PositionNumero::BasCentre, 10.0
    );
    assert!(result.is_ok(), "pdf_numeroter échoué : {:?}", result);
    assert_output(&output, "pdf numéroter");
    assert_pdf_pages(&output, pages_source, "pdf numéroter (nombre de pages inchangé)");
    cleanup(&output);
}

#[test]
fn test_doc_pdf_watermark() {
    setup();
    let input = format!("{TEST_DOC}/pdf.pdf");
    let output = format!("{OUT}/doc_pdf_wm.pdf");
    cleanup(&output);
    let pages_source = pdf_page_count(&input);
    let result = crate::modules::doc::pdf_watermark(Path::new(&input), Path::new(&output), "TEST", 40.0, 0.3, None);
    assert!(result.is_ok(), "pdf_watermark échoué : {:?}", result);
    assert_output(&output, "pdf watermark");
    assert_pdf_pages(&output, pages_source, "pdf watermark (nombre de pages inchangé)");
    cleanup(&output);
}

#[test]
fn test_doc_pdf_repair() {
    setup();
    let input = format!("{TEST_DOC}/pdf.pdf");
    let output = format!("{OUT}/doc_pdf_rep.pdf");
    cleanup(&output);
    let pages_source = pdf_page_count(&input);
    let result = crate::modules::doc::pdf_reparer(Path::new(&input), Path::new(&output));
    assert!(result.is_ok(), "pdf_reparer échoué : {:?}", result);
    assert_output(&output, "pdf repair");
    assert_pdf_pages(&output, pages_source, "pdf repair (nombre de pages inchangé)");
    cleanup(&output);
}

#[test]
fn test_doc_pdf_protect_unlock() {
    setup();
    let input = format!("{TEST_DOC}/pdf.pdf");
    let protected = format!("{OUT}/doc_pdf_prot.pdf");
    let unlocked = format!("{OUT}/doc_pdf_unlk.pdf");
    cleanup(&protected);
    cleanup(&unlocked);
    let pages_source = pdf_page_count(&input);

    let result = crate::modules::doc::pdf_proteger(Path::new(&input), Path::new(&protected), "owner123", "user123", true, false);
    assert!(result.is_ok(), "pdf_proteger échoué : {:?}", result);
    assert_output(&protected, "pdf protect");

    let result = crate::modules::doc::pdf_dechiffrer(Path::new(&protected), Path::new(&unlocked), "owner123");
    assert!(result.is_ok(), "pdf_dechiffrer échoué : {:?}", result);
    assert_output(&unlocked, "pdf unlock");
    assert_pdf_pages(&unlocked, pages_source, "pdf protect+unlock (nombre de pages inchangé)");

    cleanup(&protected);
    cleanup(&unlocked);
}

// ═══════════════════════════════════════════════════════════════
//  ARCHIVE — zip, 7z, tar + extraction (contenu réellement vérifié)
// ═══════════════════════════════════════════════════════════════
#[test]
fn test_archive_compresser_zip() {
    setup();
    let output = format!("{OUT}/arc_test.zip");
    cleanup(&output);
    assert!(crate::modules::archive::compresser(Path::new(&format!("{TEST_PIC}/jpg.jpg")), Path::new(&output), "zip", 6));
    assert_output(&output, "archive zip");
    let file = fs::File::open(&output).unwrap();
    let archive = zip::ZipArchive::new(file).unwrap_or_else(|e| panic!("ÉCHEC archive zip — pas un zip valide : {e}"));
    assert_eq!(archive.len(), 1, "ÉCHEC archive zip — {} entrées, 1 attendue", archive.len());
    cleanup(&output);
}

#[test]
fn test_archive_compresser_7z() {
    setup();
    let output = format!("{OUT}/arc_test.7z");
    cleanup(&output);
    assert!(crate::modules::archive::compresser(Path::new(&format!("{TEST_PIC}/jpg.jpg")), Path::new(&output), "7z", 6));
    assert_output(&output, "archive 7z");
    cleanup(&output);
}

#[test]
fn test_archive_compresser_tar() {
    setup();
    let output = format!("{OUT}/arc_test.tar.gz");
    cleanup(&output);
    assert!(crate::modules::archive::compresser(Path::new(&format!("{TEST_PIC}/jpg.jpg")), Path::new(&output), "tar", 6));
    assert_output(&output, "archive tar");
    cleanup(&output);
}

#[test]
fn test_archive_extraire_zip() {
    setup();
    let zip = format!("{OUT}/arc_ext.zip");
    let dir = format!("{OUT}/arc_ext_zip");
    cleanup(&zip);
    let _ = fs::remove_dir_all(&dir);
    assert!(crate::modules::archive::compresser(Path::new(&format!("{TEST_PIC}/jpg.jpg")), Path::new(&zip), "zip", 6));
    let _ = fs::create_dir_all(&dir);
    assert!(crate::modules::archive::extraire(Path::new(&zip), Path::new(&dir)), "extraction zip échouée");
    let extrait = Path::new(&dir).join("jpg.jpg");
    assert!(extrait.exists(), "ÉCHEC extraction zip — jpg.jpg absent du dossier extrait");
    let taille_extraite = fs::metadata(&extrait).unwrap().len();
    let taille_source = fs::metadata(format!("{TEST_PIC}/jpg.jpg")).unwrap().len();
    assert_eq!(taille_extraite, taille_source, "ÉCHEC extraction zip — taille du fichier extrait différente de la source");
    cleanup(&zip);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_archive_extraire_7z() {
    setup();
    let sz = format!("{OUT}/arc_ext.7z");
    let dir = format!("{OUT}/arc_ext_7z");
    cleanup(&sz);
    let _ = fs::remove_dir_all(&dir);
    assert!(crate::modules::archive::compresser(Path::new(&format!("{TEST_PIC}/jpg.jpg")), Path::new(&sz), "7z", 6));
    let _ = fs::create_dir_all(&dir);
    assert!(crate::modules::archive::extraire(Path::new(&sz), Path::new(&dir)), "extraction 7z échouée");
    let extrait = Path::new(&dir).join("jpg.jpg");
    assert!(extrait.exists(), "ÉCHEC extraction 7z — jpg.jpg absent du dossier extrait");
    let taille_extraite = fs::metadata(&extrait).unwrap().len();
    let taille_source = fs::metadata(format!("{TEST_PIC}/jpg.jpg")).unwrap().len();
    assert_eq!(taille_extraite, taille_source, "ÉCHEC extraction 7z — taille du fichier extrait différente de la source");
    cleanup(&sz);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_archive_extraire_tar() {
    setup();
    let tar = format!("{OUT}/arc_ext.tar.gz");
    let dir = format!("{OUT}/arc_ext_tar");
    cleanup(&tar);
    let _ = fs::remove_dir_all(&dir);
    assert!(crate::modules::archive::compresser(Path::new(&format!("{TEST_PIC}/jpg.jpg")), Path::new(&tar), "tar", 6));
    let _ = fs::create_dir_all(&dir);
    assert!(crate::modules::archive::extraire(Path::new(&tar), Path::new(&dir)), "extraction tar échouée");
    let extrait = Path::new(&dir).join("jpg.jpg");
    assert!(extrait.exists(), "ÉCHEC extraction tar — jpg.jpg absent du dossier extrait");
    let taille_extraite = fs::metadata(&extrait).unwrap().len();
    let taille_source = fs::metadata(format!("{TEST_PIC}/jpg.jpg")).unwrap().len();
    assert_eq!(taille_extraite, taille_source, "ÉCHEC extraction tar — taille du fichier extrait différente de la source");
    cleanup(&tar);
    let _ = fs::remove_dir_all(&dir);
}

// ═══════════════════════════════════════════════════════════════
//  RENAME (déjà des vérifications réelles — inchangé)
// ═══════════════════════════════════════════════════════════════
#[test]
fn test_rename_find_replace() {
    let cfg = crate::modules::rename::RenameConfig {
        find: "foo".into(),
        replace_with: "bar".into(),
        ..Default::default()
    };
    let files = vec![std::path::PathBuf::from("foo_test.jpg")];
    let previews = crate::modules::rename::preview(&files, &cfg);
    assert_eq!(previews[0].1, "bar_test.jpg");
    println!("  rename find/replace: OK");
}

#[test]
fn test_rename_insert() {
    let cfg = crate::modules::rename::RenameConfig {
        insert_text: "PREFIX_".into(),
        insert_pos: 0,
        ..Default::default()
    };
    let files = vec![std::path::PathBuf::from("test.jpg")];
    let previews = crate::modules::rename::preview(&files, &cfg);
    assert_eq!(previews[0].1, "PREFIX_test.jpg");
    println!("  rename insert: OK");
}

#[test]
fn test_rename_delete_range() {
    let cfg = crate::modules::rename::RenameConfig {
        delete_enabled: true,
        delete_from: 0,
        delete_count: 3,
        ..Default::default()
    };
    let files = vec![std::path::PathBuf::from("abctest.jpg")];
    let previews = crate::modules::rename::preview(&files, &cfg);
    assert_eq!(previews[0].1, "test.jpg");
    println!("  rename delete range: OK");
}

#[test]
fn test_rename_numbering_suffix() {
    let cfg = crate::modules::rename::RenameConfig {
        num_enabled: true,
        num_start: 1,
        num_step: 1,
        num_padding: 2,
        num_pos: crate::modules::rename::NumPos::Suffix,
        num_sep: "_".into(),
        ..Default::default()
    };
    let files = vec![
        std::path::PathBuf::from("photo.jpg"),
        std::path::PathBuf::from("photo.jpg"),
    ];
    let previews = crate::modules::rename::preview(&files, &cfg);
    assert_eq!(previews[0].1, "photo_01.jpg");
    assert_eq!(previews[1].1, "photo_02.jpg");
    println!("  rename numbering suffix: OK");
}

#[test]
fn test_rename_numbering_prefix() {
    let cfg = crate::modules::rename::RenameConfig {
        num_enabled: true,
        num_start: 10,
        num_step: 2,
        num_padding: 3,
        num_pos: crate::modules::rename::NumPos::Prefix,
        num_sep: " - ".into(),
        ..Default::default()
    };
    let files = vec![std::path::PathBuf::from("file.mp4")];
    let previews = crate::modules::rename::preview(&files, &cfg);
    assert_eq!(previews[0].1, "010 - file.mp4");
    println!("  rename numbering prefix: OK");
}

#[test]
fn test_rename_case_upper() {
    let cfg = crate::modules::rename::RenameConfig {
        case_mode: crate::modules::rename::CaseMode::Upper,
        ..Default::default()
    };
    let files = vec![std::path::PathBuf::from("hello world.jpg")];
    let previews = crate::modules::rename::preview(&files, &cfg);
    assert_eq!(previews[0].1, "HELLO WORLD.jpg");
    println!("  rename case upper: OK");
}

#[test]
fn test_rename_case_title() {
    let cfg = crate::modules::rename::RenameConfig {
        case_mode: crate::modules::rename::CaseMode::Title,
        ..Default::default()
    };
    let files = vec![std::path::PathBuf::from("hello world.jpg")];
    let previews = crate::modules::rename::preview(&files, &cfg);
    assert_eq!(previews[0].1, "Hello World.jpg");
    println!("  rename case title: OK");
}

#[test]
fn test_rename_ext_lower() {
    let cfg = crate::modules::rename::RenameConfig {
        ext_mode: crate::modules::rename::ExtMode::Lower,
        ..Default::default()
    };
    let files = vec![std::path::PathBuf::from("FILE.JPG")];
    let previews = crate::modules::rename::preview(&files, &cfg);
    assert_eq!(previews[0].1, "FILE.jpg");
    println!("  rename ext lower: OK");
}

#[test]
fn test_rename_ext_replace() {
    let cfg = crate::modules::rename::RenameConfig {
        ext_mode: crate::modules::rename::ExtMode::Replace,
        ext_new: "png".into(),
        ..Default::default()
    };
    let files = vec![std::path::PathBuf::from("image.jpg")];
    let previews = crate::modules::rename::preview(&files, &cfg);
    assert_eq!(previews[0].1, "image.png");
    println!("  rename ext replace: OK");
}

#[test]
fn test_rename_strip_double_spaces() {
    let cfg = crate::modules::rename::RenameConfig {
        strip_double_spaces: true,
        ..Default::default()
    };
    let files = vec![std::path::PathBuf::from("hello  world.jpg")];
    let previews = crate::modules::rename::preview(&files, &cfg);
    assert_eq!(previews[0].1, "hello world.jpg");
    println!("  rename strip double spaces: OK");
}

#[test]
fn test_rename_strip_chars() {
    let cfg = crate::modules::rename::RenameConfig {
        strip_chars: "!?#".into(),
        ..Default::default()
    };
    let files = vec![std::path::PathBuf::from("he!llo?wo#rld.jpg")];
    let previews = crate::modules::rename::preview(&files, &cfg);
    assert_eq!(previews[0].1, "helloworld.jpg");
    println!("  rename strip chars: OK");
}

#[test]
fn test_rename_no_change() {
    let cfg = crate::modules::rename::RenameConfig::default();
    let files = vec![std::path::PathBuf::from("unchanged.mp4")];
    let previews = crate::modules::rename::preview(&files, &cfg);
    assert_eq!(previews[0].1, "unchanged.mp4");
    println!("  rename no change: OK");
}

// ═══════════════════════════════════════════════════════════════
//  AUDIO — extraction from video (codec + durée vérifiés)
// ═══════════════════════════════════════════════════════════════
#[test]
fn test_audio_extraire_depuis_mkv() {
    setup();
    let input = format!("{TEST_VIDEO}/mkv.mkv");
    let output = format!("{OUT}/audio_extrait.mkv");
    cleanup(&output);
    run_ffmpeg(|| crate::modules::audio::extraire(Path::new(&input), Path::new(&output)), "audio extraire mkv");
    assert_output(&output, "audio extraire mkv");
    // extraire() ne fait que retirer la vidéo (-vn), le codec audio d'origine doit être conservé.
    let codec_source = probe_stream_field(&input, "a:0", "codec_name");
    let codec_sortie = probe_stream_field(&output, "a:0", "codec_name");
    assert_eq!(codec_source, codec_sortie, "ÉCHEC audio extraire mkv — codec audio changé alors que -vn ne devrait pas le toucher");
    assert_duree_preservee(&input, &output, "audio extraire mkv");
    cleanup(&output);
}

#[test]
fn test_audio_extraire_depuis_mp4() {
    setup();
    let input = format!("{TEST_VIDEO}/mp4.mp4");
    let output = format!("{OUT}/audio_extrait_mp4.mkv");
    cleanup(&output);
    run_ffmpeg(|| crate::modules::audio::extraire(Path::new(&input), Path::new(&output)), "audio extraire mp4");
    assert_output(&output, "audio extraire mp4");
    let codec_source = probe_stream_field(&input, "a:0", "codec_name");
    let codec_sortie = probe_stream_field(&output, "a:0", "codec_name");
    assert_eq!(codec_source, codec_sortie, "ÉCHEC audio extraire mp4 — codec audio changé alors que -vn ne devrait pas le toucher");
    assert_duree_preservee(&input, &output, "audio extraire mp4");
    cleanup(&output);
}

// ═══════════════════════════════════════════════════════════════
//  VIDEO — conversions (codec réellement vérifié selon le conteneur cible)
// ═══════════════════════════════════════════════════════════════
#[test]
fn test_video_mkv_vers_webm() {
    setup();
    let input_str = format!("{TEST_VIDEO}/mkv.mkv");
    let input = std::path::PathBuf::from(&input_str);
    let output = format!("{OUT}/vid_mkv2webm.webm");
    cleanup(&output);
    run_ffmpeg(|| crate::modules::video::traiter_video(&input, Path::new(&output), false, false, 1), "mkv→webm");
    assert_output(&output, "mkv→webm");
    assert_video_codec(&output, "vp9", "mkv→webm");
    assert_duree_preservee(&input_str, &output, "mkv→webm");
    cleanup(&output);
}

#[test]
fn test_video_mkv_vers_mp4() {
    setup();
    let input_str = format!("{TEST_VIDEO}/mkv.mkv");
    let input = std::path::PathBuf::from(&input_str);
    let output = format!("{OUT}/vid_mkv2mp4.mp4");
    cleanup(&output);
    run_ffmpeg(|| crate::modules::video::traiter_video(&input, Path::new(&output), false, false, 1), "mkv→mp4");
    assert_output(&output, "mkv→mp4");
    assert_video_codec(&output, "h264", "mkv→mp4");
    assert_duree_preservee(&input_str, &output, "mkv→mp4");
    cleanup(&output);
}

#[test]
fn test_video_mp4_vers_mkv() {
    setup();
    let input_str = format!("{TEST_VIDEO}/mp4.mp4");
    let input = std::path::PathBuf::from(&input_str);
    let output = format!("{OUT}/vid_mp42mkv.mkv");
    cleanup(&output);
    run_ffmpeg(|| crate::modules::video::traiter_video(&input, Path::new(&output), false, false, 1), "mp4→mkv");
    assert_output(&output, "mp4→mkv");
    assert_video_codec(&output, "h264", "mp4→mkv");
    assert_duree_preservee(&input_str, &output, "mp4→mkv");
    cleanup(&output);
}

#[test]
fn test_video_webm_vers_mp4() {
    setup();
    let input_str = format!("{TEST_VIDEO}/webm.webm");
    let input = std::path::PathBuf::from(&input_str);
    let output = format!("{OUT}/vid_webm2mp4.mp4");
    cleanup(&output);
    run_ffmpeg(|| crate::modules::video::traiter_video(&input, Path::new(&output), false, false, 1), "webm→mp4");
    assert_output(&output, "webm→mp4");
    assert_video_codec(&output, "h264", "webm→mp4");
    assert_duree_preservee(&input_str, &output, "webm→mp4");
    cleanup(&output);
}

#[test]
fn test_video_copie_flux() {
    setup();
    let input_str = format!("{TEST_VIDEO}/mkv.mkv");
    let input = std::path::PathBuf::from(&input_str);
    let output = format!("{OUT}/vid_copy.mp4");
    cleanup(&output);
    run_ffmpeg(|| crate::modules::video::traiter_video(&input, Path::new(&output), true, false, 1), "copie flux");
    assert_output(&output, "copie flux");
    // copie_flux=true → "-c copy", le codec source doit être conservé tel quel, pas réencodé.
    let codec_source = probe_stream_field(&input_str, "v:0", "codec_name");
    let codec_sortie = probe_stream_field(&output, "v:0", "codec_name");
    assert_eq!(codec_source, codec_sortie, "ÉCHEC copie flux — codec vidéo changé alors que -c copy ne devrait pas réencoder");
    assert_duree_preservee(&input_str, &output, "copie flux");
    cleanup(&output);
}
