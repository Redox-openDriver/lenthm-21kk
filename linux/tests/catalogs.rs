use std::collections::BTreeSet;

const SOURCES: [&str; 5] = [
    include_str!("../src/bin/lenthm-dev.rs"),
    include_str!("../src/bin/lenthmd.rs"),
    include_str!("../src/lib.rs"),
    include_str!("../../redox/src/bin/lenthmd.rs"),
    include_str!("../../redox/src/lib.rs"),
];

fn used() -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    for source in SOURCES {
        for piece in source.split('"').skip(1).step_by(2) {
            if ["dev-", "thermal-", "redox-"].iter().any(|prefix| piece.starts_with(prefix))
                && !piece.ends_with('-')
                && piece.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            {
                keys.insert(piece.to_string());
            }
        }
    }
    keys
}

#[test]
fn every_language_has_every_message_the_tools_name_with_the_same_variables() {
    let reference: BTreeSet<String> = drv_i18n::keys(lenthm_linux::LOCALES[0].1).into_iter().collect();
    for (language, source) in lenthm_linux::LOCALES {
        assert_eq!(drv_i18n::keys(source).into_iter().collect::<BTreeSet<_>>(), reference, "{language}");
        for key in &reference {
            assert_eq!(drv_i18n::variables(source, key), drv_i18n::variables(lenthm_linux::LOCALES[0].1, key), "{language}: {key}");
        }
    }
    let missing: Vec<String> = used().into_iter().filter(|key| !reference.contains(key)).collect();
    assert!(missing.is_empty(), "{missing:?}");
    let unused: Vec<&String> = reference.iter().filter(|key| !used().contains(*key)).collect();
    assert!(unused.is_empty(), "{unused:?}");
}
