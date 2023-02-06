use metrics::{counter, Counter, Gauge};
use reth_interfaces::p2p::error::{DownloadError, RequestError};
use reth_metrics_derive::Metrics;

/// Common downloader metrics.
///
/// These metrics will be dynamically initialized with the provided scope
/// by corresponding downloaders.
/// ```
/// use reth_downloaders::metrics::DownloaderMetrics;
/// use reth_interfaces::p2p::error::DownloadError;
///
/// // Initialize metrics.
/// let metrics = DownloaderMetrics::new("downloaders.headers");
/// // Increment `downloaders.headers.timeout_errors` counter by 1.
/// metrics.increment_errors(&DownloadError::Timeout);
/// ```
#[derive(Clone, Metrics)]
#[metrics(dynamic = true)]
pub struct DownloaderMetrics {
    /// The number of items that were successfully sent to the poller (stage)
    pub total_flushed: Counter,
    /// Number of items that were successfully downloaded
    pub total_downloaded: Counter,
    /// The number of in-flight requests
    pub in_flight_requests: Gauge,
    /// The number of buffered responses
    pub buffered_responses: Gauge,
}

impl DownloaderMetrics {
    /// Increment errors counter.
    pub fn increment_errors(&self, error: &DownloadError) {
        let label = match error {
            DownloadError::Timeout => "timeout",
            DownloadError::HeaderValidation { .. } | DownloadError::BodyValidation { .. } => {
                "validation"
            }
            DownloadError::TooManyBodies { .. } | DownloadError::HeadersResponseTooShort { .. } => {
                "length"
            }
            DownloadError::DatabaseError(_) => "db",
            DownloadError::EmptyResponse => "empty_response",
            DownloadError::RequestError(err) => match err {
                RequestError::Timeout => "timeout",
                RequestError::BadResponse => "bad_protocol_message",
                RequestError::ConnectionDropped => "connection_dropped",
                RequestError::ChannelClosed => "channel_closed",
                RequestError::UnsupportedCapability => "unsupported_cap",
            },
            _error => "unexpected",
        };

        counter!("errors", 1, "type" => label);
    }
}
