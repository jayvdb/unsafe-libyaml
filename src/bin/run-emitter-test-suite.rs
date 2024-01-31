#![warn(clippy::pedantic)]
#![allow(
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::items_after_statements,
    clippy::let_underscore_untyped,
    clippy::missing_errors_doc,
    clippy::missing_safety_doc,
    clippy::ptr_as_ptr,
    clippy::single_match_else,
    clippy::too_many_lines,
    clippy::unreadable_literal
)]

use libyaml_safer::{
    yaml_alias_event_initialize, yaml_document_end_event_initialize,
    yaml_document_start_event_initialize, yaml_emitter_delete, yaml_emitter_emit,
    yaml_emitter_initialize, yaml_emitter_set_canonical, yaml_emitter_set_output,
    yaml_emitter_set_unicode, yaml_emitter_t, yaml_event_t, yaml_mapping_end_event_initialize,
    yaml_mapping_start_event_initialize, yaml_scalar_event_initialize, yaml_scalar_style_t,
    yaml_sequence_end_event_initialize, yaml_sequence_start_event_initialize,
    yaml_stream_end_event_initialize, yaml_stream_start_event_initialize, YAML_ANY_SCALAR_STYLE,
    YAML_BLOCK_MAPPING_STYLE, YAML_BLOCK_SEQUENCE_STYLE, YAML_DOUBLE_QUOTED_SCALAR_STYLE,
    YAML_FOLDED_SCALAR_STYLE, YAML_LITERAL_SCALAR_STYLE, YAML_PLAIN_SCALAR_STYLE,
    YAML_SINGLE_QUOTED_SCALAR_STYLE, YAML_UTF8_ENCODING,
};
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{self, BufRead, Read, Write};
use std::mem::MaybeUninit;
use std::process::ExitCode;

pub(crate) unsafe fn unsafe_main(
    stdin: &mut dyn Read,
    stdout: &mut dyn Write,
) -> Result<(), Box<dyn Error>> {
    let mut emitter = MaybeUninit::<yaml_emitter_t>::uninit();
    if yaml_emitter_initialize(emitter.as_mut_ptr()).is_err() {
        return Err("Could not initalize the emitter object".into());
    }
    let mut emitter = emitter.assume_init();

    yaml_emitter_set_output(&mut emitter, stdout);
    yaml_emitter_set_canonical(&mut emitter, false);
    yaml_emitter_set_unicode(&mut emitter, false);

    let mut buf = std::io::BufReader::new(stdin);
    let mut line_buffer = String::with_capacity(1024);
    let mut value_buffer = String::with_capacity(128);

    let result = loop {
        let mut event = yaml_event_t::default();

        line_buffer.clear();
        let n = buf.read_line(&mut line_buffer)?;
        if n == 0 {
            break Ok(());
        }
        let line = line_buffer.strip_suffix('\n').unwrap_or(&line_buffer);

        let result = if line.starts_with("+STR") {
            yaml_stream_start_event_initialize(&mut event, YAML_UTF8_ENCODING)
        } else if line.starts_with("-STR") {
            yaml_stream_end_event_initialize(&mut event)
        } else if line.starts_with("+DOC") {
            let implicit = !line[4..].starts_with(" ---");
            yaml_document_start_event_initialize(&mut event, None, &[], implicit)
        } else if line.starts_with("-DOC") {
            let implicit = !line[4..].starts_with(" ...");
            yaml_document_end_event_initialize(&mut event, implicit)
        } else if line.starts_with("+MAP") {
            yaml_mapping_start_event_initialize(
                &mut event,
                get_anchor('&', line),
                get_tag(line),
                false,
                YAML_BLOCK_MAPPING_STYLE,
            )
        } else if line.starts_with("-MAP") {
            yaml_mapping_end_event_initialize(&mut event)
        } else if line.starts_with("+SEQ") {
            yaml_sequence_start_event_initialize(
                &mut event,
                get_anchor('&', line),
                get_tag(line),
                false,
                YAML_BLOCK_SEQUENCE_STYLE,
            )
        } else if line.starts_with("-SEQ") {
            yaml_sequence_end_event_initialize(&mut event)
        } else if line.starts_with("=VAL") {
            let mut style = YAML_ANY_SCALAR_STYLE;
            let value = get_value(line, &mut value_buffer, &mut style);
            let implicit = get_tag(line).is_none();
            yaml_scalar_event_initialize(
                &mut event,
                get_anchor('&', line),
                get_tag(line),
                value,
                implicit,
                implicit,
                style,
            )
        } else if line.starts_with("=ALI") {
            yaml_alias_event_initialize(&mut event, get_anchor('*', line).expect("no alias name"))
        } else {
            break Err(format!("Unknown event: '{line}'").into());
        };

        if result.is_err() {
            break Err("Memory error: Not enough memory for creating an event".into());
        }
        if let Err(err) = yaml_emitter_emit(&mut emitter, event) {
            break Err(err.into());
        }
    };

    yaml_emitter_delete(&mut emitter);
    result
}

fn get_anchor(sigil: char, line: &str) -> Option<&str> {
    let (_, from_sigil) = line.split_once(sigil)?;
    if let Some((until_space, _tail)) = from_sigil.split_once(' ') {
        Some(until_space)
    } else if !from_sigil.is_empty() {
        Some(from_sigil)
    } else {
        None
    }
}

fn get_tag(line: &str) -> Option<&str> {
    let (_, from_angle_open) = line.split_once('<')?;
    let (until_angle_close, _) = from_angle_open.split_once('>')?;
    Some(until_angle_close)
}

fn get_value<'a>(line: &str, buffer: &'a mut String, style: &mut yaml_scalar_style_t) -> &'a str {
    let mut remainder = line;
    let value = loop {
        let Some((_before, tail)) = remainder.split_once(' ') else {
            panic!("invalid line: {line}");
        };

        *style = match tail.chars().next().expect("string should not be empty") {
            ':' => YAML_PLAIN_SCALAR_STYLE,
            '\'' => YAML_SINGLE_QUOTED_SCALAR_STYLE,
            '"' => YAML_DOUBLE_QUOTED_SCALAR_STYLE,
            '|' => YAML_LITERAL_SCALAR_STYLE,
            '>' => YAML_FOLDED_SCALAR_STYLE,
            _ => {
                // This was an anchor, move to the next space.
                remainder = tail;
                continue;
            }
        };
        break &tail[1..];
    };

    buffer.clear();
    // Unescape the value
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            buffer.push(match chars.next().expect("unterminated escape sequence") {
                '\\' => '\\',
                '0' => '\0',
                'b' => '\x08',
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                otherwise => panic!("invalid escape character: {otherwise:?}"),
            });
        } else {
            buffer.push(ch);
        }
    }

    &*buffer
}

fn main() -> ExitCode {
    let args = env::args_os().skip(1);
    if args.len() == 0 {
        let _ = writeln!(
            io::stderr(),
            "Usage: run-emitter-test-suite <test.event>...",
        );
        return ExitCode::FAILURE;
    }
    for arg in args {
        let mut stdin = File::open(arg).unwrap();
        let mut stdout = io::stdout();
        let result = unsafe { unsafe_main(&mut stdin, &mut stdout) };
        if let Err(err) = result {
            let _ = writeln!(io::stderr(), "{}", err);
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}
