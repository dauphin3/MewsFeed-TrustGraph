use hdk::prelude::*;

pub fn validate_create_link_time_index(
    _action: CreateLink,
    _base_address: AnyLinkableHash,
    _target_address: AnyLinkableHash,
    _tag: LinkTag,
    _index_name: impl Into<String> + Clone,
) -> ExternResult<ValidateCallbackResult> {
    // @todo validate time index

    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_delete_link_time_index(
    _action: DeleteLink,
    _original_action: CreateLink,
    _base: AnyLinkableHash,
    _target: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(String::from(
        "TimeIndex links cannot be deleted",
    )))
}
