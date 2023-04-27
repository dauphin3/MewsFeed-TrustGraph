use crate::{index_tree::*, TimeIndexResult, TimeIndexingError};
use chrono::{DateTime, Utc};
use hdk::prelude::*;

/// Index a hash `hash` into the time-ordered index
/// identified by `index_hash` at the given time point.
///
/// The hash must already exist and have been written to the local DHT.
///
pub fn index_hash<I, IT: Copy + LinkTypeFilterExt, E>(
    index_name: &I,
    index_link_type: IT,
    hash: AnyLinkableHash,
    time: DateTime<Utc>,
) -> TimeIndexResult<()>
where
    I: AsRef<str>,
    ScopedLinkType: TryFrom<IT, Error = E>,
    WasmError: From<E>,
{
    // write the time index tree
    let leafmost_segment = ensure_time_index(index_name, index_link_type, time)?;
    let leafmost_hash = leafmost_segment.hash()?;

    // create a virtual segment for determining the final link tag data
    let target_hash_segment = IndexSegment::leafmost_link(&time);
    let encoded_link_tag = target_hash_segment.tag_for_index(&index_name);

    // ensure link from the leaf index to the target hash
    link_if_not_linked(
        AnyLinkableHash::from(leafmost_hash.to_owned()),
        hash.to_owned(),
        encoded_link_tag.to_owned(),
        index_link_type,
    )?;

    // ensure a reciprocal link from the target hash back to the leaf index node
    link_if_not_linked(
        hash,
        AnyLinkableHash::from(leafmost_hash),
        encoded_link_tag,
        index_link_type,
    )?;

    Ok(())
}

/// Returns the leaf-most `IndexSegment` in the time tree, so that target entries can be
/// linked from it.
///
fn ensure_time_index<I, IT: Copy + LinkTypeFilterExt, E>(
    index_name: &I,
    index_link_type: IT,
    time: DateTime<Utc>,
) -> TimeIndexResult<IndexSegment>
where
    I: AsRef<str>,
    ScopedLinkType: TryFrom<IT, Error = E>,
    WasmError: From<E>,
{
    // create a root anchor for the path based on the index name
    let root = Path::from(index_name.as_ref()).typed(index_link_type)?;
    root.ensure()?;
    let root_hash = root.path_entry_hash()?;

    let segments = get_index_segments(&time);

    for (idx, segment) in segments.iter().enumerate() {
        if idx == 0 {
            // link the first segment to the root
            if !segment_links_exist(
                index_name,
                index_link_type,
                &AnyLinkableHash::from(root_hash.clone()),
                segment,
            )? {
                create_link(
                    root_hash.to_owned(),
                    segment.hash()?,
                    index_link_type,
                    segment.tag_for_index(&index_name),
                )?;
            }
        } else {
            // link subsequent segments to the previous one
            let prev_segment_hash = segments.get(idx - 1).unwrap().hash()?;

            if !segment_links_exist(
                index_name,
                index_link_type,
                &AnyLinkableHash::from(prev_segment_hash.clone()),
                segment,
            )? {
                create_link(
                    prev_segment_hash,
                    segment.hash()?,
                    index_link_type,
                    segment.tag_for_index(&index_name),
                )?;
            }
        }
    }

    Ok(segments.last().unwrap().cloned())
}

fn segment_links_exist<I>(
    index_name: &I,
    index_link_type: impl LinkTypeFilterExt,
    base_hash: &AnyLinkableHash,
    target_segment: &IndexSegment,
) -> TimeIndexResult<bool>
where
    I: AsRef<str>,
{
    Ok(get_links(
        base_hash.to_owned(),
        index_link_type,
        Some(target_segment.tag_for_index(&index_name)),
    )?
    .len()
        > 0)
}

fn link_if_not_linked<IT: Copy + LinkTypeFilterExt, E>(
    origin_hash: AnyLinkableHash,
    dest_hash: AnyLinkableHash,
    link_tag: LinkTag,
    index_link_type: IT,
) -> TimeIndexResult<()>
where
    ScopedLinkType: TryFrom<IT, Error = E>,
    WasmError: From<E>,
{
    if false
        == get_links(
            origin_hash.to_owned(),
            index_link_type,
            Some(link_tag.to_owned()),
        )?
        .iter()
        .any(|l| AnyLinkableHash::from(l.target.to_owned()) == dest_hash)
    {
        create_link(
            origin_hash.to_owned(),
            dest_hash.to_owned(),
            index_link_type,
            link_tag,
        )
        .map_err(|e| TimeIndexingError::NotIndexed(e.to_string(), origin_hash.to_owned()))?;
    }

    Ok(())
}
