//! Imports the default French model as a custom one, from Hugging Face then from a
//! folder, and checks that a model with a corrupted vocabulary is refused.
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
