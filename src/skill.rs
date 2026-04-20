use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub when_to_use: Option<String>,
    pub body: String,
}

pub struct SkillRegistry {
    skills: Vec<Skill>,
}

#[derive(Debug, Deserialize, Default)]
struct SkillMeta {
    name: Option<String>,
    description: Option<String>,
    when_to_use: Option<String>,
}

impl SkillRegistry {
    pub fn empty() -> Self {
        Self { skills: vec![] }
    }

    pub fn load_dir(root: &Path) -> anyhow::Result<Self> {
        let mut skills = Vec::new();
        if !root.is_dir() {
            return Ok(Self { skills });
        }
        let mut dirs: Vec<_> = std::fs::read_dir(root)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .collect();
        dirs.sort_by_key(|e| e.file_name());

        for entry in dirs {
            let dir = entry.path();
            let skill_md = dir.join("skill.md");
            if !skill_md.is_file() {
                continue;
            }
            let text = std::fs::read_to_string(&skill_md)?;
            let dir_name = dir
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("skill")
                .to_string();
            skills.push(parse_skill_md(&dir_name, &text));
        }

        Ok(Self { skills })
    }

    pub fn render_system_suffix(&self) -> String {
        let mut out = String::from(
            "--- SKILLS ---\nYou have access to the following skills. Apply the relevant ones based on the conversation.\n\n",
        );
        for s in &self.skills {
            out.push_str(&format!("## {}\n", s.name));
            if !s.description.is_empty() {
                out.push_str(&format!("{}\n", s.description));
            }
            if let Some(w) = &s.when_to_use {
                out.push_str(&format!("When to use: {}\n\n", w));
            } else {
                out.push('\n');
            }
            out.push_str(&s.body);
            out.push_str("\n\n");
        }
        out.trim_end().to_string()
    }

    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }

    pub fn names(&self) -> Vec<&str> {
        self.skills.iter().map(|s| s.name.as_str()).collect()
    }
}

fn extract_frontmatter(text: &str) -> Option<(&str, &str)> {
    let text = text.strip_prefix("---\n").or_else(|| text.strip_prefix("---\r\n"))?;
    let (end, sep_len) = if let Some(i) = text.find("\n---\n") {
        (i, 5)
    } else if let Some(i) = text.find("\r\n---\r\n") {
        (i, 7)
    } else {
        return None;
    };
    let fm = &text[..end];
    let after = &text[end + sep_len..];
    Some((fm, after))
}

fn parse_skill_md(dir_name: &str, text: &str) -> Skill {
    if let Some((fm, body)) = extract_frontmatter(text) {
        match serde_yaml::from_str::<SkillMeta>(fm) {
            Ok(m) => Skill {
                name: m.name.unwrap_or_else(|| dir_name.to_string()),
                description: m.description.unwrap_or_default(),
                when_to_use: m.when_to_use,
                body: body.trim().to_string(),
            },
            Err(_) => Skill {
                name: dir_name.to_string(),
                description: String::new(),
                when_to_use: None,
                body: text.to_string(),
            },
        }
    } else {
        Skill {
            name: dir_name.to_string(),
            description: String::new(),
            when_to_use: None,
            body: text.to_string(),
        }
    }
}
