//! Integration tests for the Downloader

use hermes::utilities::{Downloader, NotificationMessenger};
use std::io::Read;

/// A small, stable endpoint for verifying download behaviour.
const TEST_URL: &str =
    "https://raw.githubusercontent.com/Ruddickmg/hermes.nvim/development/README.md";

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
        body.contains("hermes"),
        "Response should contain expected content"
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
        text.contains("hermes"),
        "Downloaded file should contain expected content"
    );

    // Cleanup
    let _ = std::fs::remove_file(&dest);
}

#[nvim_oxi::test]
fn download_to_string_with_bad_url_returns_error() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");
    let downloader = Downloader::new(messenger);

    let result = downloader.download_to_string(
        "https://httpbin.org/status/404",
        "test-bad-url",
        "Testing bad URL",
    );

    assert!(
        result.is_err(),
        "download_to_string should fail for a 404 URL"
    );
}

#[nvim_oxi::test]
fn download_to_file_with_bad_url_returns_error() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");
    let downloader = Downloader::new(messenger);

    let temp_dir = std::env::temp_dir();
    let dest = temp_dir.join("hermes_test_404.bin");

    let result = downloader.download_to_file(
        "https://httpbin.org/status/404",
        &dest,
        "test-bad-url-file",
        "Testing bad URL to file",
    );

    assert!(
        result.is_err(),
        "download_to_file should fail for a 404 URL"
    );

    let _ = std::fs::remove_file(&dest);
}
