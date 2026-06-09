use crate::acp::{Result, error::Error};
use crate::utilities::notification_messenger::{
    NotificationMessenger, ProgressMessage, ProgressStatus, ProgressTracker,
};
use std::io::{Read, Write};
use std::path::Path;

/// Reusable HTTP downloader with integrated progress reporting.
///
/// Encapsulates the full download lifecycle: ureq fetch, chunked reading,
/// percentage calculation, debounced progress emission, and error handling.
/// Methods are synchronous so callers decide whether to run on a blocking thread.
#[derive(Debug, Clone)]
pub struct Downloader {
    messenger: NotificationMessenger,
}

impl Downloader {
    pub fn new(messenger: NotificationMessenger) -> Self {
        Self { messenger }
    }

    pub fn download_to_file(&self, url: &str, dest: &Path, id: &str, title: &str) -> Result<()> {
        let mut file = std::fs::File::create(dest)
            .map_err(|e| Error::Network(format!("Failed to create file: {e}")))?;
        self.download(url, id, title, |bytes| {
            file.write_all(&bytes)
                .map_err(|e| Error::Network(format!("Failed to write download chunk: {e}")))
                .map(|_| ())
        })?;

        drop(file);

        Ok(())
    }

    pub fn download_to_string(&self, url: &str, id: &str, title: &str) -> Result<String> {
        let mut body = Vec::new();
        self.download(url, id, title, |bytes| {
            body.extend_from_slice(&bytes);
            Ok(())
        })?;
        String::from_utf8(body)
            .map_err(|e| Error::Network(format!("Failed to decode response: {e}")))
    }

    /// Download a URL to a buffer in memory, streaming with progress reporting.
    fn download<F>(&self, url: &str, id: &str, title: &str, mut handle: F) -> Result<()>
    where
        F: FnMut(Vec<u8>) -> Result<()>,
    {
        self.messenger.send_progress(ProgressMessage {
            id: id.to_string(),
            title: title.to_string(),
            status: ProgressStatus::Begin,
            percent: Some(0),
            text: Some(format!("Downloading from {}", url)),
        })?;

        let mut response = ureq::get(url)
            .call()
            .map_err(|e| Error::Network(format!("Failed to download {url}: {e}")))?;

        let total_size = response
            .headers_mut()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);

        let mut reader = response.body_mut().as_reader();
        let mut buffer = vec![0u8; 8 * 1024];
        let mut tracker = ProgressTracker::new(2, 250);
        let mut downloaded: u64 = 0;

        loop {
            let n = reader
                .read(&mut buffer)
                .map_err(|e| Error::Network(format!("Failed to read download chunk: {e}")))?;
            if n == 0 {
                break;
            }

            handle(buffer[..n].to_vec())?;
            downloaded += n as u64;

            if total_size > 0 {
                let percent = ((downloaded as f64 / total_size as f64) * 100.0) as u32;
                if tracker.should_emit(percent) {
                    self.messenger.send_progress(ProgressMessage {
                        id: id.to_string(),
                        title: title.to_string(),
                        status: ProgressStatus::Report,
                        percent: Some(percent),
                        text: Some(format!("{}% downloaded", percent)),
                    })?;
                }
            }
        }

        self.messenger.send_progress(ProgressMessage {
            id: id.to_string(),
            title: title.to_string(),
            status: ProgressStatus::End,
            percent: Some(100),
            text: Some("Download complete".to_string()),
        })
    }
}
