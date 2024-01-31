use alloc::string::String;
use alloc::vec::Vec;

use crate::externs::memcpy;
use crate::ops::ForceAdd as _;
use crate::yaml::{size_t, YamlEventData, YamlNodeData};
use crate::{
    libc, yaml_break_t, yaml_document_t, yaml_emitter_t, yaml_encoding_t, yaml_event_t,
    yaml_mapping_style_t, yaml_mark_t, yaml_node_pair_t, yaml_node_t, yaml_parser_t,
    yaml_read_handler_t, yaml_scalar_style_t, yaml_sequence_style_t, yaml_tag_directive_t,
    yaml_token_t, yaml_version_directive_t, yaml_write_handler_t, PointerExt, YAML_ANY_ENCODING,
};
use core::ptr;

pub(crate) const INPUT_RAW_BUFFER_SIZE: usize = 16384;
pub(crate) const INPUT_BUFFER_SIZE: usize = INPUT_RAW_BUFFER_SIZE;
pub(crate) const OUTPUT_BUFFER_SIZE: usize = 16384;

/// Initialize a parser.
///
/// This function creates a new parser object. An application is responsible
/// for destroying the object using the yaml_parser_delete() function.
pub unsafe fn yaml_parser_initialize(parser: *mut yaml_parser_t) -> Result<(), ()> {
    __assert!(!parser.is_null());
    core::ptr::write(parser, yaml_parser_t::default());
    let parser = &mut *parser;
    parser.raw_buffer.reserve(INPUT_RAW_BUFFER_SIZE);
    parser.buffer.reserve(INPUT_BUFFER_SIZE);
    parser.tokens.reserve(16);
    parser.indents.reserve(16);
    parser.simple_keys.reserve(16);
    parser.states.reserve(16);
    parser.marks.reserve(16);
    parser.tag_directives.reserve(16);
    Ok(())
}

/// Destroy a parser.
pub unsafe fn yaml_parser_delete(parser: &mut yaml_parser_t) {
    parser.raw_buffer.clear();
    parser.buffer.clear();
    for mut token in parser.tokens.drain(..) {
        yaml_token_delete(&mut token);
    }
    parser.indents.clear();
    parser.simple_keys.clear();
    parser.states.clear();
    parser.marks.clear();
    parser.tag_directives.clear();
}

unsafe fn yaml_string_read_handler(
    data: *mut libc::c_void,
    buffer: *mut libc::c_uchar,
    mut size: size_t,
    size_read: *mut size_t,
) -> libc::c_int {
    let parser: &mut yaml_parser_t = &mut *(data as *mut yaml_parser_t);
    if parser.input.current == parser.input.end {
        *size_read = 0_u64;
        return 1;
    }
    if size > (*parser).input.end.c_offset_from(parser.input.current) as size_t {
        size = (*parser).input.end.c_offset_from(parser.input.current) as size_t;
    }
    memcpy(
        buffer as *mut libc::c_void,
        parser.input.current as *const libc::c_void,
        size,
    );
    parser.input.current = parser.input.current.wrapping_offset(size as isize);
    *size_read = size;
    1
}

/// Set a string input.
///
/// Note that the `input` pointer must be valid while the `parser` object
/// exists. The application is responsible for destroying `input` after
/// destroying the `parser`.
pub unsafe fn yaml_parser_set_input_string(
    parser: &mut yaml_parser_t,
    input: *const libc::c_uchar,
    size: size_t,
) {
    __assert!((parser.read_handler).is_none());
    __assert!(!input.is_null());
    parser.read_handler = Some(yaml_string_read_handler);
    let parser_ptr = parser as *mut _ as *mut libc::c_void;
    parser.read_handler_data = parser_ptr;
    parser.input.start = input;
    parser.input.current = input;
    parser.input.end = input.wrapping_offset(size as isize);
}

/// Set a generic input handler.
pub unsafe fn yaml_parser_set_input(
    parser: &mut yaml_parser_t,
    handler: yaml_read_handler_t,
    data: *mut libc::c_void,
) {
    __assert!((parser.read_handler).is_none());
    parser.read_handler = Some(handler);
    parser.read_handler_data = data;
}

/// Set the source encoding.
pub fn yaml_parser_set_encoding(parser: &mut yaml_parser_t, encoding: yaml_encoding_t) {
    __assert!(parser.encoding == YAML_ANY_ENCODING);
    parser.encoding = encoding;
}

/// Initialize an emitter.
///
/// This function creates a new emitter object. An application is responsible
/// for destroying the object using the yaml_emitter_delete() function.
pub unsafe fn yaml_emitter_initialize(emitter: *mut yaml_emitter_t) -> Result<(), ()> {
    __assert!(!emitter.is_null());
    core::ptr::write(emitter, yaml_emitter_t::default());
    let emitter = &mut *emitter;
    emitter.buffer.reserve(OUTPUT_BUFFER_SIZE);
    emitter.states.reserve(16);
    emitter.events.reserve(16);
    emitter.indents.reserve(16);
    emitter.tag_directives.reserve(16);
    Ok(())
}

/// Destroy an emitter.
pub unsafe fn yaml_emitter_delete(emitter: &mut yaml_emitter_t) {
    emitter.buffer.clear();
    emitter.raw_buffer.clear();
    emitter.states.clear();
    while let Some(mut event) = emitter.events.pop_front() {
        yaml_event_delete(&mut event);
    }
    emitter.indents.clear();
    emitter.tag_directives.clear();
    *emitter = yaml_emitter_t::default();
}

unsafe fn yaml_string_write_handler(
    data: *mut libc::c_void,
    buffer: *const libc::c_uchar,
    size: size_t,
) -> libc::c_int {
    let emitter = &mut *(data as *mut yaml_emitter_t);
    if emitter
        .output
        .size
        .wrapping_sub(*emitter.output.size_written)
        < size
    {
        memcpy(
            (*emitter)
                .output
                .buffer
                .wrapping_offset(*emitter.output.size_written as isize)
                as *mut libc::c_void,
            buffer as *const libc::c_void,
            (*emitter)
                .output
                .size
                .wrapping_sub(*emitter.output.size_written),
        );
        *emitter.output.size_written = emitter.output.size;
        return 0;
    }
    memcpy(
        (*emitter)
            .output
            .buffer
            .wrapping_offset(*emitter.output.size_written as isize) as *mut libc::c_void,
        buffer as *const libc::c_void,
        size,
    );
    let fresh153 = &mut (*emitter.output.size_written);
    *fresh153 = (*fresh153 as libc::c_ulong).force_add(size) as size_t;
    1
}

/// Set a string output.
///
/// The emitter will write the output characters to the `output` buffer of the
/// size `size`. The emitter will set `size_written` to the number of written
/// bytes. If the buffer is smaller than required, the emitter produces the
/// YAML_WRITE_ERROR error.
pub unsafe fn yaml_emitter_set_output_string(
    emitter: &mut yaml_emitter_t,
    output: *mut libc::c_uchar,
    size: size_t,
    size_written: *mut size_t,
) {
    __assert!((emitter.write_handler).is_none());
    __assert!(!output.is_null());
    emitter.write_handler = Some(yaml_string_write_handler);
    emitter.write_handler_data = emitter as *mut _ as *mut libc::c_void;
    emitter.output.buffer = output;
    emitter.output.size = size;
    emitter.output.size_written = size_written;
    *size_written = 0_u64;
}

/// Set a generic output handler.
pub unsafe fn yaml_emitter_set_output(
    emitter: &mut yaml_emitter_t,
    handler: yaml_write_handler_t,
    data: *mut libc::c_void,
) {
    __assert!(emitter.write_handler.is_none());
    emitter.write_handler = Some(handler);
    emitter.write_handler_data = data;
}

/// Set the output encoding.
pub fn yaml_emitter_set_encoding(emitter: &mut yaml_emitter_t, encoding: yaml_encoding_t) {
    __assert!(emitter.encoding == YAML_ANY_ENCODING);
    emitter.encoding = encoding;
}

/// Set if the output should be in the "canonical" format as in the YAML
/// specification.
pub fn yaml_emitter_set_canonical(emitter: &mut yaml_emitter_t, canonical: bool) {
    emitter.canonical = canonical;
}

/// Set the indentation increment.
pub fn yaml_emitter_set_indent(emitter: &mut yaml_emitter_t, indent: libc::c_int) {
    emitter.best_indent = if 1 < indent && indent < 10 { indent } else { 2 };
}

/// Set the preferred line width. -1 means unlimited.
pub fn yaml_emitter_set_width(emitter: &mut yaml_emitter_t, width: libc::c_int) {
    emitter.best_width = if width >= 0 { width } else { -1 };
}

/// Set if unescaped non-ASCII characters are allowed.
pub fn yaml_emitter_set_unicode(emitter: &mut yaml_emitter_t, unicode: bool) {
    emitter.unicode = unicode;
}

/// Set the preferred line break.
pub fn yaml_emitter_set_break(emitter: &mut yaml_emitter_t, line_break: yaml_break_t) {
    emitter.line_break = line_break;
}

/// Free any memory allocated for a token object.
pub unsafe fn yaml_token_delete(token: &mut yaml_token_t) {
    *token = yaml_token_t::default();
}

/// Create the STREAM-START event.
pub fn yaml_stream_start_event_initialize(
    event: &mut yaml_event_t,
    encoding: yaml_encoding_t,
) -> Result<(), ()> {
    *event = yaml_event_t {
        data: YamlEventData::StreamStart { encoding },
        ..Default::default()
    };
    Ok(())
}

/// Create the STREAM-END event.
pub fn yaml_stream_end_event_initialize(event: &mut yaml_event_t) -> Result<(), ()> {
    *event = yaml_event_t {
        data: YamlEventData::StreamEnd,
        ..Default::default()
    };
    Ok(())
}

/// Create the DOCUMENT-START event.
///
/// The `implicit` argument is considered as a stylistic parameter and may be
/// ignored by the emitter.
pub unsafe fn yaml_document_start_event_initialize(
    event: &mut yaml_event_t,
    version_directive: Option<yaml_version_directive_t>,
    tag_directives_in: &[yaml_tag_directive_t],
    implicit: bool,
) -> Result<(), ()> {
    let tag_directives = Vec::from_iter(tag_directives_in.iter().cloned());

    *event = yaml_event_t {
        data: YamlEventData::DocumentStart {
            version_directive,
            tag_directives,
            implicit,
        },
        ..Default::default()
    };

    Ok(())
}

/// Create the DOCUMENT-END event.
///
/// The `implicit` argument is considered as a stylistic parameter and may be
/// ignored by the emitter.
pub fn yaml_document_end_event_initialize(
    event: &mut yaml_event_t,
    implicit: bool,
) -> Result<(), ()> {
    *event = yaml_event_t {
        data: YamlEventData::DocumentEnd { implicit },
        ..Default::default()
    };
    Ok(())
}

/// Create an ALIAS event.
pub unsafe fn yaml_alias_event_initialize(
    event: &mut yaml_event_t,
    anchor: &str,
) -> Result<(), ()> {
    *event = yaml_event_t {
        data: YamlEventData::Alias {
            anchor: String::from(anchor),
        },
        ..Default::default()
    };
    Ok(())
}

/// Create a SCALAR event.
///
/// The `style` argument may be ignored by the emitter.
///
/// Either the `tag` attribute or one of the `plain_implicit` and
/// `quoted_implicit` flags must be set.
///
pub unsafe fn yaml_scalar_event_initialize(
    event: &mut yaml_event_t,
    anchor: Option<&str>,
    tag: Option<&str>,
    value: &str,
    plain_implicit: bool,
    quoted_implicit: bool,
    style: yaml_scalar_style_t,
) -> Result<(), ()> {
    let mark = yaml_mark_t {
        index: 0_u64,
        line: 0_u64,
        column: 0_u64,
    };
    let mut anchor_copy: Option<String> = None;
    let mut tag_copy: Option<String> = None;

    if let Some(anchor) = anchor {
        anchor_copy = Some(String::from(anchor));
    }
    if let Some(tag) = tag {
        tag_copy = Some(String::from(tag));
    }

    *event = yaml_event_t {
        data: YamlEventData::Scalar {
            anchor: anchor_copy,
            tag: tag_copy,
            value: String::from(value),
            plain_implicit,
            quoted_implicit,
            style,
        },
        start_mark: mark,
        end_mark: mark,
    };
    Ok(())
}

/// Create a SEQUENCE-START event.
///
/// The `style` argument may be ignored by the emitter.
///
/// Either the `tag` attribute or the `implicit` flag must be set.
pub unsafe fn yaml_sequence_start_event_initialize(
    event: &mut yaml_event_t,
    anchor: Option<&str>,
    tag: Option<&str>,
    implicit: bool,
    style: yaml_sequence_style_t,
) -> Result<(), ()> {
    let mut anchor_copy: Option<String> = None;
    let mut tag_copy: Option<String> = None;

    if let Some(anchor) = anchor {
        anchor_copy = Some(String::from(anchor));
    }
    if let Some(tag) = tag {
        tag_copy = Some(String::from(tag));
    }

    *event = yaml_event_t {
        data: YamlEventData::SequenceStart {
            anchor: anchor_copy,
            tag: tag_copy,
            implicit,
            style,
        },
        ..Default::default()
    };
    return Ok(());
}

/// Create a SEQUENCE-END event.
pub fn yaml_sequence_end_event_initialize(event: &mut yaml_event_t) -> Result<(), ()> {
    *event = yaml_event_t {
        data: YamlEventData::SequenceEnd,
        ..Default::default()
    };
    Ok(())
}

/// Create a MAPPING-START event.
///
/// The `style` argument may be ignored by the emitter.
///
/// Either the `tag` attribute or the `implicit` flag must be set.
pub unsafe fn yaml_mapping_start_event_initialize(
    event: &mut yaml_event_t,
    anchor: Option<&str>,
    tag: Option<&str>,
    implicit: bool,
    style: yaml_mapping_style_t,
) -> Result<(), ()> {
    let mut anchor_copy: Option<String> = None;
    let mut tag_copy: Option<String> = None;

    if let Some(anchor) = anchor {
        anchor_copy = Some(String::from(anchor));
    }

    if let Some(tag) = tag {
        tag_copy = Some(String::from(tag));
    }

    *event = yaml_event_t {
        data: YamlEventData::MappingStart {
            anchor: anchor_copy,
            tag: tag_copy,
            implicit,
            style,
        },
        ..Default::default()
    };

    Ok(())
}

/// Create a MAPPING-END event.
pub fn yaml_mapping_end_event_initialize(event: &mut yaml_event_t) -> Result<(), ()> {
    *event = yaml_event_t {
        data: YamlEventData::MappingEnd,
        ..Default::default()
    };
    Ok(())
}

/// Free any memory allocated for an event object.
pub unsafe fn yaml_event_delete(event: &mut yaml_event_t) {
    *event = Default::default();
}

/// Create a YAML document.
pub unsafe fn yaml_document_initialize(
    document: &mut yaml_document_t,
    version_directive: Option<yaml_version_directive_t>,
    tag_directives_in: &[yaml_tag_directive_t],
    start_implicit: bool,
    end_implicit: bool,
) -> Result<(), ()> {
    let nodes = Vec::with_capacity(16);
    let tag_directives = Vec::from_iter(tag_directives_in.iter().cloned());

    *document = yaml_document_t {
        nodes,
        version_directive,
        tag_directives,
        start_implicit,
        end_implicit,
        ..Default::default()
    };

    return Ok(());
}

/// Delete a YAML document and all its nodes.
pub unsafe fn yaml_document_delete(document: &mut yaml_document_t) {
    document.nodes.clear();
    document.version_directive = None;
    document.tag_directives.clear();
}

/// Get a node of a YAML document.
///
/// The pointer returned by this function is valid until any of the functions
/// modifying the documents are called.
///
/// Returns the node object or NULL if `index` is out of range.
pub unsafe fn yaml_document_get_node(
    document: &mut yaml_document_t,
    index: libc::c_int,
) -> *mut yaml_node_t {
    if index > 0 && index as usize <= document.nodes.len() {
        return &mut document.nodes[index as usize - 1] as *mut _;
    }
    ptr::null_mut()
}

/// Get the root of a YAML document node.
///
/// The root object is the first object added to the document.
///
/// The pointer returned by this function is valid until any of the functions
/// modifying the documents are called.
///
/// An empty document produced by the parser signifies the end of a YAML stream.
///
/// Returns the node object or NULL if the document is empty.
pub unsafe fn yaml_document_get_root_node(document: &mut yaml_document_t) -> *mut yaml_node_t {
    if let Some(root) = document.nodes.get_mut(0) {
        root as _
    } else {
        ptr::null_mut()
    }
}

/// Create a SCALAR node and attach it to the document.
///
/// The `style` argument may be ignored by the emitter.
///
/// Returns the node id or 0 on error.
#[must_use]
pub unsafe fn yaml_document_add_scalar(
    document: &mut yaml_document_t,
    tag: Option<&str>,
    value: &str,
    style: yaml_scalar_style_t,
) -> libc::c_int {
    let mark = yaml_mark_t {
        index: 0_u64,
        line: 0_u64,
        column: 0_u64,
    };
    let tag = tag.unwrap_or("tag:yaml.org,2002:str");
    let tag_copy = String::from(tag);
    let value_copy = String::from(value);
    let node = yaml_node_t {
        data: YamlNodeData::Scalar {
            value: value_copy,
            style,
        },
        tag: Some(tag_copy),
        start_mark: mark,
        end_mark: mark,
    };
    document.nodes.push(node);
    document.nodes.len() as libc::c_int
}

/// Create a SEQUENCE node and attach it to the document.
///
/// The `style` argument may be ignored by the emitter.
///
/// Returns the node id or 0 on error.
#[must_use]
pub unsafe fn yaml_document_add_sequence(
    document: &mut yaml_document_t,
    tag: Option<&str>,
    style: yaml_sequence_style_t,
) -> libc::c_int {
    let mark = yaml_mark_t {
        index: 0_u64,
        line: 0_u64,
        column: 0_u64,
    };

    let items = Vec::with_capacity(16);
    let tag = tag.unwrap_or("tag:yaml.org,2002:seq");
    let tag_copy = String::from(tag);
    let node = yaml_node_t {
        data: YamlNodeData::Sequence { items, style },
        tag: Some(tag_copy),
        start_mark: mark,
        end_mark: mark,
    };
    document.nodes.push(node);
    document.nodes.len() as libc::c_int
}

/// Create a MAPPING node and attach it to the document.
///
/// The `style` argument may be ignored by the emitter.
///
/// Returns the node id or 0 on error.
#[must_use]
pub unsafe fn yaml_document_add_mapping(
    document: &mut yaml_document_t,
    tag: Option<&str>,
    style: yaml_mapping_style_t,
) -> libc::c_int {
    let mark = yaml_mark_t {
        index: 0_u64,
        line: 0_u64,
        column: 0_u64,
    };
    let pairs = Vec::with_capacity(16);
    let tag = tag.unwrap_or("tag:yaml.org,2002:map");
    let tag_copy = String::from(tag);

    let node = yaml_node_t {
        data: YamlNodeData::Mapping { pairs, style },
        tag: Some(tag_copy),
        start_mark: mark,
        end_mark: mark,
    };

    document.nodes.push(node);
    document.nodes.len() as libc::c_int
}

/// Add an item to a SEQUENCE node.
pub unsafe fn yaml_document_append_sequence_item(
    document: &mut yaml_document_t,
    sequence: libc::c_int,
    item: libc::c_int,
) -> Result<(), ()> {
    __assert!(sequence > 0 && sequence as usize - 1 < document.nodes.len());
    __assert!(matches!(
        &document.nodes[sequence as usize - 1].data,
        YamlNodeData::Sequence { .. }
    ));
    __assert!(item > 0 && item as usize - 1 < document.nodes.len());
    if let YamlNodeData::Sequence { ref mut items, .. } =
        &mut document.nodes[sequence as usize - 1].data
    {
        items.push(item);
    }
    Ok(())
}

/// Add a pair of a key and a value to a MAPPING node.
pub unsafe fn yaml_document_append_mapping_pair(
    document: &mut yaml_document_t,
    mapping: libc::c_int,
    key: libc::c_int,
    value: libc::c_int,
) -> Result<(), ()> {
    __assert!(mapping > 0 && mapping as usize - 1 < document.nodes.len());
    __assert!(matches!(
        &document.nodes[mapping as usize - 1].data,
        YamlNodeData::Mapping { .. }
    ));
    __assert!(key > 0 && key as usize - 1 < document.nodes.len());
    __assert!(value > 0 && value as usize - 1 < document.nodes.len());
    let pair = yaml_node_pair_t { key, value };
    if let YamlNodeData::Mapping { ref mut pairs, .. } =
        &mut document.nodes[mapping as usize - 1].data
    {
        pairs.push(pair);
    }
    Ok(())
}
