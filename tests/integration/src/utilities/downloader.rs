//! Integration tests for the Downloader

use hermes::utilities::{Downloader, NotificationMessenger};
use std::io::Read;

/// A small, stable endpoint for verifying download behaviour.
const TEST_URL: &str =
    "https://raw.githubusercontent.com/Ruddickmg/hermes.nvim/development/README.md";

#[nvim_oxi::test]
fn download_to_string_succeeds_for_valid_url() {
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
}

#[nvim_oxi::test]
fn download_to_string_body_contains_expected_content() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");
    let downloader = Downloader::new(messenger);

    let body = downloader
        .download_to_string(
            TEST_URL,
            "test-download-string",
            "Testing download to string",
        )
        .expect("download_to_string should succeed");

    assert!(
        body.contains("hermes"),
        "Response should contain expected content"
    );
}

#[nvim_oxi::test]
fn download_to_file_succeeds_for_valid_url() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");
    let downloader = Downloader::new(messenger);

    let temp_dir = tempfile::tempdir().expect("Should create temp dir");
    let dest = temp_dir.path().join("hermes_test_download.bin");

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
}

#[nvim_oxi::test]
fn download_to_file_contents_match_expected() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");
    let downloader = Downloader::new(messenger);

    let temp_dir = tempfile::tempdir().expect("Should create temp dir");
    let dest = temp_dir.path().join("hermes_test_download.bin");

    downloader
        .download_to_file(
            TEST_URL,
            &dest,
            "test-download-file",
            "Testing download to file",
        )
        .expect("download_to_file should succeed");

    let mut file = std::fs::File::open(&dest).expect("Downloaded file should exist");
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)
        .expect("Should be able to read downloaded file");

    let text = String::from_utf8(contents).expect("Downloaded file should be valid UTF-8");
    assert!(
        text.contains("hermes"),
        "Downloaded file should contain expected content"
    );
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

    let temp_dir = tempfile::tempdir().expect("Should create temp dir");
    let dest = temp_dir.path().join("hermes_test_404.bin");

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
}
