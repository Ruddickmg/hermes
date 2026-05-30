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
