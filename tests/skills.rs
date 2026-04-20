use std::fs;

use tempfile::TempDir;

use hermes_agent_rs::skill::SkillRegistry;

#[test]
fn empty_skills_dir_is_empty_registry() {
    let dir = TempDir::new().unwrap();
    let reg = SkillRegistry::load_dir(dir.path()).unwrap();
    assert!(reg.is_empty());
}

#[test]
fn single_skill_loads_and_renders() {
    let dir = TempDir::new().unwrap();
    let skill_dir = dir.path().join("demo");
    fs::create_dir(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("skill.md"),
        "---\nname: demo_skill\ndescription: A test skill\nwhen_to_use: when testing\n---\n\nDo the thing.\n",
    )
    .unwrap();

    let reg = SkillRegistry::load_dir(dir.path()).unwrap();
    assert!(!reg.is_empty());
    assert_eq!(reg.names(), vec!["demo_skill"]);
    let s = reg.render_system_suffix();
    assert!(s.contains("## demo_skill"));
    assert!(s.contains("Do the thing."));
    assert!(s.contains("When to use: when testing"));
}

#[test]
fn no_frontmatter_uses_dir_name_and_full_body() {
    let dir = TempDir::new().unwrap();
    let skill_dir = dir.path().join("plain_skill");
    fs::create_dir(&skill_dir).unwrap();
    fs::write(skill_dir.join("skill.md"), "Just body text.\n").unwrap();

    let reg = SkillRegistry::load_dir(dir.path()).unwrap();
    assert_eq!(reg.names(), vec!["plain_skill"]);
    let s = reg.render_system_suffix();
    assert!(s.contains("## plain_skill"));
    assert!(s.contains("Just body text."));
}

#[test]
fn invalid_frontmatter_falls_back_to_whole_file() {
    let dir = TempDir::new().unwrap();
    let skill_dir = dir.path().join("bad_yaml");
    fs::create_dir(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("skill.md"),
        "---\nthis is: not: valid: yaml: [\n---\nStill readable.\n",
    )
    .unwrap();

    let reg = SkillRegistry::load_dir(dir.path()).unwrap();
    assert_eq!(reg.names(), vec!["bad_yaml"]);
    let s = reg.render_system_suffix();
    assert!(s.contains("Still readable.") || s.contains("not: valid"));
}
