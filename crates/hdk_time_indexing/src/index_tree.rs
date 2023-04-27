use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use hdk::prelude::*;

use crate::{
    IndexType, TimeIndexResult, TimeIndexingError, CHUNK_INTERVAL, HAS_CHUNK_LEAVES, INDEX_DEPTH,
};

#[hdk_entry_helper]
#[derive(Clone)]
// string formatted value, parse string for reading timestamp, bool if a chunk index
pub struct IndexSegment(String, IndexType, bool);

impl IndexSegment {
    /// Generate an index segment by truncating a timestamp (in ms)
    /// from the input `DateTime<Utc>` to the given `granularity`
    ///
    pub fn new(from: &DateTime<Utc>, granularity: &IndexType) -> Self {
        Self(
            format!(
                "{}",
                from.format(&granularity_to_format_string(granularity, false))
            ),
            granularity.to_owned(),
            false,
        )
    }

    /// Generate an index segment corresponding to the closest leaf chunk for the given timestamp
    ///
    pub fn new_chunk(based_off: &Self, from: &DateTime<Utc>) -> Self {
        let from_millis = from.timestamp_millis() as u64;
        let chunk_millis = CHUNK_INTERVAL.as_millis() as u64;
        let based_off_millis = based_off.timestamp().timestamp_millis() as u64;
        let diff = from_millis - based_off_millis;
        Self(
            format!("{}|{}", based_off.0, (diff / chunk_millis) * chunk_millis),
            based_off.1.to_owned(),
            true,
        )
    }

    /// Generate a virtual index segment for an exact time, to use with final referencing link tag
    ///
    pub fn leafmost_link(from: &DateTime<Utc>) -> Self {
        Self(
            format!(
                "{}",
                from.format(&granularity_to_format_string(&IndexType::Nanosecond, false))
            ),
            IndexType::Nanosecond,
            false,
        )
    }

    /// :SHONK: clone the `IndexSegment`. For some reason to_owned() is returning a ref?
    pub fn cloned(&self) -> Self {
        Self(self.0.clone(), self.1.clone(), self.2)
    }

    /// return the timestamp specifier of this `IndexSegment`
    pub fn timestamp(&self) -> DateTime<Utc> {
        timestamp_for_segment_str(&self.0, &self.1, self.2).unwrap()
    }

    /// Generate a `LinkTag` with encoded time of this index, suitable for linking from
    /// other entries in the index tree rooted at `index_name`.
    ///
    pub fn tag_for_index<I>(&self, index_name: &I) -> LinkTag
    where
        I: AsRef<str>,
    {
        LinkTag::new(
            [
                index_name.as_ref().as_bytes(), // prefix with index ID
                &[0x0 as u8],                   // null byte separator
                self.0.as_ref(), // truncated timestamp as a string that is unique for each index
            ]
            .concat(),
        )
    }

    /// What is the hash for the current [ `IndexSegment` ]?
    pub fn hash(&self) -> TimeIndexResult<EntryHash> {
        Ok(hash_entry(self.to_owned())?)
    }
}

/// :TODO: update this method to handle out of range errors more gracefully
/// (will currently panic due to unwrapping a `None` value)
///
impl Into<DateTime<Utc>> for IndexSegment {
    fn into(self) -> DateTime<Utc> {
        self.timestamp()
    }
}

impl TryFrom<LinkTag> for IndexSegment {
    type Error = TimeIndexingError;

    fn try_from(l: LinkTag) -> Result<Self, Self::Error> {
        Ok(Self::leafmost_link(&decode_link_tag_timestamp(l)?))
    }
}

/// strftime format strings for formatting (for_read = false) and
/// parsing (for_read = true) data at each level of granularity
///
fn granularity_to_format_string(granularity: &IndexType, for_read: bool) -> String {
    match granularity {
        IndexType::Year => {
            if for_read {
                "%Y-%m-%dT%H:%M".into()
            } else {
                "%Y".into()
            }
        }
        IndexType::Month => {
            if for_read {
                "%Y-%m-%dT%H:%M".into()
            } else {
                "%Y-%m".into()
            }
        }
        IndexType::Day => {
            if for_read {
                "%Y-%m-%dT%H:%M".into()
            } else {
                "%Y-%m-%d".into()
            }
        }
        IndexType::Hour => {
            if for_read {
                "%Y-%m-%dT%H:%M".into()
            } else {
                "%Y-%m-%dT%H".into()
            }
        }
        IndexType::Minute => "%Y-%m-%dT%H:%M".into(),
        IndexType::Second => "%Y-%m-%dT%H:%M:%S".into(),
        IndexType::Nanosecond => "%Y-%m-%dT%H:%M:%S.%f".into(),
    }
}

/// Generate a list of `IndexSegment` representing nodes in a radix trie for the given `time`.
/// The segments are returned in order of granularity, with least granular first.
///
pub(crate) fn get_index_segments(time: &DateTime<Utc>) -> Vec<IndexSegment> {
    let mut segments = vec![];

    // build main segments
    if INDEX_DEPTH.contains(&IndexType::Year) {
        segments.push(IndexSegment::new(&time, &IndexType::Year));
    }
    if INDEX_DEPTH.contains(&IndexType::Month) {
        segments.push(IndexSegment::new(&time, &IndexType::Month));
    }
    if INDEX_DEPTH.contains(&IndexType::Day) {
        segments.push(IndexSegment::new(&time, &IndexType::Day));
    }
    if INDEX_DEPTH.contains(&IndexType::Hour) {
        segments.push(IndexSegment::new(&time, &IndexType::Hour));
    }
    if INDEX_DEPTH.contains(&IndexType::Minute) {
        segments.push(IndexSegment::new(&time, &IndexType::Minute));
    }
    if INDEX_DEPTH.contains(&IndexType::Second) {
        segments.push(IndexSegment::new(&time, &IndexType::Second));
    }

    // add remainder chunk segment if it doesn't round evenly
    if *HAS_CHUNK_LEAVES {
        segments.push(IndexSegment::new_chunk(segments.last().unwrap(), &time));
    }

    segments
}

/// Attempt to compute the timestamp for an encoded index segment string using the specified format
///
fn timestamp_for_segment_str<S>(
    segment_data: &S,
    granularity: &IndexType,
    is_chunk_segment: bool,
) -> TimeIndexResult<DateTime<Utc>>
where
    S: AsRef<str> + std::fmt::Display,
{
    // make the segment data parseable first by adding start dates to incomplete values otherwise we'll hit ParseError::NotEnough
    let adjusted_segment_data = match granularity {
        IndexType::Year => format!("{}-01-01T00:00", segment_data),
        IndexType::Month => format!("{}-01T00:00", segment_data),
        IndexType::Day => format!("{}T00:00", segment_data),
        IndexType::Hour => format!("{}:00", segment_data),
        _ => segment_data.to_string(),
    };
    let try_format_str = granularity_to_format_string(granularity, true);

    if is_chunk_segment {
        // handle chunks differently by splitting off the chunk portion first
        // and adding offset milliseconds after parsing
        let (data_str, chunk_offset_str) = adjusted_segment_data.split_once('|').unwrap();
        match NaiveDateTime::parse_from_str(data_str, try_format_str.as_ref()) {
            Ok(raw_datetime) => Ok(DateTime::<Utc>::from_utc(
                raw_datetime
                    .checked_add_signed(Duration::milliseconds(
                        chunk_offset_str.parse::<i64>().unwrap(),
                    ))
                    .unwrap(),
                Utc,
            )),
            Err(_e) => Err(TimeIndexingError::Malformed(
                adjusted_segment_data.as_bytes().to_vec(),
            )),
        }
    } else {
        // for standard segments we can just parse using the appropriate (already determined) format string
        match NaiveDateTime::parse_from_str(adjusted_segment_data.as_ref(), try_format_str.as_ref())
        {
            Ok(raw_datetime) => Ok(DateTime::<Utc>::from_utc(raw_datetime, Utc)),
            Err(_e) => Err(TimeIndexingError::Malformed(
                adjusted_segment_data.as_bytes().to_vec(),
            )),
        }
    }
}

/// Decode a timestamp from a time index link tag.
///
/// Returns a `TimeIndexingError::Malformed` if an invalid link tag is passed.
///
fn decode_link_tag_timestamp(tag: LinkTag) -> TimeIndexResult<DateTime<Utc>> {
    // take the raw bytes of the LinkTag and split on the first null byte separator. All bytes following are the truncated timestamp as an encoded string.
    let bits: Vec<&[u8]> = tag.as_ref().splitn(2, |byte| *byte == 0x0 as u8).collect();

    // return an error on any invalid format
    let time_bytes = match bits.len() {
        2 => bits
            .last()
            .ok_or(TimeIndexingError::Malformed(tag.as_ref().to_owned())),
        _ => Err(TimeIndexingError::Malformed(tag.as_ref().to_owned())),
    }?;

    // interpret time data string
    let ts_str = String::from_utf8(time_bytes.to_vec())
        .map_err(|_e| TimeIndexingError::Malformed(tag.as_ref().to_owned()))?;
    let ts_str_is_chunk = ts_str.contains('|');

    // try parsing with all format strings in order of granularity until one matches
    // :TODO: there is probably a more intelligent & efficient gway of doing this
    timestamp_for_segment_str(&ts_str, &IndexType::Nanosecond, ts_str_is_chunk)
        .or(timestamp_for_segment_str(
            &ts_str,
            &IndexType::Second,
            ts_str_is_chunk,
        ))
        .or(timestamp_for_segment_str(
            &ts_str,
            &IndexType::Minute,
            ts_str_is_chunk,
        ))
        .or(timestamp_for_segment_str(
            &ts_str,
            &IndexType::Hour,
            ts_str_is_chunk,
        ))
        .or(timestamp_for_segment_str(
            &ts_str,
            &IndexType::Day,
            ts_str_is_chunk,
        ))
        .or(timestamp_for_segment_str(
            &ts_str,
            &IndexType::Month,
            ts_str_is_chunk,
        ))
        .or(timestamp_for_segment_str(
            &ts_str,
            &IndexType::Year,
            ts_str_is_chunk,
        ))
}
