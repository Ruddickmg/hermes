use agent_client_protocol::{
    Content, ContentBlock, Diff, Terminal, TerminalId, TextContent, ToolCallContent,
};
use hermes::nvim::event::tool_call_content::parse_tool_call_content;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_parse_tool_call_content_ok() {
    let content =
        ToolCallContent::Content(Content::new(ContentBlock::Text(TextContent::new("Hello"))));
    let result = parse_tool_call_content(content);
    assert!(result.is_ok());
}

#[test]
fn test_parse_tool_call_content_text_content() {
    let content = ToolCallContent::Content(Content::new(ContentBlock::Text(TextContent::new(
        "Hello world",
    ))));
    let dict = parse_tool_call_content(content).unwrap();

    let text = dict.get("text").unwrap();
    assert_eq!(*text, nvim_oxi::Object::from("Hello world"));

    let type_field = dict.get("type").unwrap();
    assert_eq!(*type_field, nvim_oxi::Object::from("text"));
}

#[test]
fn test_parse_tool_call_content_image_content() {
    let content = ToolCallContent::Content(Content::new(ContentBlock::Image(
        agent_client_protocol::ImageContent::new("base64data", "image/png"),
    )));
    let dict = parse_tool_call_content(content).unwrap();

    let type_field = dict.get("type").unwrap();
    assert_eq!(*type_field, nvim_oxi::Object::from("image"));
}

#[test]
fn test_parse_tool_call_content_terminal() {
    let terminal = Terminal::new(TerminalId::new("term_123"));
    let content = ToolCallContent::Terminal(terminal);
    let dict = parse_tool_call_content(content).unwrap();

    let id = dict.get("id").unwrap();
    assert_eq!(*id, nvim_oxi::Object::from("term_123"));

    let type_field = dict.get("type").unwrap();
    assert_eq!(*type_field, nvim_oxi::Object::from("terminal"));
}

#[test]
fn test_parse_tool_call_content_diff_without_old_text() {
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(b"original content").unwrap();
    let path = temp_file.path().to_path_buf();

    let diff = Diff::new(path, "new content".to_string());
    let content = ToolCallContent::Diff(diff);
    let dict = parse_tool_call_content(content).unwrap();

    let type_field = dict.get("type").unwrap();
    assert_eq!(*type_field, nvim_oxi::Object::from("diff"));

    let path_field = dict.get("path").unwrap();
    assert_eq!(*path_field, nvim_oxi::Object::from("original content"));

    let new_text = dict.get("new_text").unwrap();
    assert_eq!(*new_text, nvim_oxi::Object::from("new content"));

    assert!(dict.get("old_text").is_none());
}

#[test]
fn test_parse_tool_call_content_diff_with_old_text() {
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(b"old content").unwrap();
    let path = temp_file.path().to_path_buf();

    let diff = Diff::new(path, "new content".to_string()).old_text(Some("old content".to_string()));
    let content = ToolCallContent::Diff(diff);
    let dict = parse_tool_call_content(content).unwrap();

    let type_field = dict.get("type").unwrap();
    assert_eq!(*type_field, nvim_oxi::Object::from("diff"));

    let old_text = dict.get("old_text").unwrap();
    assert_eq!(*old_text, nvim_oxi::Object::from("old content"));

    let new_text = dict.get("new_text").unwrap();
    assert_eq!(*new_text, nvim_oxi::Object::from("new content"));
}
