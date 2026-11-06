use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use renx::anim::{diff_segments, EffectKind, INTRO_TICKS};
use renx::app::{App, InputMode};
use renx::files;
use renx::input::handle_key;
use renx::operations::*;
use std::fs;

fn key(app: &mut App, code: KeyCode) {
    handle_key(app, KeyEvent::new(code, KeyModifiers::NONE));
}

fn entry(name: &str) -> files::FileEntry {
    files::FileEntry {
        file_name: name.to_string(),
        parent_dir: std::path::PathBuf::from("/nonexistent"),
    }
}

// --- Ticking and effects ---------------------------------------------------

#[test]
fn tick_advances_time_and_expires_effects() {
    let mut app = App::new(vec![entry("a.txt")]);
    app.animate = true;
    app.add_effect(EffectKind::OpFlash(0), 3);
    assert!(app.effect_progress(&EffectKind::OpFlash(0)).is_some());

    app.tick();
    app.tick();
    assert!(app.effect_progress(&EffectKind::OpFlash(0)).is_some());
    app.tick();
    assert!(app.effect_progress(&EffectKind::OpFlash(0)).is_none());
    assert_eq!(app.tick, 3);
}

#[test]
fn effect_progress_runs_from_zero_to_one() {
    let mut app = App::new(vec![]);
    app.animate = true;
    app.add_effect(EffectKind::FilePop(2), 4);
    assert_eq!(app.effect_progress(&EffectKind::FilePop(2)), Some(0.0));
    app.tick();
    app.tick();
    assert_eq!(app.effect_progress(&EffectKind::FilePop(2)), Some(0.5));
}

#[test]
fn effects_are_ignored_when_animation_is_off() {
    let mut app = App::new(vec![]);
    app.animate = false;
    app.add_effect(EffectKind::OpFlash(0), 10);
    assert!(app.effects.is_empty());
    assert!(app.effect_progress(&EffectKind::OpFlash(0)).is_none());
}

#[test]
fn needs_tick_only_while_something_moves() {
    let mut app = App::new(vec![entry("a.txt")]);
    app.animate = true;
    assert!(app.needs_tick(), "intro is running");

    for _ in 0..INTRO_TICKS {
        app.tick();
    }
    assert!(!app.needs_tick(), "idle after intro");

    app.selected[0] = true;
    app.operations
        .push(Operation::AddPrefix(AddPrefix { prefix: "x".into() }));
    app.update_preview();
    assert!(
        app.needs_tick(),
        "preview rows that change keep the arrow pulsing"
    );

    app.operations.clear();
    app.update_preview();
    assert!(!app.needs_tick());

    app.animate = false;
    app.add_effect(EffectKind::OpFlash(0), 10);
    assert!(!app.needs_tick(), "never ticks with animation off");
}

#[test]
fn intro_progress_ramps_then_completes() {
    let mut app = App::new(vec![]);
    app.animate = true;
    assert_eq!(app.intro_progress(), 0.0);
    for _ in 0..INTRO_TICKS / 2 {
        app.tick();
    }
    let mid = app.intro_progress();
    assert!(mid > 0.0 && mid < 1.0, "mid = {}", mid);
    for _ in 0..INTRO_TICKS {
        app.tick();
    }
    assert_eq!(app.intro_progress(), 1.0);

    let off = App::new(vec![]);
    assert_eq!(off.intro_progress(), 1.0, "no intro with animation off");
}

// --- Diff highlighting ------------------------------------------------------

#[test]
fn diff_segments_marks_only_the_changed_middle() {
    assert_eq!(
        diff_segments("IMG_0001.jpg", "IMG_0001.png"),
        vec![("IMG_0001.".to_string(), false), ("png".to_string(), true)]
    );
    assert_eq!(
        diff_segments("photo.jpg", "001_photo.jpg"),
        vec![("001_".to_string(), true), ("photo.jpg".to_string(), false)]
    );
    assert_eq!(
        diff_segments("same.txt", "same.txt"),
        vec![("same.txt".to_string(), false)]
    );
    assert_eq!(diff_segments("abc", "xyz"), vec![("xyz".to_string(), true)]);
    assert_eq!(diff_segments("abc", ""), Vec::<(String, bool)>::new());
}

#[test]
fn diff_segments_is_character_based() {
    assert_eq!(
        diff_segments("café.txt", "cafe.txt"),
        vec![
            ("caf".to_string(), false),
            ("e".to_string(), true),
            (".txt".to_string(), false)
        ]
    );
}

// --- Micro feedback ---------------------------------------------------------

#[test]
fn toggling_a_file_pops_and_adding_an_operation_flashes() {
    let mut app = App::new(vec![entry("a.txt"), entry("b.txt")]);
    app.animate = true;
    app.file_cursor = 1;
    key(&mut app, KeyCode::Char(' '));
    assert!(app.effect_progress(&EffectKind::FilePop(1)).is_some());

    app.input_mode = InputMode::SelectingOperationType;
    app.popup_type_cursor = T_ADD_PREFIX;
    key(&mut app, KeyCode::Enter);
    key(&mut app, KeyCode::Char('p'));
    key(&mut app, KeyCode::Enter);
    assert_eq!(app.operations.len(), 1);
    assert!(app.effect_progress(&EffectKind::OpFlash(0)).is_some());
}

// --- Rename cascade ---------------------------------------------------------

fn app_ready_to_rename(animate: bool) -> (tempfile::TempDir, App) {
    let dir = tempfile::tempdir().unwrap();
    for name in ["a.txt", "b.txt", "c.txt"] {
        fs::write(dir.path().join(name), "x").unwrap();
    }
    let mut app = App::new(files::discover_dir(dir.path()).unwrap());
    app.animate = animate;
    for _ in 0..INTRO_TICKS {
        app.tick();
    }
    app.toggle_select_all();
    app.operations.push(Operation::AddPrefix(AddPrefix {
        prefix: "new_".into(),
    }));
    app.update_preview();
    (dir, app)
}

#[test]
fn executing_shows_results_instead_of_quitting() {
    let (dir, mut app) = app_ready_to_rename(true);
    key(&mut app, KeyCode::Char('X'));

    assert!(app.executed);
    assert!(!app.should_quit, "stay open to show the cascade");
    assert_eq!(app.input_mode, InputMode::Done);
    let done = app.done.as_ref().expect("done state");
    assert_eq!(done.rows.len(), 3);
    assert_eq!(done.rows[0].old, "a.txt");
    assert_eq!(done.rows[0].new, "new_a.txt");
    assert!(done.rows.iter().all(|r| r.ok));
    assert!(app.status_message.contains("Renamed 3"));
    assert!(dir.path().join("new_c.txt").exists());
}

#[test]
fn cascade_reveals_rows_over_time_and_any_key_skips_then_quits() {
    let (_dir, mut app) = app_ready_to_rename(true);
    key(&mut app, KeyCode::Char('X'));

    assert_eq!(
        app.done_revealed(),
        0,
        "nothing revealed on the first frame"
    );
    let mut seen = vec![0];
    for _ in 0..200 {
        app.tick();
        let n = app.done_revealed();
        if n != *seen.last().unwrap() {
            seen.push(n);
        }
        if n == 3 {
            break;
        }
    }
    assert_eq!(seen, vec![0, 1, 2, 3], "rows appear one at a time");
    assert!(app.needs_tick(), "glow still fading");

    let (_dir2, mut fresh) = app_ready_to_rename(true);
    key(&mut fresh, KeyCode::Char('X'));
    fresh.tick();
    key(&mut fresh, KeyCode::Char('q'));
    assert_eq!(
        fresh.done_revealed(),
        3,
        "a key mid-cascade skips to the end"
    );
    assert!(!fresh.should_quit);
    key(&mut fresh, KeyCode::Char('q'));
    assert!(fresh.should_quit, "a key after the cascade quits");
}

#[test]
fn cascade_is_instant_with_animation_off() {
    let (_dir, mut app) = app_ready_to_rename(false);
    key(&mut app, KeyCode::Char('X'));
    assert_eq!(app.input_mode, InputMode::Done);
    assert_eq!(app.done_revealed(), 3);
    assert!(!app.needs_tick());
    key(&mut app, KeyCode::Enter);
    assert!(app.should_quit);
}

#[test]
fn cascade_row_glow_fades_after_reveal() {
    let (_dir, mut app) = app_ready_to_rename(true);
    key(&mut app, KeyCode::Char('X'));
    while app.done_revealed() < 1 {
        app.tick();
    }
    let first = app.done_row_glow(0).expect("revealed row glows");
    assert!(first > 0.9, "fresh reveal is bright: {}", first);
    assert!(app.done_row_glow(2).is_none(), "unrevealed row has no glow");
    for _ in 0..100 {
        app.tick();
    }
    assert_eq!(app.done_row_glow(0), Some(0.0), "glow fully faded");
}
