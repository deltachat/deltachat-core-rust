use std::collections::BTreeMap;

use deltachat::reaction::Reactions;
use serde::Serialize;
use typescript_type_def::TypeDef;

/// Structure representing all reactions to a particular message.
#[derive(Serialize, TypeDef)]
#[serde(rename = "Reactions", rename_all = "camelCase")]
pub struct JSONRPCReactions {
    /// Map from a contact to it's reaction to message.
    reactions_by_contact: BTreeMap<u32, Vec<String>>,
    /// Unique reactions and their count
    reactions: BTreeMap<String, u32>,
}

impl From<Reactions> for JSONRPCReactions {
    fn from(reactions: Reactions) -> Self {
        let mut reactions_by_contact: BTreeMap<u32, Vec<String>> = BTreeMap::new();
        let mut unique_reactions: BTreeMap<String, u32> = BTreeMap::new();

        for contact_id in reactions.contacts() {
            let reaction = reactions.get(contact_id);
            if reaction.is_empty() {
                continue;
            }
            let emojis: Vec<String> = reaction
                .emojis()
                .into_iter()
                .map(|emoji| emoji.to_owned())
                .collect();
            reactions_by_contact.insert(contact_id.to_u32(), emojis.clone());
            for emoji in emojis {
                if let Some(x) = unique_reactions.get_mut(&emoji) {
                    *x += 1;
                } else {
                    unique_reactions.insert(emoji, 1);
                }
            }
        }

        JSONRPCReactions {
            reactions_by_contact,
            reactions: unique_reactions,
        }
    }
}
