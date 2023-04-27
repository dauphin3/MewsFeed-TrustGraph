use hdk::prelude::*;
/**
 * Utility library for managing time-ordered indexes on data stored in a
 * Holochain DHT.
 *
 * Goals are:
 *  - arbitrary time-ordered indexing decoupled from entry data
 *  - support multiple independent orderings for the same entry (eg. creation time & update time)
 *  - paginated result retrieval from arbitrary starting points
 *  - efficient determination & retrieval of most recent index
 *  - reasonably-efficient ordering of disparate results within an index
 *  - mitigation of DHT hotspotting to spread query load among peers
 *
 * With inspiration & some copypasta from
 * https://github.com/holochain-open-dev/holochain-time-index
 * and https://github.com/lightningrodlabs/hdk_crud
 *
 * @package hdk_time_indexing
 * @author  pospi <pospi@spadgos.com>
 * @since   2022-06-16
 */
use lazy_static::lazy_static;
use std::time::Duration;
use thiserror::Error;

#[cfg(not(feature = "internal-testing"))]
mod index_tree;
#[cfg(not(feature = "internal-testing"))]
mod reading;
#[cfg(not(feature = "internal-testing"))]
mod sorting;
#[cfg(not(feature = "internal-testing"))]
mod writing;

#[cfg(feature = "internal-testing")]
pub mod index_tree;
#[cfg(feature = "internal-testing")]
pub mod reading;
#[cfg(feature = "internal-testing")]
pub mod sorting;
#[cfg(feature = "internal-testing")]
pub mod writing;

pub use index_tree::IndexSegment as TimeIndex;
pub use reading::{get_latest_hashes, get_older_hashes, read_all_hashes};
pub use sorting::sort_hashes_by_time_index;
pub use writing::index_hash;
pub mod validate;
pub use validate::{validate_create_link_time_index, validate_delete_link_time_index};

/// Configuration object that should be set in your host DNA's properties
#[derive(Serialize, Deserialize, Debug, SerializedBytes)]
pub struct IndexConfiguration {
    pub time_index_chunk_interval_ms: usize,
}

#[derive(Error, Debug, Clone)]
pub enum TimeIndexingError {
    #[error(transparent)]
    Wasm(#[from] WasmError),

    #[error("Malformed time index link with bytes: {0:?}")]
    Malformed(Vec<u8>),
    #[error("Hash not indexed in {0} for reading from offset {1}")]
    NotIndexed(String, AnyLinkableHash),
}

pub type TimeIndexResult<T> = Result<T, TimeIndexingError>;

// enum defining fidelity of indexes to create
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum IndexType {
    Year,
    Month,
    Day,
    Hour,
    Minute,
    Second,
    Nanosecond, // used internally with `leafmost_link` segments
}

// Parse configuration & setup library constants
lazy_static! {
    pub static ref CHUNK_INTERVAL: Duration = {
        let host_dna_config = dna_info().expect("Could not get zome configuration").properties;
        let properties = IndexConfiguration::try_from(host_dna_config)
            .expect("Unable to parse index config from DNA properties. Please specify index chunk size in milliseconds via 'time_index_chunk_interval_ms' DNA property.");
        Duration::from_millis(properties.time_index_chunk_interval_ms as u64)
    };
    // determine what depth of time index should be hung from
    pub static ref INDEX_DEPTH: Vec<IndexType> =
        if *CHUNK_INTERVAL < Duration::from_secs(1) {
            vec![
                IndexType::Second,
                IndexType::Minute,
                IndexType::Hour,
                IndexType::Day,
                IndexType::Month,
                IndexType::Year,
            ]
        } else if *CHUNK_INTERVAL < Duration::from_secs(60) {
            vec![IndexType::Minute, IndexType::Hour, IndexType::Day, IndexType::Month, IndexType::Year]
        } else if *CHUNK_INTERVAL < Duration::from_secs(3600) {
            vec![IndexType::Hour, IndexType::Day, IndexType::Month, IndexType::Year]
        } else {
            vec![IndexType::Day, IndexType::Month, IndexType::Year]
        };
    // determine whether there is a trailing leaf node for chunks that don't round into standard time periods
    pub static ref HAS_CHUNK_LEAVES: bool = *CHUNK_INTERVAL < Duration::from_secs(1)
        || (*CHUNK_INTERVAL > Duration::from_secs(1) && *CHUNK_INTERVAL < Duration::from_secs(60))
        || (*CHUNK_INTERVAL > Duration::from_secs(60) && *CHUNK_INTERVAL < Duration::from_secs(3600))
        || (*CHUNK_INTERVAL > Duration::from_secs(3600) && *CHUNK_INTERVAL < Duration::from_secs(86400));
}
