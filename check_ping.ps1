# Watchdog launcher - delegates to the Rust binary.
# All configuration is handled via flags or environment variables:
#   WIMESH_CHECK_URL, WIMESH_RETRY_BASE_SECONDS, WIMESH_RETRY_MAX_SECONDS
& "$env:LOCALAPPDATA\wimesh\keep_wimesh_session.exe" watch @args
