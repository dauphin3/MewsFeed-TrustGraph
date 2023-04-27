use crate::mew::get_mew_with_context;
use hdk::prelude::*;
pub use hdk_time_indexing::{get_latest_hashes, get_older_hashes, read_all_hashes};
use mews_integrity::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct GetLatestMewsInput {
    pub limit: usize,
    pub before_mew_hash: Option<ActionHash>,
}
#[hdk_extern]
pub fn get_latest_mews(input: GetLatestMewsInput) -> ExternResult<Vec<Record>> {
    let hashes = get_latest_mew_hashes(input.limit, input.before_mew_hash)?;

    let get_input: Vec<GetInput> = hashes
        .into_iter()
        .map(|hash| GetInput::new(hash.into(), GetOptions::default()))
        .collect();
    let records = HDK.with(|hdk| hdk.borrow().get(get_input))?;
    let records: Vec<Record> = records.into_iter().flatten().collect();

    Ok(records)
}

#[hdk_extern]
pub fn get_latest_mews_with_context(input: GetLatestMewsInput) -> ExternResult<Vec<FeedMew>> {
    let hashes = get_latest_mew_hashes(input.limit, input.before_mew_hash)?;

    let feedmews: Vec<FeedMew> = hashes
        .into_iter()
        .filter_map(|hash| get_mew_with_context(hash).ok())
        .collect();

    Ok(feedmews)
}

fn get_latest_mew_hashes(
    limit: usize,
    before_mew_hash: Option<ActionHash>,
) -> ExternResult<Vec<ActionHash>> {
    let hashes = match before_mew_hash {
        Some(hash) => get_older_hashes(
            &MEW_TIME_INDEX_NAME.to_owned(),
            LinkTypes::MewTimeIndex,
            AnyLinkableHash::from(hash),
            limit,
        )
        .map_err(|_| wasm_error!(WasmErrorInner::Guest("Failed to get older hashes".into())))?,
        None => get_latest_hashes(
            &MEW_TIME_INDEX_NAME.to_owned(),
            LinkTypes::MewTimeIndex,
            limit,
        )
        .map_err(|_| wasm_error!(WasmErrorInner::Guest("Failed to get latest hashes".into())))?,
    };

    let action_hashes: Vec<ActionHash> = hashes.into_iter().map(ActionHash::from).collect();

    Ok(action_hashes)
}
