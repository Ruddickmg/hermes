pub async fn command_available(cmd: &str) -> bool {
    let cmd = cmd.to_owned();
    blocking::unblock(move || {
        match std::process::Command::new(&cmd)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            Ok(mut child) => {
                let _ = child.kill();
                let _ = child.wait();
                true
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
            Err(_) => false,
        }
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_available_known_command_returns_true() {
        let result = futures_lite::future::block_on(command_available("sh"));
        assert!(result);
    }

    #[test]
    fn command_available_nonexistent_command_returns_false() {
        let result =
            futures_lite::future::block_on(command_available("nonexistent-command-xyz-999"));
        assert!(!result);
    }

    #[test]
    fn command_available_empty_string_returns_false() {
        let result = futures_lite::future::block_on(command_available(""));
        assert!(!result);
    }
}
