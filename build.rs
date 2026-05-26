fn main() -> Result<(), nvim_oxi::tests::BuildError> {
    let src = std::path::Path::new(".githooks/pre-commit");
    let dst = std::path::Path::new(".git/hooks/pre-commit");
    if src.exists() {
        std::fs::copy(src, dst).ok();
        #[cfg(unix)]
        std::fs::set_permissions(dst, std::os::unix::fs::PermissionsExt::from_mode(0o755)).ok();
    }

    nvim_oxi::tests::build()
}
