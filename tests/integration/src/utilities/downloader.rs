//! Integration tests for the Downloader

use hermes::utilities::{Downloader, NotificationMessenger};
use std::io::Read;

/// A small, stable endpoint for verifying download behaviour.
const TEST_URL: &str = "https://httpbin.org/get";

#[nvim_oxi::test]
fn download_to_string_returns_valid_utf8() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");
    let downloader = Downloader::new(messenger);

    let result = downloader.download_to_string(
        TEST_URL,
        "test-download-string",
        "Testing download to string",
    );

    assert!(
        result.is_ok(),
        "download_to_string should succeed: {:?}",
        result.err()
    );
    let body = result.unwrap();
    assert!(
        body.contains("https://httpbin.org/get"),
        "Response should contain the requested URL"
    );
}

#[nvim_oxi::test]
fn download_to_file_writes_all_bytes() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");
    let downloader = Downloader::new(messenger);

    let temp_dir = std::env::temp_dir();
    let dest = temp_dir.join("hermes_test_download.bin");

    let result = downloader.download_to_file(
        TEST_URL,
        &dest,
        "test-download-file",
        "Testing download to file",
    );

    assert!(
        result.is_ok(),
        "download_to_file should succeed: {:?}",
        result.err()
    );

    let mut file = std::fs::File::open(&dest).expect("Downloaded file should exist");
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)
        .expect("Should be able to read downloaded file");

    let text = String::from_utf8(contents).expect("Downloaded file should be valid UTF-8");
    assert!(
        text.contains("https://httpbin.org/get"),
        "Downloaded file should contain the requested URL"
    );

    // Cleanup
    let _ = std::fs::remove_file(&dest);
}
