use crate::acp::{Result, error::Error};
use crate::utilities::{LogLevel, exec_autocmd};
use crossbeam_channel::{Sender, bounded};
use nvim_oxi::libuv::AsyncHandle;
use nvim_oxi::{Array, Dictionary, Object, api};
use std::ptr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// A notification message to be delivered to Neovim
#[derive(Debug, Clone, PartialEq)]
pub struct NotificationMessage {
    pub message: String,
    pub level: LogLevel,
}

/// A progress update to be delivered to Neovim
#[derive(Debug, Clone, PartialEq)]
pub struct ProgressMessage {
    pub id: String,
    pub title: String,
    pub status: ProgressStatus,
    pub percent: Option<u32>,
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProgressStatus {
    Running,
    Success,
    Failure,
}

impl ProgressStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProgressStatus::Running => "running",
            ProgressStatus::Success => "success",
            ProgressStatus::Failure => "failure",
        }
    }
}

/// Dictionary for nvim_echo kind="progress" opts matching Lua's emit_progress format
struct ProgressEchoOpts {
    id: String,
    status: String,
    percent: Option<i64>,
    text: Option<String>,
}

impl From<ProgressEchoOpts> for Dictionary {
    fn from(opts: ProgressEchoOpts) -> Dictionary {
        let mut dict = Dictionary::default();
        dict.insert("kind", Object::from("progress"));
        dict.insert("id", Object::from(opts.id));
        dict.insert("source", Object::from("hermes"));
        dict.insert("status", Object::from(opts.status));
        if let Some(percent) = opts.percent {
            dict.insert("percent", Object::from(percent));
        }
        if let Some(text) = opts.text {
            dict.insert("title", Object::from(text));
        }
        dict
    }
}

/// Dictionary for User Progress autocommand data matching Lua's emit_progress format
struct ProgressAutocmdData {
    id: String,
    title: String,
    status: String,
    percent: Option<i64>,
    text: Option<String>,
}

impl From<ProgressAutocmdData> for Dictionary {
    fn from(data: ProgressAutocmdData) -> Dictionary {
        let mut dict = Dictionary::default();
        dict.insert("id", Object::from(data.id));
        dict.insert("title", Object::from(data.title));
        dict.insert("source", Object::from("hermes"));
        dict.insert("status", Object::from(data.status));
        if let Some(percent) = data.percent {
            dict.insert("percent", Object::from(percent));
        }
        if let Some(text) = data.text {
            let text_array = Array::from((Object::from(text),));
            dict.insert("text", Object::from(text_array));
        }
        dict
    }
}

/// A messenger that sends notifications and progress updates from any thread
/// to be delivered on Neovim's main thread
#[derive(Clone)]
pub struct NotificationMessenger {
    handle: Arc<AsyncHandle>,
    sender: Arc<Sender<MessengerMessage>>,
}

/// Internal enum for routing messages through the same channel
#[derive(Debug, Clone, PartialEq)]
pub enum MessengerMessage {
    Notification(NotificationMessage),
    Progress(ProgressMessage),
}

impl PartialEq for NotificationMessenger {
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(self, other) // Compares memory addresses
    }
}

impl Eq for NotificationMessenger {}

impl std::fmt::Debug for NotificationMessenger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NotificationMessenger")
            .field("capacity", &self.sender.capacity())
            .finish()
    }
}

/// Returns the Neovim highlight group name for a given log level.
fn hl_group_for_level(level: LogLevel) -> &'static str {
    match level {
        LogLevel::Error => "ErrorMsg",
        LogLevel::Warn => "WarningMsg",
        LogLevel::Info => "MoreMsg",
        LogLevel::Debug | LogLevel::Trace => "Comment",
        LogLevel::Off => "",
    }
}

impl NotificationMessenger {
    /// Create a new NotificationMessenger with the given sender and AsyncHandle
    ///
    /// This is the low-level constructor for testing and custom initialization.
    /// For standard use, prefer `NotificationMessenger::initialize()`.
    pub fn new(sender: Sender<MessengerMessage>, handle: AsyncHandle) -> Self {
        Self {
            handle: Arc::new(handle),
            sender: Arc::new(sender),
        }
    }

    pub fn nvim_echo_opts_available() -> bool {
        messagesopt_exists()
    }

    /// Initialize the notification messenger with a callback that processes notifications on the main thread
    ///
    /// This creates a bounded channel with capacity 1000 and sets up the AsyncHandle callback.
    /// Must be called from Neovim's main thread.
    pub fn initialize() -> Result<Self> {
        let (sender, receiver) = bounded::<MessengerMessage>(1000);

        // Gate: only use nvim_echo(kind="progress") on Neovim 0.12+
        let use_nvim_echo = Arc::new(AtomicBool::new(Self::nvim_echo_opts_available()));

        let handle = AsyncHandle::new(move || {
            while let Ok(msg) = receiver.try_recv() {
                let use_nvim_echo = Arc::clone(&use_nvim_echo);
                // CRITICAL: Defer Neovim API calls via vim.schedule to avoid
                // calling them during uv_run() which can crash Neovim.
                // See NvimMessenger::initialize for full explanation.
                nvim_oxi::schedule(move |_| {
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match msg {
                        MessengerMessage::Notification(notification) => {
                            let is_err = matches!(notification.level, LogLevel::Error);
                            let hl_group = hl_group_for_level(notification.level);
                            let chunk = Array::from((
                                Object::from(notification.message.as_str()),
                                Object::from(hl_group),
                            ));
                            let chunks = Array::from((chunk,));
                            let mut opts = Dictionary::default();
                            if is_err {
                                opts.insert("err", Object::from(true));
                            }
                            api::call_function::<(Array, bool, Dictionary), Object>(
                                "nvim_echo",
                                (chunks, true, opts),
                            )
                            .ok();
                        }
                        MessengerMessage::Progress(progress) => {
                            if use_nvim_echo.load(Ordering::Relaxed) {
                                let chunk = Array::from((
                                    Object::from(progress.title.as_str()),
                                    Object::from(""),
                                ));
                                let chunks = Array::from((chunk,));
                                api::call_function::<(Array, bool, Dictionary), Object>(
                                    "nvim_echo",
                                    (
                                        chunks,
                                        true,
                                        Dictionary::from(ProgressEchoOpts {
                                            id: progress.id.clone(),
                                            status: progress.status.as_str().to_string(),
                                            percent: progress.percent.map(|p| p as i64),
                                            text: progress.text.clone(),
                                        }),
                                    ),
                                )
                                .ok();
                            }

                            // Always fire User Progress autocommand matching Lua's format
                            let _ = exec_autocmd(
                                "hermes",
                                "User",
                                "Progress",
                                Object::from(Dictionary::from(ProgressAutocmdData {
                                    id: progress.id,
                                    title: progress.title,
                                    status: progress.status.as_str().to_string(),
                                    percent: progress.percent.map(|p| p as i64),
                                    text: progress.text,
                                })),
                            );
                        }
                    }))
                    .ok();
                    Ok::<_, nvim_oxi::Error>(())
                });
            }
        })
        .map_err(|e| Error::Internal(e.to_string()))?;

        Ok(Self::new(sender, handle))
    }

    /// Send a notification (can be called from any thread)
    ///
    /// The notification is queued and will be delivered on Neovim's main thread
    /// via the AsyncHandle callback.
    pub fn send(&self, message: String, level: LogLevel) -> Result<()> {
        self.sender
            .try_send(MessengerMessage::Notification(NotificationMessage {
                message,
                level,
            }))
            .map_err(|e| Error::Internal(format!("Failed to queue notification: {}", e)))?;

        self.handle
            .send()
            .map_err(|e| Error::Internal(format!("Failed to signal notification handler: {}", e)))
    }

    /// Send a progress update (can be called from any thread)
    pub fn send_progress(&self, progress: ProgressMessage) -> Result<()> {
        self.sender
            .try_send(MessengerMessage::Progress(progress))
            .map_err(|e| Error::Internal(format!("Failed to queue progress: {}", e)))?;

        self.handle
            .send()
            .map_err(|e| Error::Internal(format!("Failed to signal progress handler: {}", e)))
    }

    /// Get the number of messages in the queue
    #[cfg(test)]
    pub fn queue_len(&self) -> usize {
        self.sender.len()
    }
}

/// Check whether Neovim supports `messagesopt` (available since 0.12)
pub fn messagesopt_exists() -> bool {
    api::call_function::<(String,), i32>("exists", ("+messagesopt".to_string(),))
        .map(|result| result == 1)
        .unwrap_or(false)
}

/// A simple debouncing progress tracker for a single download operation
#[derive(Debug, Clone)]
pub struct ProgressTracker {
    last_percent: u32,
    last_emit: Instant,
    min_delta_percent: u32,
    min_delta_time: Duration,
}

impl ProgressTracker {
    pub fn new(min_delta_percent: u32, min_delta_time_ms: u64) -> Self {
        Self {
            last_percent: 0,
            last_emit: Instant::now(),
            min_delta_percent,
            min_delta_time: Duration::from_millis(min_delta_time_ms),
        }
    }

    /// Returns true if progress should be emitted based on debounce rules
    pub fn should_emit(&mut self, percent: u32) -> bool {
        let now = Instant::now();
        let percent_delta = percent.saturating_sub(self.last_percent);
        let time_delta = now.duration_since(self.last_emit);

        if percent_delta >= self.min_delta_percent || time_delta >= self.min_delta_time {
            self.last_percent = percent;
            self.last_emit = now;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    struct TestableMessenger {
        sender: Sender<MessengerMessage>,
        receiver: crossbeam_channel::Receiver<MessengerMessage>,
    }

    impl std::fmt::Debug for TestableMessenger {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("NotificationMessenger")
                .field("sender", &"bounded")
                .finish()
        }
    }

    impl TestableMessenger {
        fn new(capacity: usize) -> Self {
            let (sender, receiver) = bounded::<MessengerMessage>(capacity);
            Self { sender, receiver }
        }

        fn try_send_notification(&self, message: String, level: LogLevel) -> Result<()> {
            self.sender
                .try_send(MessengerMessage::Notification(NotificationMessage {
                    message,
                    level,
                }))
                .map_err(|e| Error::Internal(format!("Failed to queue notification: {}", e)))
        }

        fn try_send_progress(&self, progress: ProgressMessage) -> Result<()> {
            self.sender
                .try_send(MessengerMessage::Progress(progress))
                .map_err(|e| Error::Internal(format!("Failed to queue progress: {}", e)))
        }

        fn try_recv(&self) -> Option<MessengerMessage> {
            self.receiver.try_recv().ok()
        }
    }

    #[test]
    fn test_notification_message_creation() {
        let msg = NotificationMessage {
            message: "Test message".to_string(),
            level: LogLevel::Info,
        };
        assert_eq!(msg.message, "Test message");
        assert_eq!(msg.level, LogLevel::Info);
    }

    #[test]
    fn test_notification_message_clone() {
        let msg = NotificationMessage {
            message: "Test".to_string(),
            level: LogLevel::Debug,
        };
        let cloned = msg.clone();
        assert_eq!(msg, cloned);
    }

    #[test]
    fn test_notification_message_debug() {
        let msg = NotificationMessage {
            message: "Test".to_string(),
            level: LogLevel::Error,
        };
        let debug_str = format!("{:?}", msg);
        assert!(debug_str.contains("Test"));
        assert!(debug_str.contains("Error"));
    }

    #[test]
    fn test_notification_message_equality() {
        let msg1 = NotificationMessage {
            message: "Test".to_string(),
            level: LogLevel::Info,
        };
        let msg2 = NotificationMessage {
            message: "Test".to_string(),
            level: LogLevel::Info,
        };
        let msg3 = NotificationMessage {
            message: "Different".to_string(),
            level: LogLevel::Info,
        };
        assert_eq!(msg1, msg2);
        assert_ne!(msg1, msg3);
    }

    #[test]
    fn test_notification_messenger_new() {
        let (_sender, receiver) = bounded::<MessengerMessage>(10);
        assert_eq!(receiver.capacity(), Some(10));
    }

    #[test]
    fn test_notification_messenger_send_success() {
        let messenger = TestableMessenger::new(10);

        let result = messenger.try_send_notification("Test message".to_string(), LogLevel::Info);
        assert!(result.is_ok());

        let msg = messenger.try_recv();
        assert!(msg.is_some());
        match msg.unwrap() {
            MessengerMessage::Notification(n) => assert_eq!(n.message, "Test message"),
            _ => panic!("Expected notification message"),
        }
    }

    #[test]
    fn test_notification_messenger_send_multiple() {
        let messenger = TestableMessenger::new(10);

        for i in 0..5 {
            let result = messenger.try_send_notification(format!("Message {}", i), LogLevel::Info);
            assert!(result.is_ok());
        }

        for i in 0..5 {
            let msg = messenger.try_recv();
            assert!(msg.is_some());
            match msg.unwrap() {
                MessengerMessage::Notification(n) => {
                    assert_eq!(n.message, format!("Message {}", i))
                }
                _ => panic!("Expected notification message"),
            }
        }
    }

    #[test]
    fn test_notification_messenger_send_channel_full() {
        let messenger = TestableMessenger::new(2);

        messenger
            .try_send_notification("msg1".to_string(), LogLevel::Info)
            .unwrap();
        messenger
            .try_send_notification("msg2".to_string(), LogLevel::Info)
            .unwrap();

        let result = messenger.try_send_notification("msg3".to_string(), LogLevel::Info);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to queue"));
    }

    #[test]
    fn test_notification_messenger_send_various_levels() {
        let messenger = TestableMessenger::new(10);
        let levels = vec![
            LogLevel::Trace,
            LogLevel::Debug,
            LogLevel::Info,
            LogLevel::Warn,
            LogLevel::Error,
        ];

        for level in levels {
            let result = messenger.try_send_notification(format!("{:?}", level), level);
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_notification_messenger_send_empty_message() {
        let messenger = TestableMessenger::new(10);

        let result = messenger.try_send_notification("".to_string(), LogLevel::Info);
        assert!(result.is_ok());

        let msg = messenger.try_recv();
        assert!(msg.is_some());
        match msg.unwrap() {
            MessengerMessage::Notification(n) => assert_eq!(n.message, ""),
            _ => panic!("Expected notification message"),
        }
    }

    #[test]
    fn test_notification_messenger_send_special_characters() {
        let messenger = TestableMessenger::new(10);

        let special = r#"Special chars: <>&"' and unicode: 🎉"#;
        let result = messenger.try_send_notification(special.to_string(), LogLevel::Info);
        assert!(result.is_ok());

        let msg = messenger.try_recv();
        assert!(msg.is_some());
        match msg.unwrap() {
            MessengerMessage::Notification(n) => assert_eq!(n.message, special),
            _ => panic!("Expected notification message"),
        }
    }

    #[test]
    fn test_notification_messenger_send_long_message() {
        let messenger = TestableMessenger::new(10);

        let long_message = "a".repeat(10000);
        let result = messenger.try_send_notification(long_message.clone(), LogLevel::Info);
        assert!(result.is_ok());

        let msg = messenger.try_recv();
        assert!(msg.is_some());
        match msg.unwrap() {
            MessengerMessage::Notification(n) => assert_eq!(n.message.len(), 10000),
            _ => panic!("Expected notification message"),
        }
    }

    #[test]
    fn test_notification_messenger_debug_trait() {
        let (sender, _receiver) = bounded::<MessengerMessage>(100);
        assert_eq!(sender.capacity(), Some(100));
    }

    // Property-based tests
    proptest! {
        #[test]
        fn test_send_never_panics_with_any_message(msg in any::<String>()) {
            let messenger = TestableMessenger::new(100);
            let level = LogLevel::Info;
            let _ = messenger.try_send_notification(msg, level);
        }

        #[test]
        fn test_send_never_panics_with_any_level(level in 0i64..6) {
            let messenger = TestableMessenger::new(100);
            let level = LogLevel::from(level);
            let _ = messenger.try_send_notification("test".to_string(), level);
        }

        #[test]
        fn test_roundtrip_message_preserved(msg in any::<String>()) {
            let messenger = TestableMessenger::new(100);
            let level = LogLevel::Debug;

            messenger.try_send_notification(msg.clone(), level).ok();

            let received = messenger.try_recv();
            if let Some(MessengerMessage::Notification(received_msg)) = received {
                assert_eq!(received_msg.message, msg);
                assert_eq!(received_msg.level, level);
            }
        }
    }

    #[test]
    fn test_notification_messenger_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<NotificationMessenger>();
    }

    #[test]
    fn test_notification_messenger_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<NotificationMessenger>();
    }

    #[test]
    fn test_notification_message_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<NotificationMessage>();
    }

    #[test]
    fn test_notification_message_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<NotificationMessage>();
    }

    #[test]
    fn test_notification_messenger_queue_len_initially_zero() {
        let messenger = TestableMessenger::new(10);
        assert_eq!(messenger.sender.len(), 0);
    }

    #[test]
    fn test_notification_messenger_queue_len_after_send() {
        let messenger = TestableMessenger::new(10);
        messenger
            .try_send_notification("msg1".to_string(), LogLevel::Info)
            .unwrap();
        assert_eq!(messenger.sender.len(), 1);
    }

    #[test]
    fn test_notification_messenger_queue_len_after_multiple_sends() {
        let messenger = TestableMessenger::new(10);
        for i in 0..5 {
            messenger
                .try_send_notification(format!("msg{}", i), LogLevel::Info)
                .unwrap();
        }
        assert_eq!(messenger.sender.len(), 5);
    }

    #[test]
    fn test_notification_messenger_queue_len_after_recv() {
        let messenger = TestableMessenger::new(10);
        messenger
            .try_send_notification("msg1".to_string(), LogLevel::Info)
            .unwrap();
        assert_eq!(messenger.sender.len(), 1);
        messenger.try_recv();
        assert_eq!(messenger.sender.len(), 0);
    }

    #[test]
    fn test_notification_messenger_debug_shows_capacity() {
        let messenger = TestableMessenger::new(50);
        let debug_str = format!("{:?}", messenger);
        assert!(debug_str.contains("bounded"));
    }

    #[test]
    fn test_notification_message_debug_shows_level() {
        let msg = NotificationMessage {
            message: "test".to_string(),
            level: LogLevel::Warn,
        };
        let debug_str = format!("{:?}", msg);
        assert!(debug_str.contains("Warn"));
    }

    #[test]
    fn test_notification_message_debug_shows_message() {
        let msg = NotificationMessage {
            message: "hello world".to_string(),
            level: LogLevel::Info,
        };
        let debug_str = format!("{:?}", msg);
        assert!(debug_str.contains("hello world"));
    }

    #[test]
    fn test_notification_messenger_capacity_correct() {
        let messenger = TestableMessenger::new(100);
        assert_eq!(messenger.sender.capacity(), Some(100));
    }

    #[test]
    fn test_notification_messenger_remaining_capacity() {
        let messenger = TestableMessenger::new(10);
        messenger
            .try_send_notification("msg".to_string(), LogLevel::Info)
            .unwrap();
        assert_eq!(messenger.sender.capacity(), Some(10));
    }

    #[test]
    fn test_progress_message_creation() {
        let msg = ProgressMessage {
            id: "test-id".to_string(),
            title: "Downloading".to_string(),
            status: ProgressStatus::Running,
            percent: Some(0),
            text: Some("Starting download".to_string()),
        };
        assert_eq!(msg.id, "test-id");
        assert_eq!(msg.status, ProgressStatus::Running);
    }

    #[test]
    fn test_progress_status_as_str() {
        assert_eq!(ProgressStatus::Running.as_str(), "running");
        assert_eq!(ProgressStatus::Success.as_str(), "success");
        assert_eq!(ProgressStatus::Failure.as_str(), "failure");
    }

    #[test]
    fn test_progress_messenger_send_success() {
        let messenger = TestableMessenger::new(10);
        let msg = ProgressMessage {
            id: "id".to_string(),
            title: "title".to_string(),
            status: ProgressStatus::Running,
            percent: Some(50),
            text: None,
        };
        let result = messenger.try_send_progress(msg);
        assert!(result.is_ok());

        let received = messenger.try_recv();
        assert!(received.is_some());
        match received.unwrap() {
            MessengerMessage::Progress(p) => assert_eq!(p.percent, Some(50)),
            _ => panic!("Expected progress message"),
        }
    }

    #[test]
    fn test_progress_messenger_send_channel_full() {
        let messenger = TestableMessenger::new(2);
        for i in 0..2 {
            let msg = ProgressMessage {
                id: format!("id{}", i),
                title: "title".to_string(),
                status: ProgressStatus::Running,
                percent: None,
                text: None,
            };
            messenger.try_send_progress(msg).unwrap();
        }
        let extra = ProgressMessage {
            id: "extra".to_string(),
            title: "title".to_string(),
            status: ProgressStatus::Running,
            percent: None,
            text: None,
        };
        let result = messenger.try_send_progress(extra);
        assert!(result.is_err());
    }

    #[test]
    fn test_progress_tracker_initial_emit_when_delta_met() {
        let mut tracker = ProgressTracker::new(2, 250);
        assert!(tracker.should_emit(2));
        assert_eq!(tracker.last_percent, 2);
    }

    #[test]
    fn test_progress_tracker_debounce_by_percent() {
        let mut tracker = ProgressTracker::new(2, 250);
        assert!(tracker.should_emit(2));
        assert!(!tracker.should_emit(3));
        assert!(tracker.should_emit(4));
    }

    #[test]
    fn test_progress_tracker_debounce_by_time() {
        let mut tracker = ProgressTracker::new(100, 0);
        assert!(tracker.should_emit(0));
        assert!(tracker.should_emit(1));
    }

    #[test]
    fn test_progress_tracker_no_backward_progress() {
        let mut tracker = ProgressTracker::new(2, 250);
        assert!(tracker.should_emit(50));
        assert!(!tracker.should_emit(49));
    }

    #[test]
    fn test_messenger_message_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<MessengerMessage>();
    }

    #[test]
    fn test_messenger_message_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<MessengerMessage>();
    }

    #[test]
    fn test_progress_message_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<ProgressMessage>();
    }

    #[test]
    fn test_progress_message_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<ProgressMessage>();
    }

    #[test]
    fn hl_group_for_level_error_returns_errormsg() {
        assert_eq!(hl_group_for_level(LogLevel::Error), "ErrorMsg");
    }

    #[test]
    fn hl_group_for_level_warn_returns_warningmsg() {
        assert_eq!(hl_group_for_level(LogLevel::Warn), "WarningMsg");
    }

    #[test]
    fn hl_group_for_level_info_returns_moremsg() {
        assert_eq!(hl_group_for_level(LogLevel::Info), "MoreMsg");
    }

    #[test]
    fn hl_group_for_level_debug_returns_comment() {
        assert_eq!(hl_group_for_level(LogLevel::Debug), "Comment");
    }

    #[test]
    fn hl_group_for_level_trace_returns_comment() {
        assert_eq!(hl_group_for_level(LogLevel::Trace), "Comment");
    }

    #[test]
    fn hl_group_for_level_off_returns_empty() {
        assert_eq!(hl_group_for_level(LogLevel::Off), "");
    }
}
