use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum AudioKind {
    Music,
    SoundEffect,
    Cry,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AudioProgramRef {
    pub kind: AudioKind,
    pub asset_id: String,
}

impl AudioProgramRef {
    pub fn new(kind: AudioKind, asset_id: impl Into<String>) -> Result<Self> {
        let reference = Self {
            kind,
            asset_id: asset_id.into(),
        };
        reference.validate()?;
        Ok(reference)
    }

    pub fn music(asset_id: impl Into<String>) -> Result<Self> {
        Self::new(AudioKind::Music, asset_id)
    }

    pub fn validate(&self) -> Result<()> {
        validate_audio_asset_id(self.kind, &self.asset_id)
    }
}

impl<'de> Deserialize<'de> for AudioProgramRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawAudioProgramRef {
            kind: AudioKind,
            asset_id: String,
        }

        let raw = RawAudioProgramRef::deserialize(deserializer)?;
        let reference = Self {
            kind: raw.kind,
            asset_id: raw.asset_id,
        };
        reference.validate().map_err(serde::de::Error::custom)?;
        Ok(reference)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioPointerTable {
    pub music: Vec<String>,
    pub sfx: Vec<String>,
    pub cries: Vec<String>,
}

impl AudioPointerTable {
    pub fn from_audio_pointers_text(text: &str) -> Result<Self> {
        let sections = parse_pointer_sections(text);
        Ok(Self {
            music: required_pointer_section(&sections, "Music")?,
            sfx: required_pointer_section(&sections, "SFX")?,
            cries: required_pointer_section(&sections, "Cries")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioPcmFormat {
    pub sample_rate_hz: u32,
    pub channels: u8,
    pub bits_per_sample: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum AudioProgramSource {
    Midi(Vec<u8>),
    Pcm {
        bytes: Vec<u8>,
        format: AudioPcmFormat,
        loop_start_sample: Option<usize>,
        loop_end_sample: Option<usize>,
    },
    PcmFile {
        path: PathBuf,
        format: AudioPcmFormat,
        byte_len: usize,
        payload_hash: String,
        loop_start_sample: Option<usize>,
        loop_end_sample: Option<usize>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioProgram {
    pub cache_key: String,
    pub source: AudioProgramSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioCommand {
    pub command: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioSource {
    pub number: Option<u8>,
    pub commands: Vec<AudioCommand>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParsedAudioProgram {
    pub channel_count: u8,
    pub channels: BTreeMap<String, AudioSource>,
    pub subroutines: BTreeMap<String, AudioSource>,
}

pub struct AudioRepository {
    audio_root: PathBuf,
}

impl AudioRepository {
    pub fn new(repository_root: impl AsRef<Path>) -> Self {
        Self {
            audio_root: repository_root.as_ref().join("apps/web/assets/data/audio"),
        }
    }

    pub fn from_audio_root(audio_root: impl Into<PathBuf>) -> Self {
        Self {
            audio_root: audio_root.into(),
        }
    }

    pub fn load_pointer_table(&self) -> Result<AudioPointerTable> {
        Ok(AudioPointerTable {
            music: self.load_mid_entries("music")?,
            sfx: self.load_mid_entries("sfx")?,
            cries: self.load_mid_entries("cries")?,
        })
    }

    fn load_mid_entries(&self, namespace: &str) -> Result<Vec<String>> {
        let mut entries = Vec::new();
        let directory = self.audio_root.join(namespace);
        let kind = audio_kind_for_namespace(namespace)
            .with_context(|| format!("unknown audio namespace '{namespace}'"))?;
        for entry in std::fs::read_dir(&directory)
            .with_context(|| format!("read audio directory {}", directory.display()))?
        {
            let entry = entry.with_context(|| format!("read {}", directory.display()))?;
            let path = entry.path();
            if path.is_dir() {
                continue;
            }
            let extension = path
                .extension()
                .and_then(|extension| extension.to_str())
                .with_context(|| {
                    format!("audio file {} is missing an extension", path.display())
                })?;
            if extension != "mid" {
                anyhow::bail!(
                    "audio file {} must use the .mid extension; no alternate audio formats are supported",
                    path.display()
                );
            }
            let stem = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .with_context(|| format!("audio file {} has no UTF-8 stem", path.display()))?;
            validate_audio_asset_id(kind, stem)?;
            entries.push(stem.to_string());
        }
        entries.sort();
        Ok(entries)
    }

    pub fn build_program(&self, kind: AudioKind, asset_id: &str) -> Result<AudioProgram> {
        if asset_id.is_empty() {
            anyhow::bail!("audio program id must be explicit");
        }
        validate_audio_asset_id(kind, asset_id)?;

        match kind {
            AudioKind::Music => self.load_midi_program("music", "music", asset_id),
            AudioKind::SoundEffect => self.load_midi_program("sfx", "sfx", asset_id),
            AudioKind::Cry => self.load_midi_program("cry", "cries", asset_id),
        }
    }

    fn load_midi_program(
        &self,
        namespace: &str,
        directory: &str,
        stem: &str,
    ) -> Result<AudioProgram> {
        let midi_path = self.audio_root.join(directory).join(format!("{stem}.mid"));
        let source = std::fs::read(&midi_path)
            .with_context(|| format!("read MIDI audio program {}", midi_path.display()))?;
        if !source.starts_with(b"MThd") {
            anyhow::bail!("audio program {} is not a MIDI file", midi_path.display());
        }
        Ok(AudioProgram {
            cache_key: format!("{namespace}:{}", midi_path.display()),
            source: AudioProgramSource::Midi(source),
        })
    }
}

fn audio_kind_for_namespace(namespace: &str) -> Option<AudioKind> {
    match namespace {
        "music" => Some(AudioKind::Music),
        "sfx" => Some(AudioKind::SoundEffect),
        "cries" => Some(AudioKind::Cry),
        _ => None,
    }
}

fn validate_audio_asset_id(kind: AudioKind, asset_id: &str) -> Result<()> {
    let prefix = match kind {
        AudioKind::Music => "MUSIC_",
        AudioKind::SoundEffect => "SFX_",
        AudioKind::Cry => "CRY_",
    };
    let valid = asset_id.starts_with(prefix)
        && asset_id
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_');
    if !valid {
        anyhow::bail!("audio asset id '{asset_id}' must use exact {prefix}* modpack id");
    }
    let payload = &asset_id[prefix.len()..];
    if payload.starts_with("FALLBACK") || payload.starts_with("LEGACY") {
        anyhow::bail!("audio asset id '{asset_id}' uses reserved runtime pack prefix");
    }
    Ok(())
}

pub fn extract_asm_program(source_text: &str, entry_label: &str) -> Option<String> {
    let lines: Vec<&str> = source_text.lines().collect();
    let label_index = label_indices(&lines);
    let mut queue = VecDeque::from([entry_label.to_string()]);
    let mut seen = BTreeMap::<String, ()>::new();
    let mut blocks = Vec::new();

    while let Some(label) = queue.pop_front() {
        if seen.contains_key(&label) {
            continue;
        }
        seen.insert(label.clone(), ());

        let Some((start, end)) = label_block_bounds(&lines, &label_index, &label) else {
            continue;
        };
        let block = lines[start..end].join("\n");
        for line in block
            .lines()
            .map(clean_asm_line)
            .filter(|line| !line.is_empty())
        {
            if let Some(target) = parse_command_target(&line, "channel") {
                queue.push_back(target);
            }
            if let Some(target) = parse_command_target(&line, "sound_call") {
                queue.push_back(resolve_local_label(&label, &target));
            }
        }
        blocks.push(block);
    }

    (!blocks.is_empty()).then(|| blocks.join("\n\n"))
}

pub fn parse_audio_program(source_text: &str) -> Result<ParsedAudioProgram> {
    let mut parsed = ParsedAudioProgram {
        channel_count: 0,
        channels: BTreeMap::new(),
        subroutines: BTreeMap::new(),
    };
    let mut current_label: Option<String> = None;
    let mut channel_count_seen = false;

    for raw in source_text.lines() {
        let line = clean_asm_line(raw);
        if line.is_empty() {
            continue;
        }
        if let Some(label) = exact_label(&line) {
            current_label = Some(label.to_string());
            if label.contains("_Ch") {
                parsed
                    .channels
                    .entry(label.to_string())
                    .or_insert(AudioSource {
                        number: channel_number_from_label(label),
                        commands: Vec::new(),
                    });
            } else if label.contains(".sub") || label.starts_with('.') {
                parsed
                    .subroutines
                    .entry(label.to_string())
                    .or_insert(AudioSource {
                        number: None,
                        commands: Vec::new(),
                    });
            }
            continue;
        }

        let (command, args) = split_command(&line);
        if command == "channel_count" {
            if channel_count_seen {
                anyhow::bail!("audio program declares channel_count more than once");
            }
            channel_count_seen = true;
            if args.len() != 1 {
                anyhow::bail!(
                    "audio program channel_count requires exactly one argument, found {}",
                    args.len()
                );
            }
            parsed.channel_count = args[0]
                .parse::<u8>()
                .with_context(|| format!("invalid audio channel_count '{}'", args[0]))?;
            if parsed.channel_count == 0 {
                anyhow::bail!("audio program channel_count must be positive");
            }
        }

        if let Some(label) = &current_label {
            let target = if parsed.channels.contains_key(label) {
                parsed.channels.get_mut(label)
            } else {
                parsed.subroutines.get_mut(label)
            };
            if let Some(source) = target {
                source.commands.push(AudioCommand { command, args });
            }
        }
    }

    if !channel_count_seen {
        anyhow::bail!("audio program is missing channel_count");
    }
    if parsed.channels.len() != usize::from(parsed.channel_count) {
        anyhow::bail!(
            "audio program channel_count {} does not match {} parsed channels",
            parsed.channel_count,
            parsed.channels.len()
        );
    }

    Ok(parsed)
}

fn parse_pointer_sections(text: &str) -> BTreeMap<String, Vec<String>> {
    let mut sections: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut current_section: Option<String> = None;
    for raw in text.lines() {
        let line = clean_asm_line(raw);
        if line.is_empty() {
            continue;
        }
        if let Some(label) = line.strip_suffix(':') {
            if matches!(label, "Music" | "SFX" | "Cries") {
                current_section = Some(label.to_string());
                sections.entry(label.to_string()).or_default();
            }
            continue;
        }
        if let Some(section) = &current_section
            && let Some(rest) = line.strip_prefix("dba ")
        {
            sections
                .entry(section.clone())
                .or_default()
                .push(rest.trim().to_string());
        }
    }
    sections
}

fn required_pointer_section(
    sections: &BTreeMap<String, Vec<String>>,
    section: &str,
) -> Result<Vec<String>> {
    sections
        .get(section)
        .cloned()
        .with_context(|| format!("audio pointer table is missing required {section} section"))
}

fn label_indices(lines: &[&str]) -> BTreeMap<String, usize> {
    let mut labels = BTreeMap::new();
    let mut scope: Option<String> = None;
    for (index, line) in lines.iter().enumerate() {
        let clean = clean_asm_line(line);
        let Some(label) = exact_label(&clean) else {
            continue;
        };
        if label.starts_with('.') {
            if let Some(scope) = &scope {
                labels.insert(format!("{scope}{label}"), index);
            }
        } else {
            scope = Some(label.to_string());
            labels.insert(label.to_string(), index);
        }
    }
    labels
}

fn label_block_bounds(
    lines: &[&str],
    label_index: &BTreeMap<String, usize>,
    label: &str,
) -> Option<(usize, usize)> {
    let start = *label_index.get(label)?;
    let scoped_local = label.contains('.');
    let end = lines
        .iter()
        .enumerate()
        .skip(start + 1)
        .find_map(|(index, line)| {
            let clean = clean_asm_line(line);
            let found = exact_label(&clean)?;
            if scoped_local || !found.starts_with('.') {
                Some(index)
            } else {
                None
            }
        })
        .unwrap_or(lines.len());
    Some((start, end))
}

fn parse_command_target(line: &str, command: &str) -> Option<String> {
    let (found, args) = split_command(line);
    if found != command {
        return None;
    }
    if command == "channel" {
        return args.get(1).cloned();
    }
    args.first().cloned()
}

fn resolve_local_label(owner: &str, target: &str) -> String {
    if target.starts_with('.') {
        format!("{owner}{target}")
    } else {
        target.to_string()
    }
}

fn split_command(line: &str) -> (String, Vec<String>) {
    let mut parts = line.splitn(2, char::is_whitespace);
    let command = parts.next().unwrap_or_default().trim().to_string();
    let args = parts
        .next()
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|arg| !arg.is_empty())
        .map(ToOwned::to_owned)
        .collect();
    (command, args)
}

fn clean_asm_line(line: &str) -> String {
    line.split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn channel_number_from_label(label: &str) -> Option<u8> {
    label
        .rsplit_once("_Ch")
        .and_then(|(_, number)| number.parse().ok())
}

fn exact_label(line: &str) -> Option<&str> {
    if let Some(label) = line.strip_suffix(':') {
        return label.chars().all(is_label_char).then_some(label);
    }
    (line.starts_with('.') && line.chars().all(is_label_char)).then_some(line)
}

fn is_label_char(c: char) -> bool {
    c == '_' || c == '.' || c.is_ascii_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static AUDIO_FIXTURE_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn repository_root_for_tests() -> PathBuf {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest_dir
            .ancestors()
            .nth(3)
            .expect("workspace is nested under rust/crates/crystal-audio")
            .to_path_buf()
    }

    fn generated_midi_audio_root() -> PathBuf {
        let root = repository_root_for_tests();
        let fixture = root.join("apps/web/test-fixtures/audio/route29.mid");
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let counter = AUDIO_FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temp = std::env::temp_dir().join(format!(
            "crystal-audio-midi-fixture-{}-{unique}-{counter}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&temp);
        for directory in ["music", "sfx", "cries"] {
            std::fs::create_dir_all(temp.join(directory)).expect("create temp audio namespace");
        }
        std::fs::copy(&fixture, temp.join("music/MUSIC_ROUTE_29.mid"))
            .expect("copy music MIDI fixture");
        std::fs::copy(&fixture, temp.join("sfx/SFX_TACKLE.mid")).expect("copy sfx MIDI fixture");
        std::fs::copy(fixture, temp.join("cries/CRY_NIDORAN_M.mid"))
            .expect("copy cry MIDI fixture");
        temp
    }

    #[test]
    fn discovers_audio_entries_from_generated_mid_files() {
        let temp = generated_midi_audio_root();

        let table = AudioRepository::from_audio_root(&temp)
            .load_pointer_table()
            .expect("load pointer table");

        assert_eq!(table.music, vec!["MUSIC_ROUTE_29".to_string()]);
        assert_eq!(table.sfx, vec!["SFX_TACKLE".to_string()]);
        assert_eq!(table.cries, vec!["CRY_NIDORAN_M".to_string()]);
        let _ = std::fs::remove_dir_all(temp);
    }

    #[test]
    fn audio_discovery_rejects_lowercase_stems_instead_of_coercing_ids() {
        let temp = generated_midi_audio_root();
        std::fs::copy(
            repository_root_for_tests().join("apps/web/test-fixtures/audio/route29.mid"),
            temp.join("music/route29.mid"),
        )
        .expect("copy lowercase music fixture");

        let error = AudioRepository::from_audio_root(&temp)
            .load_pointer_table()
            .expect_err("lowercase audio stem must not be accepted")
            .to_string();

        assert!(
            error.contains("must use exact MUSIC_* modpack id"),
            "{error}"
        );
        let _ = std::fs::remove_dir_all(temp);
    }

    #[test]
    fn audio_discovery_rejects_reserved_runtime_identity_prefixes() {
        let temp = generated_midi_audio_root();
        std::fs::copy(
            repository_root_for_tests().join("apps/web/test-fixtures/audio/route29.mid"),
            temp.join("music/MUSIC_FALLBACK_ROUTE_29.mid"),
        )
        .expect("copy reserved music fixture");

        let error = AudioRepository::from_audio_root(&temp)
            .load_pointer_table()
            .expect_err("reserved audio asset ids must not be accepted")
            .to_string();

        assert!(
            error.contains("uses reserved runtime pack prefix"),
            "{error}"
        );
        let _ = std::fs::remove_dir_all(temp);
    }

    #[test]
    fn audio_discovery_rejects_non_mid_files_instead_of_ignoring_them() {
        let temp = generated_midi_audio_root();
        std::fs::copy(
            repository_root_for_tests().join("apps/web/test-fixtures/audio/route29.mid"),
            temp.join("music/ignored.midi"),
        )
        .expect("copy unsupported MIDI extension fixture");
        std::fs::write(temp.join("cries/CRY_NIDORAN_M.mp3"), b"not supported")
            .expect("write unsupported cry format");
        std::fs::write(temp.join("sfx/SFX_TACKLE.MID"), b"MThd")
            .expect("write case-changed MIDI extension");

        let error = AudioRepository::from_audio_root(&temp)
            .load_pointer_table()
            .expect_err("audio discovery must reject unsupported formats")
            .to_string();

        assert!(error.contains("must use the .mid extension"));
        assert!(
            error.contains("ignored.midi")
                || error.contains("CRY_NIDORAN_M.mp3")
                || error.contains("SFX_TACKLE.MID"),
            "{error}"
        );
        let _ = std::fs::remove_dir_all(temp);
    }

    #[test]
    fn builds_music_program_from_mid_file() {
        let temp = generated_midi_audio_root();
        let repo = AudioRepository::from_audio_root(&temp);
        let program = repo
            .build_program(AudioKind::Music, "MUSIC_ROUTE_29")
            .expect("build program");
        assert!(program.cache_key.contains("MUSIC_ROUTE_29.mid"));
        assert!(!program.cache_key.contains(".mp3"));
        match program.source {
            AudioProgramSource::Midi(bytes) => assert!(bytes.starts_with(b"MThd")),
            AudioProgramSource::Pcm { .. } | AudioProgramSource::PcmFile { .. } => {
                panic!("MIDI repository must not emit PCM")
            }
        }
        let _ = std::fs::remove_dir_all(temp);
    }

    #[test]
    fn builds_sfx_program_from_mid_file() {
        let temp = generated_midi_audio_root();
        let repo = AudioRepository::from_audio_root(&temp);
        let sfx = repo
            .build_program(AudioKind::SoundEffect, "SFX_TACKLE")
            .expect("build sfx");
        assert!(sfx.cache_key.contains("SFX_TACKLE.mid"));
        match sfx.source {
            AudioProgramSource::Midi(bytes) => assert!(bytes.starts_with(b"MThd")),
            AudioProgramSource::Pcm { .. } | AudioProgramSource::PcmFile { .. } => {
                panic!("MIDI repository must not emit PCM")
            }
        }
        let _ = std::fs::remove_dir_all(temp);
    }

    #[test]
    fn builds_cry_program_from_mid_file() {
        let temp = generated_midi_audio_root();
        let repo = AudioRepository::from_audio_root(&temp);
        let cry = repo
            .build_program(AudioKind::Cry, "CRY_NIDORAN_M")
            .expect("build cry");
        assert!(cry.cache_key.contains("CRY_NIDORAN_M.mid"));
        assert!(!cry.cache_key.contains(".mp3"));
        match cry.source {
            AudioProgramSource::Midi(bytes) => assert!(bytes.starts_with(b"MThd")),
            AudioProgramSource::Pcm { .. } | AudioProgramSource::PcmFile { .. } => {
                panic!("MIDI repository must not emit PCM")
            }
        }
        let _ = std::fs::remove_dir_all(temp);
    }

    #[test]
    fn build_program_rejects_empty_or_missing_midi_stems_without_none_fallback() {
        let temp = generated_midi_audio_root();
        let repo = AudioRepository::from_audio_root(&temp);

        let empty = repo
            .build_program(AudioKind::Music, "")
            .expect_err("empty stem is invalid")
            .to_string();
        assert!(empty.contains("id must be explicit"));

        let lowercase = repo
            .build_program(AudioKind::Music, "route29")
            .expect_err("lowercase asset id is invalid")
            .to_string();
        assert!(
            lowercase.contains("must use exact MUSIC_* modpack id"),
            "{lowercase}"
        );

        let reserved = repo
            .build_program(AudioKind::Cry, "CRY_LEGACY_NIDORAN_M")
            .expect_err("reserved cry asset id is invalid")
            .to_string();
        assert!(
            reserved.contains("uses reserved runtime pack prefix"),
            "{reserved}"
        );

        let missing = repo
            .build_program(AudioKind::Cry, "CRY_MISSING")
            .expect_err("missing cry MIDI is invalid")
            .to_string();
        assert!(missing.contains("read MIDI audio program"));
        assert!(missing.contains("CRY_MISSING.mid"));
        let _ = std::fs::remove_dir_all(temp);
    }

    #[test]
    fn pointer_tables_require_music_sfx_and_cries_sections() {
        let table = AudioPointerTable::from_audio_pointers_text(
            r#"
Music:
    dba Music_Route29
SFX:
    dba Sfx_Tackle
Cries:
    dba Cry_NidoranM
"#,
        )
        .expect("complete pointer table");
        assert_eq!(table.music, vec!["Music_Route29".to_string()]);
        assert_eq!(table.sfx, vec!["Sfx_Tackle".to_string()]);
        assert_eq!(table.cries, vec!["Cry_NidoranM".to_string()]);

        let error = AudioPointerTable::from_audio_pointers_text(
            r#"
Music:
    dba Music_Route29
SFX:
    dba Sfx_Tackle
"#,
        )
        .expect_err("missing cries section must not become empty")
        .to_string();
        assert!(error.contains("missing required Cries section"));
    }

    #[test]
    fn asm_audio_extraction_scopes_local_labels_without_rewriting_source() {
        let source = r#"
Music_Test:
	channel_count 1
	channel 1, Music_Test_Ch1

Music_Test_Ch1:
	sound_call .sub1
.mainloop
	note C_, 1
	sound_loop 0, .mainloop
.sub1:
	note D_, 1
	sound_ret

Music_Other:
	channel_count 1
	channel 1, Music_Other_Ch1
"#;

        let extracted = extract_asm_program(source, "Music_Test").expect("extract exact program");

        assert!(extracted.contains(".mainloop\n"), "{extracted}");
        assert!(!extracted.contains(".mainloop:"), "{extracted}");
        assert!(extracted.contains(".sub1:"), "{extracted}");
        assert!(extracted.contains("sound_call .sub1"), "{extracted}");
        assert!(!extracted.contains("Music_Other:"), "{extracted}");

        let parsed = parse_audio_program(&extracted).expect("parse extracted audio");
        assert_eq!(parsed.channel_count, 1);
        assert!(parsed.channels.contains_key("Music_Test_Ch1"));
        assert!(parsed.subroutines.contains_key(".mainloop"));
        assert!(parsed.subroutines.contains_key(".sub1"));
    }

    #[test]
    fn asm_audio_extraction_rejects_missing_top_level_label_colon() {
        let source = r#"
Music_Test
	channel_count 1
	channel 1, Music_Test_Ch1

Music_Test_Ch1:
	sound_ret
"#;

        assert_eq!(extract_asm_program(source, "Music_Test"), None);
    }

    #[test]
    fn parsed_audio_program_requires_exact_channel_count() {
        let missing = parse_audio_program(
            r#"
Music_Test:
    channel 1, Music_Test_Ch1

Music_Test_Ch1:
    sound_ret
"#,
        )
        .expect_err("missing channel_count must not default to zero")
        .to_string();
        assert!(missing.contains("missing channel_count"), "{missing}");

        let malformed = parse_audio_program(
            r#"
Music_Test:
    channel_count many
    channel 1, Music_Test_Ch1

Music_Test_Ch1:
    sound_ret
"#,
        )
        .expect_err("malformed channel_count must not be ignored")
        .to_string();
        assert!(
            malformed.contains("invalid audio channel_count"),
            "{malformed}"
        );

        let mismatch = parse_audio_program(
            r#"
Music_Test:
    channel_count 2
    channel 1, Music_Test_Ch1

Music_Test_Ch1:
    sound_ret
"#,
        )
        .expect_err("channel_count must match parsed channels")
        .to_string();
        assert!(
            mismatch.contains("does not match 1 parsed channels"),
            "{mismatch}"
        );
    }

    #[test]
    fn audio_json_rejects_unknown_mp3_and_fallback_fields() {
        let reference = serde_json::from_value::<AudioProgramRef>(serde_json::json!({
            "kind": "music",
            "asset_id": "MUSIC_ROUTE_29"
        }))
        .expect("valid exact audio ref");
        assert_eq!(
            reference,
            AudioProgramRef::music("MUSIC_ROUTE_29").expect("checked constructor")
        );

        let lowercase_ref = serde_json::from_value::<AudioProgramRef>(serde_json::json!({
            "kind": "music",
            "asset_id": "route29"
        }))
        .expect_err("audio refs must reject lowercase ids")
        .to_string();
        assert!(
            lowercase_ref.contains("must use exact MUSIC_* modpack id"),
            "{lowercase_ref}"
        );

        let wrong_kind_ref = serde_json::from_value::<AudioProgramRef>(serde_json::json!({
            "kind": "cry",
            "asset_id": "MUSIC_ROUTE_29"
        }))
        .expect_err("audio refs must reject kind/id prefix mismatches")
        .to_string();
        assert!(
            wrong_kind_ref.contains("must use exact CRY_* modpack id"),
            "{wrong_kind_ref}"
        );

        let reserved_ref = AudioProgramRef::music("MUSIC_FALLBACK_ROUTE_29")
            .expect_err("audio refs must reject reserved ids")
            .to_string();
        assert!(
            reserved_ref.contains("uses reserved runtime pack prefix"),
            "{reserved_ref}"
        );

        let ref_error = serde_json::from_value::<AudioProgramRef>(serde_json::json!({
            "kind": "music",
            "asset_id": "route29",
            "mp3": "route29.mp3"
        }))
        .expect_err("audio refs must not accept alternate formats")
        .to_string();
        assert!(ref_error.contains("unknown field `mp3`"), "{ref_error}");

        let program_error = serde_json::from_value::<ParsedAudioProgram>(serde_json::json!({
            "channel_count": 1,
            "channels": {
                "Ch1": {
                    "number": 1,
                    "commands": [
                        {"command": "tempo", "args": ["128"], "fallback": true}
                    ]
                }
            },
            "subroutines": {}
        }))
        .expect_err("parsed audio commands must not accept fallback fields")
        .to_string();
        assert!(
            program_error.contains("unknown field `fallback`"),
            "{program_error}"
        );
    }
}
