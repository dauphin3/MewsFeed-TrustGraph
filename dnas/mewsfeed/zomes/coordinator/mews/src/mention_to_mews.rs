use crate::mew_with_context::get_batch_mews_with_context;
use hc_link_pagination::{paginate_by_hash, HashPagination};
use hdk::prelude::*;
use mews_integrity::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct AddMentionForMewInput {
    pub base_mention: AgentPubKey,
    pub target_mew_hash: ActionHash,
}
#[hdk_extern]
pub fn add_mention_for_mew(input: AddMentionForMewInput) -> ExternResult<()> {
    create_link(
        input.base_mention,
        input.target_mew_hash,
        LinkTypes::MentionToMews,
        (),
    )?;

    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetMewsForMentionInput {
    mention: AgentPubKey,
    page: Option<HashPagination>,
}
#[hdk_extern]
pub fn get_mews_for_mention(input: GetMewsForMentionInput) -> ExternResult<Vec<Record>> {
    let hashes = get_mew_hashes_for_mention(input.mention, input.page)?;
    let get_input: Vec<GetInput> = hashes
        .into_iter()
        .map(|hash| GetInput::new(hash.into(), GetOptions::default()))
        .collect();

    // Get the records to filter out the deleted ones
    let records: Vec<Record> = HDK
        .with(|hdk| hdk.borrow().get(get_input))?
        .into_iter()
        .flatten()
        .collect();

    Ok(records)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetMewsForMentionWithContextInput {
    mention: AgentPubKey,
    page: Option<HashPagination>,
}
#[hdk_extern]
pub fn get_mews_for_mention_with_context(
    input: GetMewsForMentionWithContextInput,
) -> ExternResult<Vec<FeedMew>> {
    let hashes = get_mew_hashes_for_mention(input.mention, input.page)?;

    get_batch_mews_with_context(hashes)
}

fn get_mew_hashes_for_mention(
    mention: AgentPubKey,
    page: Option<HashPagination>,
) -> ExternResult<Vec<ActionHash>> {
    let links: Vec<Link> = get_links(mention, LinkTypes::MentionToMews, None)?;
    let links_page = paginate_by_hash(links, page)?;

    let hashes: Vec<ActionHash> = links_page
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

#[derive(Serialize, Deserialize, Debug)]
pub struct RemoveMentionForMewInput {
    pub base_mention: AgentPubKey,
    pub target_mew_hash: ActionHash,
}
#[hdk_extern]
pub fn remove_mention_for_mew(input: RemoveMentionForMewInput) -> ExternResult<()> {
    let links = get_links(input.base_mention.clone(), LinkTypes::MentionToMews, None)?;

    for link in links {
        let action_hash = ActionHash::try_from(link.target.clone()).map_err(|_| {
            wasm_error!(WasmErrorInner::Guest(
                "Failed to convert link target to ActionHash".into()
            ))
        })?;
        if action_hash == input.target_mew_hash {
            delete_link(link.create_link_hash)?;
        }
    }

    Ok(())
}
