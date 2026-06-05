//! Integration tests for buffer utilities
use hermes::utilities::buffer::{
    buffer_get_lines, buffer_line_count, create_hidden_buffer, delete_buffer_force,
};

#[nvim_oxi::test]
fn create_hidden_buffer_returns_valid_buffer() -> nvim_oxi::Result<()> {
    let buf = create_hidden_buffer()
        .map_err(|e| nvim_oxi::api::Error::Other(format!("create_hidden_buffer failed: {}", e)))?;

    let bufs: Vec<_> = nvim_oxi::api::list_bufs().collect();
    assert!(
        bufs.iter().any(|b| b.handle() == buf.handle()),
        "Buffer should exist in list_bufs after creation"
    );

    Ok(())
}

#[nvim_oxi::test]
fn delete_buffer_force_removes_buffer() -> nvim_oxi::Result<()> {
    let buf = create_hidden_buffer()
        .map_err(|e| nvim_oxi::api::Error::Other(format!("create_hidden_buffer failed: {}", e)))?;
    let handle = buf.handle();

    delete_buffer_force(&buf)
        .map_err(|e| nvim_oxi::api::Error::Other(format!("delete_buffer_force failed: {}", e)))?;

    let bufs: Vec<_> = nvim_oxi::api::list_bufs().collect();
    assert!(
        !bufs.iter().any(|b| b.handle() == handle),
        "Buffer should no longer exist after force deletion"
    );

    Ok(())
}

#[nvim_oxi::test]
fn delete_buffer_force_errors_on_deleted_buffer() -> nvim_oxi::Result<()> {
    let buf = create_hidden_buffer()
        .map_err(|e| nvim_oxi::api::Error::Other(format!("create_hidden_buffer failed: {}", e)))?;

    // Delete once
    delete_buffer_force(&buf).map_err(|e| {
        nvim_oxi::api::Error::Other(format!("first delete_buffer_force failed: {}", e))
    })?;

    // Delete again — should error because buffer no longer exists
    let result = delete_buffer_force(&buf);
    assert!(
        result.is_err(),
        "Deleting an already-deleted buffer should error"
    );

    Ok(())
}

#[nvim_oxi::test]
fn buffer_line_count_returns_one_for_empty_buffer() -> nvim_oxi::Result<()> {
    let buf = create_hidden_buffer()
        .map_err(|e| nvim_oxi::api::Error::Other(format!("create_hidden_buffer failed: {}", e)))?;

    let count = buffer_line_count(&buf)
        .map_err(|e| nvim_oxi::api::Error::Other(format!("buffer_line_count failed: {}", e)))?;

    assert_eq!(count, 1, "Empty buffer should have 1 line (empty string)");

    Ok(())
}

#[nvim_oxi::test]
fn buffer_line_count_errors_on_deleted_buffer() -> nvim_oxi::Result<()> {
    let buf = create_hidden_buffer()
        .map_err(|e| nvim_oxi::api::Error::Other(format!("create_hidden_buffer failed: {}", e)))?;

    delete_buffer_force(&buf)
        .map_err(|e| nvim_oxi::api::Error::Other(format!("delete_buffer_force failed: {}", e)))?;

    let result = buffer_line_count(&buf);
    assert!(
        result.is_err(),
        "buffer_line_count on deleted buffer should error"
    );

    Ok(())
}

#[nvim_oxi::test]
fn buffer_get_lines_returns_empty_line_for_new_buffer() -> nvim_oxi::Result<()> {
    let buf = create_hidden_buffer()
        .map_err(|e| nvim_oxi::api::Error::Other(format!("create_hidden_buffer failed: {}", e)))?;

    let lines = buffer_get_lines(&buf, 0, 1, false)
        .map_err(|e| nvim_oxi::api::Error::Other(format!("buffer_get_lines failed: {}", e)))?;

    assert_eq!(
        lines,
        vec![""],
        "New buffer should have one empty line in range 0..1"
    );

    Ok(())
}

#[nvim_oxi::test]
fn buffer_get_lines_errors_on_deleted_buffer() -> nvim_oxi::Result<()> {
    let buf = create_hidden_buffer()
        .map_err(|e| nvim_oxi::api::Error::Other(format!("create_hidden_buffer failed: {}", e)))?;

    delete_buffer_force(&buf)
        .map_err(|e| nvim_oxi::api::Error::Other(format!("delete_buffer_force failed: {}", e)))?;

    let result = buffer_get_lines(&buf, 0, 1, false);
    assert!(
        result.is_err(),
        "buffer_get_lines on deleted buffer should error"
    );

    Ok(())
}
