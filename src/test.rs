// ═══════════════════════════════════════════════════════════════
//  OXYON — Tests automatisés exhaustifs
//  Lance : cargo test
// ═══════════════════════════════════════════════════════════════

use std::path::Path;
use std::fs;
use std::sync::{Once, Mutex};

const TEST_AUDIO: &str = "tests/audio";
const TEST_DOC:   &str = "tests/doc";
const TEST_PIC:   &str = "tests/pic";
const TEST_FMT:   &str = "tests/formats";
const TEST_VIDEO: &str = "tests/video";
const OUT:        &str = "tests/_output";

static INIT: Once = Once::new();
/// Mutex global pour sérialiser les opérations ffmpeg (évite les crashs concurrents)
static FFMPEG_LOCK: Mutex<()> = Mutex::new(());

fn setup() {
    INIT.call_once(|| {
        let _ = crate::modules::binaries::extraire_deps();
    });
    let _ = fs::create_dir_all(OUT);
}

/// Verrouille le mutex ffmpeg, exécute le spawn PUIS attend la fin (séquentiel garanti)
fn run_ffmpeg<F>(spawn_fn: F, context: &str)
where F: FnOnce() -> Result<std::process::Child, std::io::Error>
{
    let _lock = FFMPEG_LOCK.lock().unwrap();
    match spawn_fn() {
        Ok(mut child) => {
            match child.wait() {
                Ok(status) => {
                    if !status.success() {
                        panic!("{} : FFmpeg a retourné un code d'erreur : {}", context, status);
                    }
                }
                Err(e) => panic!("{} : Échec de l'attente du processus FFmpeg : {}", context, e),
            }
        }
        Err(e) => panic!("{} : Impossible de lancer FFmpeg : {}", context, e),
    }
}

// ═══════════════════════════════════════════════════════════════
//  MODULE : PIC (IMAGES)
// ═══════════════════════════════════════════════════════════════

#[test]
fn test_pic_compresser_png() {
    setup();
    let output = format!("{OUT}/pic_compressed.png");
    let _ = fs::remove_file(&output);
    assert!(crate::modules::pic::compresser(Path::new(&format!("{TEST_PIC}/png.png")), &output, 2));
    assert!(Path::new(&output).exists());
}

#[test]
fn test_pic_compresser_ico() {
    setup();
    let output = format!("{OUT}/pic_compressed.ico");
    let _ = fs::remove_file(&output);
    assert!(crate::modules::pic::compresser(Path::new(&format!("{TEST_PIC}/ico.ico")), &output, 2));
    assert!(Path::new(&output).exists());
}

#[test]
fn test_pic_convertir_png_vers_jpg() {
    setup();
    let output = format!("{OUT}/pic_converted.jpg");
    let _ = fs::remove_file(&output);
    assert!(crate::modules::pic::convertir(Path::new(&format!("{TEST_PIC}/png.png")), &output));
    assert!(Path::new(&output).exists());
}

#[test]
fn test_pic_convertir_png_vers_webp() {
    setup();
    let output = format!("{OUT}/pic_converted.webp");
    let _ = fs::remove_file(&output);
    assert!(crate::modules::pic::convertir(Path::new(&format!("{TEST_PIC}/png.png")), &output));
    assert!(Path::new(&output).exists());
}

#[test]
fn test_pic_convertir_png_vers_jxl() {
    setup();
    let output = format!("{OUT}/pic_converted.jxl");
    let _ = fs::remove_file(&output);
    assert!(crate::modules::pic::convertir(Path::new(&format!("{TEST_PIC}/png.png")), &output));
    assert!(Path::new(&output).exists());
}

// ═══════════════════════════════════════════════════════════════
//  MODULE : DOC (PDF)
// ═══════════════════════════════════════════════════════════════

#[test]
fn test_doc_pdf_merge() {
    setup();
    let output = format!("{OUT}/pdf_merged.pdf");
    let _ = fs::remove_file(&output);
    
    let files = vec![
        format!("{TEST_DOC}/pdf.pdf"),
        format!("{TEST_DOC}/pdf.pdf")
    ];
    
    match crate::modules::doc::pdf_merge(&files, &output) {
        Ok(_) => assert!(Path::new(&output).exists()),
        Err(e) => panic!("pdf_merge échoué : {:?}", e),
    }
}

#[test]
fn test_doc_pdf_split() {
    setup();
    let out_dir = format!("{OUT}/pdf_split_dir");
    let _ = fs::remove_dir_all(&out_dir);
    let _ = fs::create_dir_all(&out_dir);

    match crate::modules::doc::pdf_split(&format!("{TEST_DOC}/pdf.pdf"), &out_dir) {
        Ok(_) => {
            let paths = fs::read_dir(&out_dir).unwrap();
            assert!(paths.count() > 0);
        }
        Err(e) => panic!("pdf_split échoué : {:?}", e),
    }
}

#[test]
fn test_doc_pdf_supprimer_pages() {
    setup();
    let output = format!("{OUT}/pdf_pages_supprimees.pdf");
    let _ = fs::remove_file(&output);

    match crate::modules::doc::pdf_supprimer_pages(&format!("{TEST_DOC}/pdf.pdf"), &output, "1") {
        Ok(_) => assert!(Path::new(&output).exists()),
        Err(e) => panic!("pdf_supprimer_pages échoué : {:?}", e),
    }
}

#[test]
fn test_doc_pdf_organiser() {
    setup();
    let output = format!("{OUT}/pdf_organise.pdf");
    let _ = fs::remove_file(&output);

    match crate::modules::doc::pdf_organiser(&format!("{TEST_DOC}/pdf.pdf"), &output, "1") {
        Ok(_) => assert!(Path::new(&output).exists()),
        Err(e) => panic!("pdf_organiser échoué : {:?}", e),
    }
}

#[test]
fn test_doc_pdf_rotate() {
    setup();
    let output = format!("{OUT}/pdf_rotated.pdf");
    let _ = fs::remove_file(&output);

    match crate::modules::doc::pdf_rotate_90(&format!("{TEST_DOC}/pdf.pdf"), &output) {
        Ok(_) => assert!(Path::new(&output).exists()),
        Err(e) => panic!("pdf_rotate échoué : {:?}", e),
    }
}

#[test]
fn test_doc_pdf_rotate_180() {
    setup();
    let output = format!("{OUT}/pdf_rotated_180.pdf");
    let _ = fs::remove_file(&output);

    match crate::modules::doc::pdf_rotate_180(&format!("{TEST_DOC}/pdf.pdf"), &output) {
        Ok(_) => assert!(Path::new(&output).exists()),
        Err(e) => panic!("pdf_rotate_180 échoué : {:?}", e),
    }
}

#[test]
fn test_doc_pdf_rotate_pages_specifiques() {
    setup();
    let output = format!("{OUT}/pdf_rotated_specifique.pdf");
    let _ = fs::remove_file(&output);

    match crate::modules::doc::pdf_rotate_pages_specifiques(&format!("{TEST_DOC}/pdf.pdf"), &output, "1", 90) {
        Ok(_) => assert!(Path::new(&output).exists()),
        Err(e) => panic!("pdf_rotate_pages_specifiques échoué : {:?}", e),
    }
}

#[test]
fn test_doc_pdf_crop() {
    setup();
    let output = format!("{OUT}/pdf_cropped.pdf");
    let _ = fs::remove_file(&output);

    match crate::modules::doc::pdf_crop(&format!("{TEST_DOC}/pdf.pdf"), &output, 0.0, 0.0, 200.0, 200.0) {
        Ok(_) => assert!(Path::new(&output).exists()),
        Err(e) => panic!("pdf_crop échoué : {:?}", e),
    }
}

#[test]
fn test_doc_pdf_compress() {
    setup();
    let output = format!("{OUT}/pdf_compressed.pdf");
    let _ = fs::remove_file(&output);

    match crate::modules::doc::pdf_compresser(&format!("{TEST_DOC}/pdf.pdf"), &output, 2) {
        Ok(_) => assert!(Path::new(&output).exists()),
        Err(e) => panic!("pdf_compresser échoué : {:?}", e),
    }
}

#[test]
fn test_doc_pdf_protect_unlock() {
    setup();
    let output_prot = format!("{OUT}/pdf_protege.pdf");
    let output_unl  = format!("{OUT}/pdf_deverrouille.pdf");
    let _ = fs::remove_file(&output_prot);
    let _ = fs::remove_file(&output_unl);

    match crate::modules::doc::pdf_proteger(&format!("{TEST_DOC}/pdf.pdf"), &output_prot, "password123") {
        Ok(_) => assert!(Path::new(&output_prot).exists()),
        Err(e) => panic!("pdf_proteger échoué : {:?}", e),
    }

    match crate::modules::doc::pdf_deverrouiller(&output_prot, &output_unl, "password123") {
        Ok(_) => assert!(Path::new(&output_unl).exists()),
        Err(e) => panic!("pdf_deverrouiller échoué : {:?}", e),
    }
}

#[test]
fn test_doc_pdf_watermark() {
    setup();
    let output = format!("{OUT}/pdf_watermarked.pdf");
    let _ = fs::remove_file(&output);

    match crate::modules::doc::pdf_watermark_texte(&format!("{TEST_DOC}/pdf.pdf"), &output, "CONFIDENTIEL", 45.0, 0.3) {
        Ok(_) => assert!(Path::new(&output).exists()),
        Err(e) => panic!("pdf_watermark échoué : {:?}", e),
    }
}

#[test]
fn test_doc_pdf_numeroter() {
    setup();
    let output = format!("{OUT}/pdf_numerote.pdf");
    let _ = fs::remove_file(&output);

    match crate::modules::doc::pdf_numeroter(&format!("{TEST_DOC}/pdf.pdf"), &output, "Bas-Droite", "Page {n} sur {total}") {
        Ok(_) => assert!(Path::new(&output).exists()),
        Err(e) => panic!("pdf_numeroter échoué : {:?}", e),
    }
}

#[test]
fn test_doc_pdf_repair() {
    setup();
    let output = format!("{OUT}/pdf_repare.pdf");
    let _ = fs::remove_file(&output);

    match crate::modules::doc::pdf_reparer(&format!("{TEST_DOC}/pdf.pdf"), &output) {
        Ok(_) => assert!(Path::new(&output).exists()),
        Err(e) => panic!("pdf_repair échoué : {:?}", e),
    }
}

// ═══════════════════════════════════════════════════════════════
//  MODULE : VIDEO
// ═══════════════════════════════════════════════════════════════

#[test]
fn test_video_traiter_mp4_vers_mkv_copie() {
    setup();
    let output = format!("{OUT}/video_copie.mkv");
    let _ = fs::remove_file(&output);

    run_ffmpeg(|| {
        crate::modules::video::traiter_video(
            &std::path::PathBuf::from(format!("{TEST_VIDEO}/mp4.mp4")),
            &output,
            true,
            false,
            4,
        )
    }, "test_video_traiter_mp4_vers_mkv_copie");

    assert!(Path::new(&output).exists());
}

#[test]
fn test_video_traiter_mp4_vers_webm() {
    setup();
    let output = format!("{OUT}/video_encode.webm");
    let _ = fs::remove_file(&output);

    run_ffmpeg(|| {
        crate::modules::video::traiter_video(
            &std::path::PathBuf::from(format!("{TEST_VIDEO}/mp4.mp4")),
            &output,
            false,
            false,
            5,
        )
    }, "test_video_traiter_mp4_vers_webm");

    assert!(Path::new(&output).exists());
}

#[test]
fn test_video_extraire_audio_copie() {
    setup();
    let output = format!("{OUT}/audio_extrait.mp3");
    let _ = fs::remove_file(&output);

    run_ffmpeg(|| {
        crate::modules::video::traiter_video(
            &std::path::PathBuf::from(format!("{TEST_VIDEO}/mp4.mp4")),
            &output,
            true,
            true,
            4,
        )
    }, "test_video_extraire_audio_copie");

    assert!(Path::new(&output).exists());
}

#[test]
fn test_video_extraire_nom_codec() {
    setup();
    let codec = crate::modules::video::extraire_nom_codec(&std::path::PathBuf::from(format!("{TEST_VIDEO}/mp4.mp4")));
    assert!(!codec.is_empty());
    println!("  Codec identifié : {}", codec);
}

// ═══════════════════════════════════════════════════════════════
//  MODULE : AUDIO
// ═══════════════════════════════════════════════════════════════

#[test]
fn test_audio_convertir_mp3_vers_wav() {
    setup();
    let output = format!("{OUT}/audio_converted.wav");
    let _ = fs::remove_file(&output);

    run_ffmpeg(|| {
        crate::modules::audio::convertir_audio(
            &std::path::PathBuf::from(format!("{TEST_AUDIO}/mp3.mp3")),
            &output,
            0,
        )
    }, "test_audio_convertir_mp3_vers_wav");

    assert!(Path::new(&output).exists());
}

// ═══════════════════════════════════════════════════════════════
//  MODULE : RENAME (NOMENCLATURE)
// ═══════════════════════════════════════════════════════════════

#[test]
fn test_rename_prefix() {
    let cfg = crate::modules::rename::RenameConfig {
        prefix: "pre_".into(),
        ..Default::default()
    };
    let files = vec![std::path::PathBuf::from("image.jpg")];
    let previews = crate::modules::rename::preview(&files, &cfg);
    assert_eq!(previews[0].1, "pre_image.jpg");
    println!("  rename prefix: OK");
}

#[test]
fn test_rename_suffix() {
    let cfg = crate::modules::rename::RenameConfig {
        suffix: "_post".into(),
        ..Default::default()
    };
    let files = vec![std::path::PathBuf::from("image.jpg")];
    let previews = crate::modules::rename::preview(&files, &cfg);
    assert_eq!(previews[0].1, "image_post.jpg");
    println!("  rename suffix: OK");
}

#[test]
fn test_rename_case_upper() {
    let cfg = crate::modules::rename::RenameConfig {
        case_mode: crate::modules::rename::CaseMode::Upper,
        ..Default::default()
    };
    let files = vec![std::path::PathBuf::from("image.jpg")];
    let previews = crate::modules::rename::preview(&files, &cfg);
    assert_eq!(previews[0].1, "IMAGE.jpg");
    println!("  rename case upper: OK");
}

#[test]
fn test_rename_case_lower() {
    let cfg = crate::modules::rename::RenameConfig {
        case_mode: crate::modules::rename::CaseMode::Lower,
        ..Default::default()
    };
    let files = vec![std::path::PathBuf::from("IMAGE.JPG")];
    let previews = crate::modules::rename::preview(&files, &cfg);
    assert_eq!(previews[0].1, "image.jpg");
    println!("  rename case lower: OK");
}

#[test]
fn test_rename_ext_upper() {
    let cfg = crate::modules::rename::RenameConfig {
        ext_case: crate::modules::rename::CaseMode::Upper,
        ..Default::default()
    };
    let files = vec![std::path::PathBuf::from("image.jpg")];
    let previews = crate::modules::rename::preview(&files, &cfg);
    assert_eq!(previews[0].1, "image.JPG");
    println!("  rename ext upper: OK");
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