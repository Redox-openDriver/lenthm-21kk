use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

const SCRIPTS: [&str; 3] = ["install.sh", "uninstall.sh", "messages.sh"];
const LOCALES: [(&str, &str); 2] = [("en", include_str!("../dist/locales/en.ftl")), ("ru", include_str!("../dist/locales/ru.ftl"))];

fn dist() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("dist")
}

#[test]
fn every_script_parses_in_the_shells_at_hand() {
    let mut checked = 0;
    for shell in ["sh", "dash", "bash"] {
        if Command::new(shell).arg("-c").arg("exit 0").status().is_err() {
            continue;
        }
        for script in SCRIPTS {
            let parsed = Command::new(shell).arg("-n").arg(dist().join(script)).output().unwrap();
            assert!(parsed.status.success(), "{shell} {script}: {}", String::from_utf8_lossy(&parsed.stderr));
            checked += 1;
        }
    }
    assert!(checked > 0);
}

#[test]
fn every_message_the_scripts_say_is_in_both_catalogs() {
    let mut said = BTreeSet::new();
    for script in ["install.sh", "uninstall.sh"] {
        let text = std::fs::read_to_string(dist().join(script)).unwrap();
        for line in text.lines() {
            for verb in ["say ", "fail "] {
                if let Some(rest) = line.trim().strip_prefix(verb).or_else(|| line.split("|| ").nth(1).and_then(|part| part.trim().strip_prefix(verb)))
                    && let Some(key) = rest.split_whitespace().next().filter(|key| key.contains('-'))
                {
                    said.insert(key.to_string());
                }
            }
        }
    }
    let reference: BTreeSet<String> = drv_i18n::keys(LOCALES[0].1).into_iter().collect();
    for (language, source) in LOCALES {
        assert_eq!(drv_i18n::keys(source).into_iter().collect::<BTreeSet<_>>(), reference, "{language}");
        for key in &reference {
            assert_eq!(drv_i18n::variables(source, key), drv_i18n::variables(LOCALES[0].1, key), "{language}: {key}");
        }
    }
    let missing: Vec<&String> = said.iter().filter(|key| !reference.contains(*key)).collect();
    assert!(missing.is_empty(), "{missing:?}");
    let unused: Vec<&String> = reference.iter().filter(|key| !said.contains(*key)).collect();
    assert!(unused.is_empty(), "{unused:?}");
}
