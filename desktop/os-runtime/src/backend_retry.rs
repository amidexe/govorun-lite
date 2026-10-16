//! Preserve the deployed engine's single CPU retry on a backend error only.
//! Other failures, including cancellation, cannot trigger a second attempt.
pub fn once<T, E>(
    already_cpu: bool,
    mut attempt: impl FnMut(bool) -> Result<T, E>,
    backend_error: impl Fn(&E) -> bool,
) -> Result<T, E> {
    match attempt(false) {
        Err(error) if !already_cpu && backend_error(&error) => attempt(true),
        result => result,
    }
}
