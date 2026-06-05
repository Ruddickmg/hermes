//! Integration tests for job utilities
use hermes::utilities::buffer::create_hidden_buffer;
use hermes::utilities::job::{start_job_in_buffer, stop_job};
use nvim_oxi::Dictionary;

#[nvim_oxi::test]
fn start_job_in_buffer_returns_positive_job_id() -> nvim_oxi::Result<()> {
    let buf = create_hidden_buffer()
        .map_err(|e| nvim_oxi::api::Error::Other(format!("create_hidden_buffer failed: {}", e)))?;

    let job_id = start_job_in_buffer(
        &buf,
        "echo".to_string(),
        vec!["hello".to_string()],
        Dictionary::new(),
    )
    .map_err(|e| nvim_oxi::api::Error::Other(format!("start_job_in_buffer failed: {}", e)))?;

    assert!(job_id > 0, "job_id should be positive, got: {}", job_id);

    Ok(())
}

#[nvim_oxi::test]
fn stop_job_terminates_sleep_job() -> nvim_oxi::Result<()> {
    let buf = create_hidden_buffer()
        .map_err(|e| nvim_oxi::api::Error::Other(format!("create_hidden_buffer failed: {}", e)))?;

    let job_id = start_job_in_buffer(
        &buf,
        "sleep".to_string(),
        vec!["10".to_string()],
        Dictionary::new(),
    )
    .map_err(|e| nvim_oxi::api::Error::Other(format!("start_job_in_buffer failed: {}", e)))?;

    let result = stop_job(job_id);
    assert!(
        result.is_ok(),
        "stop_job should succeed on a running job: {:?}",
        result
    );

    Ok(())
}

#[nvim_oxi::test]
fn stop_job_errors_for_invalid_job_id() -> nvim_oxi::Result<()> {
    let result = stop_job(-1);
    assert!(
        result.is_err(),
        "stop_job with invalid job_id should return an error"
    );

    Ok(())
}
