use super::*;
use std::os::unix::fs::MetadataExt;

#[test]
fn lookup_prefers_upper_then_ordered_lowers_and_merges_directories() {
    let temp = tempfile::tempdir().unwrap();
    let upper = temp.path().join("upper");
    let high = temp.path().join("high");
    let low = temp.path().join("low");
    fs::create_dir_all(upper.join("dir")).unwrap();
    fs::create_dir_all(high.join("dir")).unwrap();
    fs::create_dir_all(low.join("dir")).unwrap();
    fs::write(upper.join("dir/upper-only"), b"upper").unwrap();
    fs::write(upper.join("dir/shared"), b"upper").unwrap();
    fs::write(high.join("dir/shared"), b"high").unwrap();
    fs::write(low.join("dir/shared"), b"low").unwrap();
    fs::write(high.join("dir/layer-priority"), b"high").unwrap();
    fs::write(low.join("dir/layer-priority"), b"low").unwrap();
    fs::write(low.join("dir/low-only"), b"low").unwrap();
    let overlay = OverlayFs::new(upper.as_path(), [high.as_path(), low.as_path()]).unwrap();

    let shared = overlay.resolve(Path::new("dir/shared")).unwrap().unwrap();
    assert_eq!(shared.path, upper.join("dir/shared"));
    assert_eq!(fs::read(shared.path).unwrap(), b"upper");
    let prioritized = overlay
        .resolve(Path::new("dir/layer-priority"))
        .unwrap()
        .unwrap();
    assert_eq!(prioritized.path, high.join("dir/layer-priority"));
    let entries = overlay.readdir_entries(Path::new("dir")).unwrap();
    let names = entries
        .into_iter()
        .map(|(name, _)| name)
        .collect::<HashSet<_>>();
    assert!(names.contains(OsStr::new("upper-only")));
    assert!(names.contains(OsStr::new("shared")));
    assert!(names.contains(OsStr::new("layer-priority")));
    assert!(names.contains(OsStr::new("low-only")));
}

#[test]
fn copy_up_changes_only_the_upper_layer() {
    let temp = tempfile::tempdir().unwrap();
    let upper = temp.path().join("upper");
    let lower = temp.path().join("lower");
    fs::create_dir(&upper).unwrap();
    fs::create_dir(&lower).unwrap();
    fs::write(lower.join("config"), b"base").unwrap();
    let overlay = OverlayFs::new(upper.as_path(), [lower.as_path()]).unwrap();

    let copied = overlay.copy_up(Path::new("config")).unwrap();
    fs::write(&copied, b"changed").unwrap();

    assert_eq!(fs::read(&copied).unwrap(), b"changed");
    assert_eq!(fs::read(lower.join("config")).unwrap(), b"base");
}

#[test]
fn linking_a_lower_file_copies_it_up_and_creates_an_upper_hard_link() {
    let temp = tempfile::tempdir().unwrap();
    let upper = temp.path().join("upper");
    let lower = temp.path().join("lower");
    fs::create_dir(&upper).unwrap();
    fs::create_dir(&lower).unwrap();
    fs::write(lower.join("source"), b"value").unwrap();
    let overlay = OverlayFs::new(upper.as_path(), [lower.as_path()]).unwrap();

    overlay
        .link_path(Path::new("source"), Path::new("linked"))
        .unwrap();

    let source = fs::metadata(upper.join("source")).unwrap();
    let linked = fs::metadata(upper.join("linked")).unwrap();
    assert_eq!(source.ino(), linked.ino());
    assert_eq!(fs::read(lower.join("source")).unwrap(), b"value");
}

#[test]
fn unlink_persists_a_whiteout_across_reopen() {
    let temp = tempfile::tempdir().unwrap();
    let upper = temp.path().join("upper");
    let lower = temp.path().join("lower");
    fs::create_dir(&upper).unwrap();
    fs::create_dir(&lower).unwrap();
    fs::write(lower.join("removed"), b"lower").unwrap();

    {
        let overlay = OverlayFs::new(upper.as_path(), [lower.as_path()]).unwrap();
        overlay.unlink_path(Path::new("removed")).unwrap();
        assert!(overlay.resolve(Path::new("removed")).unwrap().is_none());
    }

    let reopened = OverlayFs::new(upper.as_path(), [lower.as_path()]).unwrap();
    assert!(reopened.resolve(Path::new("removed")).unwrap().is_none());
    assert!(lower.join("removed").exists());
}

#[test]
fn renaming_a_lower_file_copies_it_up_and_hides_the_old_name() {
    let temp = tempfile::tempdir().unwrap();
    let upper = temp.path().join("upper");
    let lower = temp.path().join("lower");
    fs::create_dir(&upper).unwrap();
    fs::create_dir(&lower).unwrap();
    fs::write(lower.join("old"), b"value").unwrap();
    let overlay = OverlayFs::new(upper.as_path(), [lower.as_path()]).unwrap();

    overlay
        .rename_path(Path::new("old"), Path::new("new"), RenameFlags::empty())
        .unwrap();

    assert!(overlay.resolve(Path::new("old")).unwrap().is_none());
    assert_eq!(fs::read(upper.join("new")).unwrap(), b"value");
    assert_eq!(fs::read(lower.join("old")).unwrap(), b"value");
}

#[test]
fn upper_directory_over_a_lower_file_can_be_renamed() {
    let temp = tempfile::tempdir().unwrap();
    let upper = temp.path().join("upper");
    let lower = temp.path().join("lower");
    fs::create_dir_all(upper.join("entry")).unwrap();
    fs::create_dir(&lower).unwrap();
    fs::write(lower.join("entry"), b"lower file").unwrap();
    let overlay = OverlayFs::new(upper.as_path(), [lower.as_path()]).unwrap();

    overlay
        .rename_path(Path::new("entry"), Path::new("moved"), RenameFlags::empty())
        .unwrap();

    assert!(upper.join("moved").is_dir());
    assert!(overlay.resolve(Path::new("entry")).unwrap().is_none());
    assert_eq!(fs::read(lower.join("entry")).unwrap(), b"lower file");
}

#[test]
fn paths_cannot_escape_the_overlay_root() {
    let temp = tempfile::tempdir().unwrap();
    let upper = temp.path().join("upper");
    fs::create_dir(&upper).unwrap();
    let overlay = OverlayFs::new(upper.as_path(), std::iter::empty::<&Path>()).unwrap();

    assert!(overlay.normalize_path(Path::new("../outside")).is_err());
    assert!(overlay.normalize_path(Path::new("/outside")).is_err());
    assert!(overlay.normalize_path(Path::new(CONTROL_DIR)).is_err());
    assert!(
        overlay
            .normalize_path(Path::new(".easy_fuser_overlay_tmp.entry"))
            .is_err()
    );
}
