use hdk::prelude::*;
use mews_integrity::*;

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
