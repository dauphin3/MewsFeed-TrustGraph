use crate::{index_tree::IndexSegment, reading::link_prefix_for_index, TimeIndexResult};
use chrono::{DateTime, NaiveDateTime, Utc};
use hdk::prelude::*;
use std::cmp::Ordering;

/**
 * Higher-order function to generate a comparator callback for use in
 * sorting sets of `AnyLinkableHash`es that have been indexed into the
 * `index_name` time-ordered index.
 *
 * It is presumed that the hashes are validly linked in the index. If
 * not, they will be placed at the end of the list in undefined order.
 *
 * Usage:
 *  let mut my_hashes: Vec<AnyLinkableHash> = some_list_of_hashes;
 *  my_hashes.sort_by(sort_hashes_by_time_index("my_time_index_name"));
 */
pub fn sort_hashes_by_time_index<'a, I, IT: Copy + LinkTypeFilterExt + 'a, E>(
    index_name: &'a I,
    index_link_type: IT,
) -> Box<dyn for<'r> Fn(&AnyLinkableHash, &AnyLinkableHash) -> Ordering + 'a>
where
    I: AsRef<str>,
    ScopedLinkType: TryFrom<IT, Error = E>,
    WasmError: From<E>,
{
    let prefix = link_prefix_for_index(index_name);

    Box::new(move |a, b| {
        let a_timestamp = get_time_for_hash(prefix.clone(), index_link_type.clone(), a);
        let b_timestamp = get_time_for_hash(prefix.clone(), index_link_type.clone(), b);
        b_timestamp.cmp(&a_timestamp)
    })
}

/// Determine the timestamp for an `hash` indexed into a time index.
///
/// `index_link_prefix` is the leading bytes shared by all `LinkTag`s relevant to the index, used
/// with Holochain's native link filtering. Pre-generated via `link_prefix_for_index` for minor performance gains.
///
/// Hashes may on occasion (usually due to faulty logic in controller zomes, or to
/// network partitions) be indexed multiple times into the same index.
///
/// :WARNING:
///     This method silently fails for any `hash` that is not indexed into the
///     associated time index of `index_link_prefix`, and such hashes are sorted to
///     the end of any list via way of a return value of `null_time()`.
///
///     Always ensure that your client code has appropriately pre-stored the given
///     hashes into a time index before querying it, or (if you're not comfortable
///     making this bargain on your code's correctness) test that all hashes are
///     stored into the index before querying.
///
/// Note that hashes written multiple times into the same index will be sorted based
/// upon the *first observed* indexing time in the local Holochain Cell DHT. This may
/// result in discrepancy in the ordering of data for different peers in a loosely-
/// partitioned network.
///
fn get_time_for_hash(
    index_link_prefix: LinkTag,
    index_link_type: impl LinkTypeFilterExt,
    hash: &AnyLinkableHash,
) -> DateTime<Utc> {
    let links = get_links(hash.to_owned(), index_link_type, Some(index_link_prefix));
    match links {
        Err(_e) => null_time(),
        Ok(links) => {
            // no index present into the given time index
            if links.len() < 1 {
                return null_time();
            }
            // take first link for the index as source of truth- the first agent who
            // knew about a thing probably knew about it first.
            let try_segment: TimeIndexResult<IndexSegment> =
                links.first().unwrap().tag.to_owned().try_into();
            match try_segment {
                Ok(segment) => match segment.try_into() {
                    Ok(date) => date,
                    Err(_e) => null_time(),
                },
                Err(_e) => null_time(),
            }
        }
    }
}

fn null_time() -> DateTime<Utc> {
    DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp_opt(0, 0).unwrap(), Utc)
}
