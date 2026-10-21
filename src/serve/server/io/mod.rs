// Changed from oxhttp 0.3.3 for Statac: exports only what the server uses.
mod decoder;
mod encoder;

pub(super) use decoder::{decode_request_headers, skip_request_body};
pub(super) use encoder::encode_response;

/// Capacity for buffers.
///
/// Should be significantly greater than BufWriter capacity to avoid flush in the `copy` method.
pub(super) const BUFFER_CAPACITY: usize = 16 * 1024;
