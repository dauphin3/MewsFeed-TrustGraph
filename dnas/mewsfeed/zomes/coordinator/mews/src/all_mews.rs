use crate::mew_with_context::get_batch_mews_with_context;
use hdk::prelude::*;
use mews_integrity::*;

#[hdk_extern]
pub fn get_all_mews(_: ()) -> ExternResult<Vec<Record>> {
    let hashes = get_all_mew_hashes()?;
    let get_input: Vec<GetInput> = hashes
        .into_iter()
        .map(|hash| GetInput::new(hash.into(), GetOptions::default()))
        .collect();
    let records = HDK.with(|hdk| hdk.borrow().get(get_input))?;
    let records: Vec<Record> = records.into_iter().flatten().collect();

    Ok(records)
}

#[hdk_extern]
pub fn get_all_mews_with_context(_: ()) -> ExternResult<Vec<FeedMew>> {
    let hashes = get_all_mew_hashes()?;

    get_batch_mews_with_context(hashes)
}

pub fn get_all_mew_hashes() -> ExternResult<Vec<ActionHash>> {
    let path = Path::from("all_mews");
    let mut links = get_links(path.path_entry_hash()?, LinkTypes::AllMews, None)?;
    links.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    let hashes: Vec<ActionHash> = links
        .into_iter()
        .map(|link| {
            ActionHash::try_from(link.target).map_err(|_| {
                wasm_error!(WasmErrorInner::Guest(
                    "Failed to convert link target to ActionHash".into()
                ))
            })
        })
        .collect::<ExternResult<Vec<ActionHash>>>()?;

    Ok(hashes)
}
