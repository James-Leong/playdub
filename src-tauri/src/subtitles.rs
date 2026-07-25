use serde::{Deserialize, Serialize};
use std::fmt::Write as _;

const MAX_CUES: usize = 100_000;
const MAX_CUE_TEXT_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SubtitleSegment {
    pub id: u32,
    pub start_ms: u64,
    pub end_ms: u64,
    pub source_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translated_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Cue {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

pub(crate) fn parse_srt(input: &str) -> Result<Vec<Cue>, String> {
    let normalized = input
        .strip_prefix('\u{feff}')
        .unwrap_or(input)
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    let mut cues = Vec::new();
    let mut block = Vec::new();

    for line in normalized.lines().chain(std::iter::once("")) {
        if line.trim().is_empty() {
            if block.is_empty() {
                continue;
            }
            parse_srt_block(&block, &mut cues)?;
            block.clear();
            continue;
        }
        block.push(line);
    }
    cues.sort_by_key(|cue| (cue.start_ms, cue.end_ms));
    Ok(cues)
}

fn parse_srt_block(lines: &[&str], cues: &mut Vec<Cue>) -> Result<(), String> {
    let timing_index = lines
        .iter()
        .position(|line| line.contains("-->"))
        .ok_or_else(|| "An SRT cue is missing its timestamp line".to_owned())?;
    if timing_index > 1 {
        return Err("An SRT cue contains unexpected content before its timestamps".to_owned());
    }

    let (start_ms, end_ms) = parse_timing_line(lines[timing_index])?;
    let text = normalize_cue_text(&lines[timing_index + 1..].join("\n"))?;
    if text.is_empty() {
        return Ok(());
    }

    cues.push(Cue {
        start_ms,
        end_ms,
        text,
    });
    if cues.len() > MAX_CUES {
        return Err(format!("Subtitle output exceeds the {MAX_CUES} cue limit"));
    }
    Ok(())
}

fn parse_timing_line(line: &str) -> Result<(u64, u64), String> {
    let (start, remainder) = line
        .split_once("-->")
        .ok_or_else(|| "Invalid SRT timestamp separator".to_owned())?;
    let end = remainder
        .split_ascii_whitespace()
        .next()
        .ok_or_else(|| "An SRT cue is missing its end timestamp".to_owned())?;
    let start_ms = parse_timestamp(start.trim())?;
    let end_ms = parse_timestamp(end.trim())?;
    if end_ms <= start_ms {
        return Err("An SRT cue must end after it starts".to_owned());
    }
    Ok((start_ms, end_ms))
}

fn parse_timestamp(value: &str) -> Result<u64, String> {
    let (clock, fraction) = value
        .rsplit_once([',', '.'])
        .ok_or_else(|| format!("Invalid SRT timestamp: {value}"))?;
    if fraction.is_empty()
        || fraction.len() > 3
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(format!("Invalid SRT millisecond field: {value}"));
    }

    let mut parts = clock.split(':');
    let hours = parse_clock_part(parts.next(), value)?;
    let minutes = parse_clock_part(parts.next(), value)?;
    let seconds = parse_clock_part(parts.next(), value)?;
    if parts.next().is_some() || minutes >= 60 || seconds >= 60 {
        return Err(format!("Invalid SRT clock field: {value}"));
    }

    let fraction_value = fraction
        .parse::<u64>()
        .map_err(|_| format!("Invalid SRT millisecond field: {value}"))?;
    let millis = match fraction.len() {
        1 => fraction_value * 100,
        2 => fraction_value * 10,
        3 => fraction_value,
        _ => unreachable!("fraction length was validated"),
    };

    hours
        .checked_mul(3_600_000)
        .and_then(|total| total.checked_add(minutes * 60_000))
        .and_then(|total| total.checked_add(seconds * 1_000))
        .and_then(|total| total.checked_add(millis))
        .ok_or_else(|| format!("SRT timestamp is too large: {value}"))
}

fn parse_clock_part(part: Option<&str>, original: &str) -> Result<u64, String> {
    let part = part.ok_or_else(|| format!("Invalid SRT timestamp: {original}"))?;
    if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(format!("Invalid SRT timestamp: {original}"));
    }
    part.parse::<u64>()
        .map_err(|_| format!("Invalid SRT timestamp: {original}"))
}

fn normalize_cue_text(value: &str) -> Result<String, String> {
    if value.len() > MAX_CUE_TEXT_BYTES {
        return Err(format!(
            "A subtitle cue exceeds the {MAX_CUE_TEXT_BYTES}-byte text limit"
        ));
    }
    Ok(value
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n"))
}

pub(crate) fn align_translation(source: &[Cue], translation: &[Cue]) -> Vec<Option<String>> {
    let mut assigned = vec![Vec::<&str>::new(); source.len()];

    for translated_cue in translation {
        let best = source
            .iter()
            .enumerate()
            .map(|(index, source_cue)| {
                let overlap = overlap_ms(source_cue, translated_cue);
                let distance = midpoint(source_cue).abs_diff(midpoint(translated_cue));
                (index, overlap, distance)
            })
            .filter(|(_, overlap, distance)| *overlap > 0 || *distance <= 2_000)
            .max_by(|left, right| {
                left.1
                    .cmp(&right.1)
                    .then_with(|| right.2.cmp(&left.2))
                    .then_with(|| right.0.cmp(&left.0))
            });

        if let Some((index, _, _)) = best {
            assigned[index].push(translated_cue.text.as_str());
        }
    }

    assigned
        .into_iter()
        .map(|parts| {
            if parts.is_empty() {
                None
            } else {
                Some(parts.join(" "))
            }
        })
        .collect()
}

fn overlap_ms(left: &Cue, right: &Cue) -> u64 {
    left.end_ms
        .min(right.end_ms)
        .saturating_sub(left.start_ms.max(right.start_ms))
}

fn midpoint(cue: &Cue) -> u64 {
    cue.start_ms + (cue.end_ms - cue.start_ms) / 2
}

pub(crate) fn render_srt(segments: &[SubtitleSegment]) -> Result<String, String> {
    let mut output = String::new();
    for (index, segment) in segments.iter().enumerate() {
        validate_segment(segment)?;
        let text = export_text(segment)?;
        writeln!(
            output,
            "{}\n{} --> {}\n{}\n",
            index + 1,
            format_timestamp(segment.start_ms, ','),
            format_timestamp(segment.end_ms, ','),
            text
        )
        .expect("writing to a String cannot fail");
    }
    Ok(output)
}

pub(crate) fn render_vtt(segments: &[SubtitleSegment]) -> Result<String, String> {
    let mut output = String::from("WEBVTT\n\n");
    for segment in segments {
        validate_segment(segment)?;
        let text = export_text(segment)?;
        writeln!(
            output,
            "{} --> {}\n{}\n",
            format_timestamp(segment.start_ms, '.'),
            format_timestamp(segment.end_ms, '.'),
            text
        )
        .expect("writing to a String cannot fail");
    }
    Ok(output)
}

fn validate_segment(segment: &SubtitleSegment) -> Result<(), String> {
    if segment.end_ms <= segment.start_ms {
        return Err(format!(
            "Subtitle segment {} must end after it starts",
            segment.id
        ));
    }
    normalize_cue_text(&segment.source_text)?;
    if let Some(text) = &segment.translated_text {
        normalize_cue_text(text)?;
    }
    Ok(())
}

fn export_text(segment: &SubtitleSegment) -> Result<String, String> {
    let value = segment
        .translated_text
        .as_deref()
        .filter(|text| !text.trim().is_empty())
        .unwrap_or(&segment.source_text);
    let normalized = normalize_cue_text(value)?;
    if normalized.is_empty() {
        return Err(format!("Subtitle segment {} has no text", segment.id));
    }
    Ok(normalized)
}

fn format_timestamp(milliseconds: u64, separator: char) -> String {
    let hours = milliseconds / 3_600_000;
    let minutes = (milliseconds / 60_000) % 60;
    let seconds = (milliseconds / 1_000) % 60;
    let millis = milliseconds % 1_000;
    format!("{hours:02}:{minutes:02}:{seconds:02}{separator}{millis:03}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cue(start_ms: u64, end_ms: u64, text: &str) -> Cue {
        Cue {
            start_ms,
            end_ms,
            text: text.to_owned(),
        }
    }

    #[test]
    fn parses_bom_windows_lines_optional_indices_and_dot_milliseconds() {
        let input = "\u{feff}1\r\n00:00:01,2 --> 00:00:03,450\r\nHello\r\nworld\r\n\r\n\
                     \t \r\n00:00:04.000 --> 00:00:05.010 align:start\r\nSecond cue\r\n";
        assert_eq!(
            parse_srt(input).unwrap(),
            vec![
                cue(1_200, 3_450, "Hello\nworld"),
                cue(4_000, 5_010, "Second cue")
            ]
        );
    }

    #[test]
    fn rejects_invalid_or_reversed_timestamps() {
        assert!(parse_srt("1\n00:61:00,000 --> 00:62:00,000\nbad\n").is_err());
        assert!(parse_srt("1\n00:00:02,000 --> 00:00:01,000\nbad\n").is_err());
    }

    #[test]
    fn aligns_split_translation_cues_using_overlap() {
        let source = vec![cue(0, 2_000, "one"), cue(2_000, 5_000, "two")];
        let translated = vec![
            cue(0, 900, "uno"),
            cue(900, 2_000, "primero"),
            cue(2_100, 4_900, "dos"),
        ];
        assert_eq!(
            align_translation(&source, &translated),
            vec![Some("uno primero".to_owned()), Some("dos".to_owned())]
        );
    }

    #[test]
    fn does_not_attach_distant_translation_cues() {
        let source = vec![cue(0, 1_000, "one")];
        let translated = vec![cue(10_000, 11_000, "far away")];
        assert_eq!(align_translation(&source, &translated), vec![None]);
    }

    #[test]
    fn renders_deterministic_srt_and_vtt_with_translation_fallback() {
        let segments = vec![
            SubtitleSegment {
                id: 7,
                start_ms: 1_002,
                end_ms: 2_345,
                source_text: "Source".to_owned(),
                translated_text: Some("Translation".to_owned()),
            },
            SubtitleSegment {
                id: 8,
                start_ms: 3_000,
                end_ms: 4_000,
                source_text: "Fallback".to_owned(),
                translated_text: None,
            },
        ];
        assert_eq!(
            render_srt(&segments).unwrap(),
            "1\n00:00:01,002 --> 00:00:02,345\nTranslation\n\n\
             2\n00:00:03,000 --> 00:00:04,000\nFallback\n\n"
        );
        assert_eq!(
            render_vtt(&segments).unwrap(),
            "WEBVTT\n\n00:00:01.002 --> 00:00:02.345\nTranslation\n\n\
             00:00:03.000 --> 00:00:04.000\nFallback\n\n"
        );
    }
}
