use crate::utils;
use hdk::PUBLIC_TOKEN;
use hdk::{
  entry_definition::ValidatingEntryType,
  error::{ZomeApiError, ZomeApiResult},
  holochain_core_types::{
    cas::content::Address, dna::entry_types::Sharing, entry::Entry, error::HolochainError,
    json::JsonString, link::LinkMatch, signature::Provenance,
  },
  AGENT_ADDRESS,
};
use holochain_wasm_utils::api_serialization::get_entry::{GetEntryOptions, GetEntryResult};
use std::convert::TryInto;

#[derive(Serialize, Deserialize, Debug, DefaultJson, Clone)]
pub struct Perspective {
  origin: String,
  creatorId: Address,
  timestamp: u128,
  contextId: Address,
  name: String,
}

impl Perspective {
  pub fn new(
    name: &str,
    timestamp: &u128,
    creator: &Address,
    context_address: &Address,
  ) -> Perspective {
    Perspective {
      origin: crate::get_origin(),
      creatorId: creator.to_owned(),
      timestamp: timestamp.to_owned(),
      contextId: context_address.to_owned(),
      name: name.to_owned(),
    }
  }
}

pub fn definition() -> ValidatingEntryType {
  entry!(
    name: "perspective",
    description: "perspective pointing to a proxied commit",
    sharing: Sharing::Public,

    validation_package: || {
      hdk::ValidationPackageDefinition::ChainFull
    },

    validation: |_ctx: hdk::EntryValidationData<Perspective>| {
      Ok(())
    },

    links: [
      to!(
        "proxy",
        link_type: "head",
        validation_package: || {
          hdk::ValidationPackageDefinition::ChainFull
        },
        validation: |_validation_data: hdk::LinkValidationData | {
          Ok(())
        }
      ),
      from!(
        "proxy",
        link_type: "perspectives",
        validation_package: || {
          hdk::ValidationPackageDefinition::ChainFull
        },
        validation: |_validation_data: hdk::LinkValidationData | {
          Ok(())
        }
      )
    ]
  )
}

/** Zome exposed functions */

/**
 * Creates a new perspective with the given properties,
 * and associates its previous address if present
 */
pub fn handle_create_perspective(
  previous_address: Option<Address>,
  perspective: Perspective,
) -> ZomeApiResult<Address> {
  let perspective_entry = Entry::App("perspective".into(), perspective.clone().into());
  // TODO change for create_entry_custom_provenance
  let perspective_address = utils::store_entry_if_new(&perspective_entry)?;

  link_context_to_perspective(perspective.contextId, perspective_address.clone())?;

  utils::set_entry_proxy(
    perspective_address.clone(),
    Some(perspective_address.clone()),
  )?;

  if let Some(proxy_address) = previous_address {
    utils::set_entry_proxy(proxy_address.clone(), Some(perspective_address.clone()))?;
  }

  Ok(perspective_address)
}

/**
 * Retrieves the information about the perspective
 */
pub fn handle_get_perspective_info(perspective_address: Address) -> ZomeApiResult<GetEntryResult> {
  hdk::get_entry_result(&perspective_address, GetEntryOptions::default())
}

/**
 * Returns the address of the head commit for the given perspective
 */
pub fn handle_get_perspective_head(perspective_address: Address) -> ZomeApiResult<Address> {
  let internal_perspective_address = get_internal_address(perspective_address)?;

  let response = hdk::call(
    hdk::THIS_INSTANCE,
    "proxy",
    Address::from(PUBLIC_TOKEN.to_string()),
    "get_links_to_proxy",
    json!({ "base_address": internal_perspective_address, "link_type": "head", "tag": ""}).into(),
  )?;

  let links_result: ZomeApiResult<Vec<Address>> = response.try_into()?;
  let links = links_result?;

  if links.len() == 0 {
    return Err(ZomeApiError::from(String::from(
      "given perspective has no commits",
    )));
  }

  Ok(links[0].clone())
}

/**
 * Sets the given perspective head pointing to the given commit head
 */
pub fn handle_update_perspective_head(
  perspective_address: Address,
  head_address: Address,
) -> ZomeApiResult<()> {
  // Perspective address can be a proxy address, get the internal address
  let internal_perspective_address = get_internal_address(perspective_address)?;

  utils::remove_previous_links(
    &internal_perspective_address,
    Some(String::from("head")),
    None,
  )?;

  link_perspective_to_commit(internal_perspective_address.clone(), head_address)?;

  Ok(())
}

/** Proxy handlers */

fn get_internal_address(perspective_address: Address) -> ZomeApiResult<Address> {
  let response = hdk::call(
    hdk::THIS_INSTANCE,
    "proxy",
    Address::from(PUBLIC_TOKEN.to_string()),
    "get_internal_address",
    json!({ "proxy_address": perspective_address }).into(),
  )?;
  let result: ZomeApiResult<Option<Address>> = response.try_into()?;

  match result? {
    Some(internal_address) => Ok(internal_address),
    None => Err(ZomeApiError::from(format!(
      "entry with hash {} does not exist",
      perspective_address
    ))),
  }
}

pub fn link_perspective_to_commit(
  perspective_address: Address,
  commit_address: Address,
) -> ZomeApiResult<()> {
  // Head commit may not exist on this hApp, we have to set its proxy address and use that entry to link
  utils::set_entry_proxy(commit_address.clone(), Some(commit_address.clone()))?;

  let response = hdk::call(
    hdk::THIS_INSTANCE,
    "proxy",
    Address::from(PUBLIC_TOKEN.to_string()),
    "link_to_proxy",
    json!({ "base_address": perspective_address, "proxy_address": commit_address, "link_type": "head", "tag": ""}).into(),
  )?;

  let _result: ZomeApiResult<Address> = response.try_into()?;
  let _address = _result?;

  Ok(())
}

pub fn link_context_to_perspective(
  context_address: Address,
  perspective_address: Address,
) -> ZomeApiResult<()> {
  // Context may not exist on this hApp, we have to set its proxy address and use that entry to link
  utils::set_entry_proxy(context_address.clone(), Some(context_address.clone()))?;

  let response = hdk::call(
    hdk::THIS_INSTANCE,
    "proxy",
    Address::from(PUBLIC_TOKEN.to_string()),
    "link_from_proxy",
    json!({"proxy_address": context_address, "to_address": perspective_address, "link_type": "perspectives", "tag": ""}).into(),
  )?;

  let _result: ZomeApiResult<Address> = response.try_into()?;
  let _address = _result?;

  Ok(())
}
