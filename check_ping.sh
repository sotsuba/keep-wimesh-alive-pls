#!/bin/bash
# Watchdog launcher - delegates to the Rust binary.
# All configuration is handled via flags or environment variables:
#   WIMESH_CHECK_URL, WIMESH_RETRY_BASE_SECONDS, WIMESH_RETRY_MAX_SECONDS
exec /usr/local/bin/keep_wimesh_session watch "$@"
