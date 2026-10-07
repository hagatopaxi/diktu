//! Imports the default French model as a custom one, from Hugging Face then from a
//! folder, and checks that a model with a corrupted vocabulary is refused.
//! The folder import without a real model runs by default.
//! Run with `cargo test -p diktu-core --test custom -- --ignored --nocapture`.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use diktu_core::custom;
use diktu_core::download;

#[test]
#[ignore = "downloads ~71 MB from Hugging Face"]
fn imports_and_checks_real_model() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("custom-models");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let repo = &diktu_core::registry::default_for("fr").unwrap().repo;

    let model = custom::stage_hf(
        download::HF_BASE_URL,
        &root,
        repo,
        "fr",
        &AtomicBool::new(false),
        |_, _| {},
    )
    .unwrap();
    let staged = custom::staging_dir(&root, &model);
    println!("HF: {}", custom::check(&staged).unwrap());
    custom::commit(&root, &model).unwrap();
    assert_eq!(custom::list(&root).len(), 1);
    assert!(
        custom::stage_hf(
            download::HF_BASE_URL,
            &root,
            repo,
            "fr",
            &AtomicBool::new(false),
            |_, _| {}
        )
        .is_err()
    );

    let folder = root.join("kroko-copy");
    fs::create_dir_all(&folder).unwrap();
    for f in &model.files {
        fs::copy(root.join(&model.id).join(&f.path), folder.join(&f.path)).unwrap();
    }
    let local = custom::stage_folder(&root, &folder, "fr").unwrap();
    println!(
        "folder: {}",
        custom::check(&custom::staging_dir(&root, &local)).unwrap()
    );
    custom::discard(&root, &local);

    // Shuffled vocabulary: valid format, wrong ids, so the test sentence comes out wrong.
    let tokens = folder.join("tokens.txt");
    let text = fs::read_to_string(&tokens).unwrap();
    let mut lines: Vec<&str> = text.lines().collect();
    let symbols: Vec<&str> = lines
        .iter()
        .map(|l| l.rsplit_once(' ').unwrap().0)
        .collect();
    let shuffled: Vec<String> = lines
        .iter()
        .enumerate()
        .map(|(i, l)| {
            format!(
                "{} {}",
                symbols[(i * 7 + 3) % symbols.len()],
                l.rsplit_once(' ').unwrap().1
            )
        })
        .collect();
    lines = shuffled.iter().map(String::as_str).collect();
    fs::write(&tokens, lines.join("\n")).unwrap();
    let broken = custom::stage_folder(&root, &folder, "fr").unwrap();
    let err = custom::check(&custom::staging_dir(&root, &broken)).unwrap_err();
    println!("broken: {err}");
    custom::discard(&root, &broken);
}

#[test]
fn folder_import_is_staged_then_committed() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("custom-import");
    let _ = fs::remove_dir_all(&root);
    let src = root.join("src/My Model");
    fs::create_dir_all(&src).unwrap();
    for f in [
        "encoder.onnx",
        "decoder.onnx",
        "joiner.onnx",
        "tokens.txt",
        "README.md",
    ] {
        fs::write(src.join(f), f).unwrap();
    }
    let models = root.join("models");

    assert!(custom::stage_folder(&models, &src, "FR").is_err());
    let model = custom::stage_folder(&models, &src, "fr").unwrap();
    assert_eq!(model.id, "custom-my-model");
    assert_eq!(model.files.len(), 4);
    assert!(
        model
            .files
            .iter()
            .all(|f| f.size > 0 && f.sha256.len() == 64)
    );
    assert!(custom::list(&models).is_empty(), "staged is not installed");
    assert_eq!(
        custom::read_manifest(&custom::staging_dir(&models, &model))
            .unwrap()
            .revision,
        model.revision
    );

    custom::commit(&models, &model).unwrap();
    assert!(!custom::staging_dir(&models, &model).exists());
    let listed = custom::list(&models);
    assert_eq!(listed.len(), 1);
    assert!(download::is_installed(&models, &listed[0]));
    assert!(
        custom::stage_folder(&models, &src, "fr").is_err(),
        "same name"
    );

    fs::remove_file(src.join("joiner.onnx")).unwrap();
    let other = root.join("src/Other");
    fs::rename(&src, &other).unwrap();
    assert!(custom::stage_folder(&models, &other, "fr").is_err());
    assert!(!models.join(".staging-custom-other").exists());
}
