//! A store containing registered MQTT entities.
//!
//! References:
//!
//! - <https://github.com/thin-edge/thin-edge.io/issues/2081>
//! - <https://thin-edge.github.io/thin-edge.io/next/references/mqtt-api/#entity-store>

// TODO: move entity business logic to its own module

use crate::entity_store;
use crate::mqtt_topics::Channel;
use crate::mqtt_topics::EntityTopicId;
use crate::mqtt_topics::MqttSchema;
use crate::mqtt_topics::TopicIdError;
use log::debug;
use mqtt_channel::Message;
use serde_json::json;
use serde_json::Map;
use serde_json::Value as JsonValue;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::Display;
use thiserror::Error;

/// Represents an "Entity topic identifier" portion of the MQTT topic
///
/// Example:
/// - topic: `te/device/dev1/service/myservice/m//my_measurement`
/// - entity id: `device/dev1/service/myservice`
///
/// <https://thin-edge.github.io/thin-edge.io/next/references/mqtt-api/#group-identifier>

// In the future, root will be read from config
const MQTT_ROOT: &str = "te";

/// Represents externally provided unique ID of an entity.
///
/// Although this struct doesn't enforce any restrictions for the values,
/// the consumers may impose restrictions on the accepted values.

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct EntityExternalId(String);

impl AsRef<str> for EntityExternalId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// XXX: As `EntityExternalId` is used as a part of cloudbound MQTT topic, it
// can't contain characters invalid in topics, i.e. `+` and `#`. ([MQTT-4.7]).
// If it's derived from a MQTT topic, this holds, but if created from a string,
// this isn't checked, which is invalid!
impl From<&str> for EntityExternalId {
    fn from(val: &str) -> Self {
        Self(val.to_string())
    }
}

impl From<&String> for EntityExternalId {
    fn from(val: &String) -> Self {
        Self(val.to_string())
    }
}

impl From<String> for EntityExternalId {
    fn from(val: String) -> Self {
        Self(val)
    }
}

impl From<EntityExternalId> for String {
    fn from(value: EntityExternalId) -> Self {
        value.0
    }
}

impl From<&EntityExternalId> for String {
    fn from(value: &EntityExternalId) -> Self {
        value.0.clone()
    }
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Invalid external ID: {external_id} contains invalid character: {invalid_char}")]
pub struct InvalidExternalIdError {
    pub external_id: String,
    pub invalid_char: char,
}

type ExternalIdMapperFn =
    Box<dyn Fn(&EntityTopicId, &EntityExternalId) -> EntityExternalId + Send + Sync + 'static>;
type ExternalIdValidatorFn =
    Box<dyn Fn(&str) -> Result<EntityExternalId, InvalidExternalIdError> + Send + Sync + 'static>;

/// A store for topic-based entity metadata lookup.
///
/// This object is a hashmap from MQTT identifiers to entities (devices or
/// services) that publish on those topics. It keeps track of type of entities,
/// their relationships (parent and child entities), and other metadata.
///
/// The entity store takes as input registration messages published by entities
/// (devices and services) and stores information about entities and their
/// hierarchy, allowing to efficiently query it. It's possible to:
///
/// - enumerate all registered devices
/// - check if a given entity is already registered
/// - query services and child devices of a given device
/// - query parent of an entity
///
/// # Examples
///
/// ```
/// # use mqtt_channel::{Message, Topic};
/// # use tedge_api::entity_store::{EntityStore, EntityRegistrationMessage};
/// let mqtt_message = Message::new(
///     &Topic::new("te/device/main//").unwrap(),
///     r#"{"@type": "device"}"#.to_string(),
/// );
/// let registration_message = EntityRegistrationMessage::try_from(&mqtt_message).unwrap();
///
/// let mut entity_store = EntityStore::with_main_device(
///     registration_message,
///     |tid, xid| tid.to_string().into(),
///     |xid| Ok(xid.into()),
/// );
/// ```
pub struct EntityStore {
    main_device: EntityTopicId,
    entities: HashMap<EntityTopicId, EntityMetadata>,
    entity_id_index: HashMap<EntityExternalId, EntityTopicId>,
    external_id_mapper: ExternalIdMapperFn,
    external_id_validator_fn: ExternalIdValidatorFn,
    // TODO: this is a c8y cloud specific concern and it'd be better to put it somewhere else.
    default_service_type: String,
}

impl EntityStore {
    /// Creates a new entity store with a given main device.
    #[must_use]
    pub fn with_main_device<MF, SF>(
        main_device: EntityRegistrationMessage,
        external_id_mapper_fn: MF,
        external_id_validator_fn: SF,
    ) -> Option<Self>
    where
        MF: Fn(&EntityTopicId, &EntityExternalId) -> EntityExternalId,
        MF: 'static + Send + Sync,
        SF: Fn(&str) -> Result<EntityExternalId, InvalidExternalIdError>,
        SF: 'static + Send + Sync,
    {
        Self::with_main_device_and_default_service_type(
            main_device,
            "service".to_string(),
            external_id_mapper_fn,
            external_id_validator_fn,
        )
    }

    #[must_use]
    pub fn with_main_device_and_default_service_type<MF, SF>(
        main_device: EntityRegistrationMessage,
        default_service_type: String,
        external_id_mapper_fn: MF,
        external_id_validator_fn: SF,
    ) -> Option<Self>
    where
        MF: Fn(&EntityTopicId, &EntityExternalId) -> EntityExternalId,
        MF: 'static + Send + Sync,
        SF: Fn(&str) -> Result<EntityExternalId, InvalidExternalIdError>,
        SF: 'static + Send + Sync,
    {
        if main_device.r#type != EntityType::MainDevice {
            return None;
        }

        let entity_id: EntityExternalId = main_device.external_id?;
        let metadata = EntityMetadata {
            topic_id: main_device.topic_id.clone(),
            external_id: entity_id.clone(),
            r#type: main_device.r#type,
            parent: None,
            other: main_device.other,
            twin_data: Map::new(),
        };

        Some(EntityStore {
            main_device: main_device.topic_id.clone(),
            entities: HashMap::from([(main_device.topic_id.clone(), metadata)]),
            entity_id_index: HashMap::from([(entity_id, main_device.topic_id)]),
            external_id_mapper: Box::new(external_id_mapper_fn),
            external_id_validator_fn: Box::new(external_id_validator_fn),
            default_service_type,
        })
    }

    /// Returns information about an entity under a given MQTT entity topic identifier.
    pub fn get(&self, entity_topic_id: &EntityTopicId) -> Option<&EntityMetadata> {
        self.entities.get(entity_topic_id)
    }

    /// Returns a mutable reference to the `EntityMetadata` for the given `EntityTopicId`.
    fn get_mut(&mut self, entity_topic_id: &EntityTopicId) -> Option<&mut EntityMetadata> {
        self.entities.get_mut(entity_topic_id)
    }

    /// Tries to get information about an entity using its `EntityTopicId`,
    /// returning an error if the entity is not registered.
    pub fn try_get(&self, entity_topic_id: &EntityTopicId) -> Result<&EntityMetadata, Error> {
        self.get(entity_topic_id)
            .ok_or_else(|| Error::UnknownEntity(entity_topic_id.to_string()))
    }

    /// Tries to get a mutable reference to the `EntityMetadata` for the given `EntityTopicId`,
    /// returning an error if the entity is not registered.
    fn try_get_mut(
        &mut self,
        entity_topic_id: &EntityTopicId,
    ) -> Result<&mut EntityMetadata, Error> {
        self.get_mut(entity_topic_id)
            .ok_or_else(|| Error::UnknownEntity(entity_topic_id.to_string()))
    }

    /// Returns information for an entity under a given device/service id.
    pub fn get_by_external_id(&self, external_id: &EntityExternalId) -> Option<&EntityMetadata> {
        let topic_id = self.entity_id_index.get(external_id)?;
        self.get(topic_id)
    }

    /// Tries to get information about an entity using its `EntityExternalId`,
    /// returning an error if the entity is not registered.
    pub fn try_get_by_external_id(
        &self,
        external_id: &EntityExternalId,
    ) -> Result<&EntityMetadata, Error> {
        self.get_by_external_id(external_id)
            .ok_or_else(|| Error::UnknownEntity(external_id.into()))
    }

    /// Returns the MQTT identifier of the main device.
    ///
    /// The main device is an entity with `@type: "device"`.
    pub fn main_device(&self) -> &EntityTopicId {
        &self.main_device
    }

    /// Returns the external id of the main device.
    pub fn main_device_external_id(&self) -> EntityExternalId {
        self.get(&self.main_device).unwrap().external_id.clone()
    }

    /// Returns an ordered list of ancestors of the given entity
    /// starting from the immediate parent all the way till the root main device.
    /// The last parent in the list for any entity would always be the main device.
    /// The list would be empty for the main device as it has no further parents.
    pub fn ancestors(&self, entity_topic_id: &EntityTopicId) -> Result<Vec<&EntityTopicId>, Error> {
        if self.entities.get(entity_topic_id).is_none() {
            return Err(Error::UnknownEntity(entity_topic_id.to_string()));
        }

        let mut ancestors = vec![];

        let mut current_entity_id = entity_topic_id;
        while let Some(entity) = self.entities.get(current_entity_id) {
            if let Some(parent_id) = &entity.parent {
                ancestors.push(parent_id);
                current_entity_id = parent_id;
            } else {
                break; // No more parents
            }
        }

        Ok(ancestors)
    }

    /// Returns an ordered list of ancestors' external ids of the given entity
    /// starting from the immediate parent all the way till the root main device.
    /// The last parent in the list for any entity would always be the main device id.
    pub fn ancestors_external_ids(
        &self,
        entity_topic_id: &EntityTopicId,
    ) -> Result<Vec<String>, Error> {
        let mapped_ancestors = self
            .ancestors(entity_topic_id)?
            .iter()
            .map(|tid| {
                self.entities
                    .get(tid)
                    .map(|e| e.external_id.clone().into())
                    .unwrap()
            })
            .collect();

        Ok(mapped_ancestors)
    }

    /// Returns MQTT identifiers of child devices of a given device.
    pub fn child_devices(&self, entity_topic: &EntityTopicId) -> Vec<&EntityTopicId> {
        self.entities
            .iter()
            .filter(|(_, e)| {
                // can be replaced by `is_some_and` after MSRV upgrade to 1.70
                e.parent.as_ref().map_or(false, |p| p == entity_topic)
                    && e.r#type == EntityType::ChildDevice
            })
            .map(|(k, _)| k)
            .collect()
    }

    /// Returns MQTT identifiers of services running on a given device.
    pub fn services(&self, entity_topic: &EntityTopicId) -> Vec<&EntityTopicId> {
        self.entities
            .iter()
            .filter(|(_, e)| {
                // can be replaced by `is_some_and` after MSRV upgrade to 1.70
                e.parent.as_ref().map_or(false, |p| p == entity_topic)
                    && e.r#type == EntityType::Service
            })
            .map(|(k, _)| k)
            .collect()
    }

    /// Updates entity store state based on the content of the entity
    /// registration message.
    ///
    /// It can register a new entity in the store or update already registered
    /// entity, returning a list of all entities affected by the update, e.g.:
    ///
    /// - when adding/removing a child device or service, the parent is affected
    pub fn update(
        &mut self,
        message: EntityRegistrationMessage,
    ) -> Result<Vec<EntityTopicId>, Error> {
        debug!("Processing entity registration message, {:?}", message);
        let topic_id = message.topic_id;

        let mut affected_entities = vec![];

        let parent = match message.r#type {
            EntityType::MainDevice => None,
            EntityType::ChildDevice => message.parent.or_else(|| Some(self.main_device.clone())),
            EntityType::Service => message
                .parent
                .or_else(|| topic_id.default_parent_identifier())
                .or_else(|| Some(self.main_device.clone())),
        };

        // parent device is affected if new device is its child
        if let Some(parent) = &parent {
            if !self.entities.contains_key(parent) {
                return Err(Error::NoParent(parent.to_string().into_boxed_str()));
            }

            affected_entities.push(parent.clone());
        }

        let external_id = match message.r#type {
            EntityType::MainDevice => self.main_device_external_id(),
            _ => {
                if let Some(id) = message.external_id {
                    (self.external_id_validator_fn)(id.as_ref())?
                } else {
                    (self.external_id_mapper)(&topic_id, &self.main_device_external_id())
                }
            }
        };

        let mut other = message.other;

        if message.r#type == EntityType::Service {
            other
                .entry("type".to_string())
                .or_insert(JsonValue::String(self.default_service_type.clone()));
        }

        let entity_metadata = EntityMetadata {
            topic_id: topic_id.clone(),
            r#type: message.r#type,
            external_id: external_id.clone(),
            parent,
            other,
            twin_data: Map::new(),
        };

        // device is affected if it was previously registered and was updated
        // (i.e. EntityMetadata has changed)
        let previous = self.entities.entry(topic_id.clone());
        match previous {
            Entry::Occupied(mut occupied) => {
                // if there is no change, no entities were affected
                let mut existing_entity = occupied.get().clone();
                if existing_entity == entity_metadata {
                    return Ok(vec![]);
                }

                existing_entity.other.extend(entity_metadata.other.clone());
                let merged_entity = EntityMetadata {
                    twin_data: existing_entity.twin_data,
                    other: existing_entity.other,
                    ..entity_metadata
                };

                occupied.insert(merged_entity);
                affected_entities.push(topic_id);
            }
            Entry::Vacant(vacant) => {
                vacant.insert(entity_metadata);
                self.entity_id_index.insert(external_id, topic_id);
            }
        }
        debug!("Updated entity map: {:?}", self.entities);
        debug!("Updated external id map: {:?}", self.entity_id_index);

        Ok(affected_entities)
    }

    /// An iterator over all registered entities.
    pub fn iter(&self) -> impl Iterator<Item = (&EntityTopicId, &EntityMetadata)> {
        self.entities.iter()
    }

    /// Performs auto-registration process for an entity under a given
    /// identifier.
    ///
    /// If an entity is a service, its parent device is also auto-registered if it's
    /// not already registered.
    ///
    /// It returns MQTT register messages for the given entities to be published
    /// by the mapper, so other components can also be aware of a new device
    /// being registered.
    pub fn auto_register_entity(
        &mut self,
        entity_topic_id: &EntityTopicId,
    ) -> Result<Vec<EntityRegistrationMessage>, entity_store::Error> {
        if entity_topic_id.matches_default_topic_scheme() {
            if entity_topic_id.is_default_main_device() {
                return Ok(vec![]); // Do nothing as the main device is always pre-registered
            }

            let mut register_messages = vec![];

            let parent_device_id = entity_topic_id
                .default_parent_identifier()
                .expect("device id must be present as the topic id follows the default scheme");

            if !parent_device_id.is_default_main_device() && self.get(&parent_device_id).is_none() {
                let device_local_id = entity_topic_id.default_device_name().unwrap();
                let device_external_id =
                    (self.external_id_mapper)(&parent_device_id, &self.main_device_external_id());

                let device_register_message = EntityRegistrationMessage {
                    topic_id: parent_device_id.clone(),
                    external_id: Some(device_external_id),
                    r#type: EntityType::ChildDevice,
                    parent: None,
                    other: json!({ "name": device_local_id })
                        .as_object()
                        .unwrap()
                        .to_owned(),
                };
                register_messages.push(device_register_message.clone());
                self.update(device_register_message)?;
            }

            // if the entity is a service, register the service as well
            if let Some(service_id) = entity_topic_id.default_service_name() {
                let service_external_id =
                    (self.external_id_mapper)(entity_topic_id, &self.main_device_external_id());

                let service_register_message = EntityRegistrationMessage {
                    topic_id: entity_topic_id.clone(),
                    external_id: Some(service_external_id),
                    r#type: EntityType::Service,
                    parent: Some(parent_device_id),
                    other: json!({ "name": service_id, "type": self.default_service_type })
                        .as_object()
                        .unwrap()
                        .to_owned(),
                };
                register_messages.push(service_register_message.clone());
                self.update(service_register_message)?;
            }

            Ok(register_messages)
        } else {
            Err(Error::NonDefaultTopicScheme(entity_topic_id.clone()))
        }
    }

    /// Updates the entity twin data with the provided fragment data.
    /// Returns `true`, if the twin data got updated with the new fragment value.
    /// If the provided fragment already existed, `false` is returned.
    pub fn update_twin_data(
        &mut self,
        entity_topic_id: &EntityTopicId,
        fragment_key: String,
        fragment_value: JsonValue,
    ) -> Result<bool, entity_store::Error> {
        let entity = self.try_get_mut(entity_topic_id)?;
        if fragment_value.is_null() {
            let existing = entity.twin_data.remove(&fragment_key);
            if existing.is_none() {
                return Ok(false);
            }
        } else {
            let existing = entity
                .twin_data
                .insert(fragment_key, fragment_value.clone());
            if existing.is_some_and(|v| v.eq(&fragment_value)) {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityMetadata {
    pub topic_id: EntityTopicId,
    pub parent: Option<EntityTopicId>,
    pub r#type: EntityType,
    pub external_id: EntityExternalId,

    // TODO: use a dedicated struct for cloud-specific fields, have `EntityMetadata` be generic over
    // cloud we're currently connected to
    pub other: Map<String, JsonValue>,
    pub twin_data: Map<String, JsonValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntityType {
    MainDevice,
    ChildDevice,
    Service,
}

impl Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntityType::MainDevice => write!(f, "device"),
            EntityType::ChildDevice => write!(f, "child-device"),
            EntityType::Service => write!(f, "service"),
        }
    }
}

impl EntityMetadata {
    /// Creates a entity metadata for the main device.
    pub fn main_device(device_id: String) -> Self {
        Self {
            topic_id: EntityTopicId::default_main_device(),
            external_id: device_id.into(),
            r#type: EntityType::MainDevice,
            parent: None,
            other: Map::new(),
            twin_data: Map::new(),
        }
    }

    /// Creates a entity metadata for a child device.
    pub fn child_device(child_device_id: String) -> Result<Self, TopicIdError> {
        Ok(Self {
            topic_id: EntityTopicId::default_child_device(&child_device_id)?,
            external_id: child_device_id.into(),
            r#type: EntityType::ChildDevice,
            parent: Some(EntityTopicId::default_main_device()),
            other: Map::new(),
            twin_data: Map::new(),
        })
    }
}

/// Represents an error encountered while updating the store.
#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
pub enum Error {
    #[error("Specified parent {0:?} does not exist in the store")]
    NoParent(Box<str>),

    #[error("Main device was not registered. Before registering child entities, register the main device")]
    NoMainDevice,

    #[error("The main device was already registered at topic {0}")]
    MainDeviceAlreadyRegistered(Box<str>),

    #[error("The specified entity {0} does not exist in the store")]
    UnknownEntity(String),

    #[error("Auto registration of the entity with topic id {0} failed as it does not match the default topic scheme: 'device/<device-id>/service/<service-id>'. Try explicit registration instead.")]
    NonDefaultTopicScheme(EntityTopicId),

    #[error(transparent)]
    InvalidExternalIdError(#[from] InvalidExternalIdError),

    // In practice won't be thrown because usually it is a map
    // TODO: remove this error variant when `EntityRegistrationMessage` is changed
    #[error("`EntityRegistrationMessage::other` field needs to be a Map")]
    EntityRegistrationOtherNotMap,
}

/// An object representing a valid entity registration message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityRegistrationMessage {
    // fields used by thin-edge locally
    pub topic_id: EntityTopicId,
    pub external_id: Option<EntityExternalId>,
    pub r#type: EntityType,
    pub parent: Option<EntityTopicId>,

    // other properties, usually cloud-specific
    // TODO: replace with `Map` and use type wrapper that forbids fields `@id`,
    // `@parent`, etc.
    pub other: Map<String, JsonValue>,
}

impl EntityRegistrationMessage {
    /// Parses a MQTT message as an entity registration message.
    ///
    /// MQTT message is an entity registration message if
    /// - published on a prefix of `te/+/+/+/+`
    /// - its payload contains a registration message.
    // TODO: replace option with proper error handling
    // TODO: this is basically manual Deserialize implementation, better impl
    // Serialize/Deserialize.
    #[must_use]
    pub fn new(message: &Message) -> Option<Self> {
        let topic_id = message
            .topic
            .name
            .strip_prefix(MQTT_ROOT)
            .and_then(|s| s.strip_prefix('/'))?;

        let payload = parse_entity_register_payload(message.payload_bytes())?;

        let JsonValue::Object(mut properties) = payload else {
            return None;
        };

        let Some(JsonValue::String(r#type)) = properties.remove("@type") else {
            return None;
        };

        let r#type = match r#type.as_str() {
            "device" => EntityType::MainDevice,
            "child-device" => EntityType::ChildDevice,
            "service" => EntityType::Service,
            _ => return None,
        };

        let parent = properties.remove("@parent");
        let parent = if let Some(parent) = parent {
            let JsonValue::String(parent) = parent else {
                return None;
            };
            let Ok(parent) = parent.parse() else {
                return None;
            };
            Some(parent)
        } else {
            None
        };

        let parent = if r#type == EntityType::ChildDevice || r#type == EntityType::Service {
            parent
        } else {
            None
        };

        let entity_id = properties.remove("@id");
        let entity_id = if let Some(entity_id) = entity_id {
            let JsonValue::String(entity_id) = entity_id else {
                return None;
            };
            Some(entity_id.into())
        } else {
            None
        };

        let other = properties;

        assert_eq!(other.get("@id"), None);
        assert_eq!(other.get("@type"), None);
        assert_eq!(other.get("@parent"), None);

        Some(Self {
            topic_id: topic_id.parse().ok()?,
            external_id: entity_id,
            r#type,
            parent,
            other,
        })
    }

    /// Creates a entity registration message for a main device.
    pub fn main_device(main_device_id: String) -> Self {
        Self {
            topic_id: EntityTopicId::default_main_device(),
            external_id: Some(main_device_id.into()),
            r#type: EntityType::MainDevice,
            parent: None,
            other: Map::new(),
        }
    }

    // TODO: manual serialize impl
    pub fn to_mqtt_message(mut self, mqtt_schema: &MqttSchema) -> Message {
        let mut props = serde_json::Map::new();

        props.insert("@type".to_string(), self.r#type.to_string().into());

        if let Some(external_id) = self.external_id {
            props.insert("@id".to_string(), external_id.as_ref().to_string().into());
        }

        if let Some(parent) = self.parent {
            props.insert("@parent".to_string(), parent.to_string().into());
        }

        props.append(&mut self.other);

        let message = serde_json::to_string(&props).unwrap();

        let message_topic = mqtt_schema.topic_for(&self.topic_id, &Channel::EntityMetadata);
        Message::new(&message_topic, message).with_retain()
    }
}

impl TryFrom<&Message> for EntityRegistrationMessage {
    type Error = ();

    fn try_from(value: &Message) -> Result<Self, Self::Error> {
        EntityRegistrationMessage::new(value).ok_or(())
    }
}

/// Parse a MQTT message payload as an entity registration payload.
///
/// Returns `Some(register_payload)` if a payload is valid JSON and is a
/// registration payload, or `None` otherwise.
fn parse_entity_register_payload(payload: &[u8]) -> Option<JsonValue> {
    let payload = serde_json::from_slice::<JsonValue>(payload).ok()?;

    if payload.get("@type").is_some() {
        Some(payload)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;
    use mqtt_channel::Topic;
    use serde_json::json;
    use std::collections::HashSet;
    use std::str::FromStr;

    fn dummy_external_id_mapper(
        entity_topic_id: &EntityTopicId,
        _main_device_xid: &EntityExternalId,
    ) -> EntityExternalId {
        entity_topic_id
            .to_string()
            .trim_end_matches('/')
            .replace('/', ":")
            .into()
    }

    fn dummy_external_id_sanitizer(id: &str) -> Result<EntityExternalId, InvalidExternalIdError> {
        let forbidden_chars = HashSet::from(['/', '+', '#']);
        for c in id.chars() {
            if forbidden_chars.contains(&c) {
                return Err(InvalidExternalIdError {
                    external_id: id.into(),
                    invalid_char: c,
                });
            }
        }
        Ok(id.into())
    }

    #[test]
    fn parse_entity_registration_message() {
        let message = Message::new(
            &Topic::new("te/device/child1//").unwrap(),
            json!({
                "@type" : "child-device",
                "name": "child1",
                "type": "RPi",
                "version": "5",
                "complex": {
                    "foo" : "bar"
                }
            })
            .to_string(),
        );
        let parsed = EntityRegistrationMessage::new(&message).unwrap();
        assert_eq!(parsed.r#type, EntityType::ChildDevice);
        assert_eq!(parsed.other.get("name").unwrap(), "child1");
        assert_eq!(parsed.other.get("type").unwrap(), "RPi");
        assert_eq!(parsed.other.get("version").unwrap(), "5");
        assert_eq!(
            parsed.other.get("complex").unwrap().get("foo").unwrap(),
            "bar"
        );
    }

    #[test]
    fn registers_main_device() {
        let store = new_entity_store();

        assert_eq!(store.main_device(), &EntityTopicId::default_main_device());
        assert!(store.get(&EntityTopicId::default_main_device()).is_some());
    }

    #[test]
    fn lists_child_devices() {
        let mut store = new_entity_store();

        // If the @parent info is not provided, it is assumed to be an immediate
        // child of the main device.
        let updated_entities = store
            .update(
                EntityRegistrationMessage::new(&Message::new(
                    &Topic::new("te/device/child1//").unwrap(),
                    json!({"@type": "child-device"}).to_string(),
                ))
                .unwrap(),
            )
            .unwrap();

        assert_eq!(updated_entities, ["device/main//"]);
        assert_eq!(
            store.child_devices(&EntityTopicId::default_main_device()),
            ["device/child1//"]
        );

        let updated_entities = store
            .update(
                EntityRegistrationMessage::new(&Message::new(
                    &Topic::new("te/device/child2//").unwrap(),
                    json!({"@type": "child-device", "@parent": "device/main//"}).to_string(),
                ))
                .unwrap(),
            )
            .unwrap();
        assert_eq!(updated_entities, ["device/main//"]);
        let children = store.child_devices(&EntityTopicId::default_main_device());
        assert!(children.iter().any(|&e| e == "device/child1//"));
        assert!(children.iter().any(|&e| e == "device/child2//"));
    }

    #[test]
    fn lists_services() {
        let mut store = new_entity_store();

        // Services are namespaced under devices, so `parent` is not necessary
        let updated_entities = store
            .update(EntityRegistrationMessage {
                r#type: EntityType::Service,
                external_id: None,
                topic_id: EntityTopicId::default_main_service("service1").unwrap(),
                parent: None,
                other: Map::new(),
            })
            .unwrap();

        assert_eq!(updated_entities, ["device/main//"]);
        assert_eq!(
            store.services(&EntityTopicId::default_main_device()),
            ["device/main/service/service1"]
        );

        let updated_entities = store
            .update(EntityRegistrationMessage {
                r#type: EntityType::Service,
                external_id: None,
                topic_id: EntityTopicId::default_main_service("service2").unwrap(),
                parent: None,
                other: Map::new(),
            })
            .unwrap();

        assert_eq!(updated_entities, ["device/main//"]);
        let services = store.services(&EntityTopicId::default_main_device());
        assert!(services
            .iter()
            .any(|&e| e == &EntityTopicId::default_main_service("service1").unwrap()));
        assert!(services
            .iter()
            .any(|&e| e == &EntityTopicId::default_main_service("service2").unwrap()));
    }

    #[test]
    fn forbids_nonexistent_parents() {
        let mut store = new_entity_store();

        let res = store.update(EntityRegistrationMessage {
            topic_id: EntityTopicId::default_main_device(),
            external_id: None,
            r#type: EntityType::ChildDevice,
            parent: Some(EntityTopicId::default_child_device("myawesomeparent").unwrap()),
            other: Map::new(),
        });

        assert!(matches!(res, Err(Error::NoParent(_))));
    }

    #[test]
    fn list_ancestors() {
        let mut store = new_entity_store();

        // Assert no ancestors of main device
        assert!(store
            .ancestors(&EntityTopicId::default_main_device())
            .unwrap()
            .is_empty());

        // Register service on main
        store
            .update(
                EntityRegistrationMessage::new(&Message::new(
                    &Topic::new("te/device/main/service/collectd").unwrap(),
                    json!({"@type": "service"}).to_string(),
                ))
                .unwrap(),
            )
            .unwrap();

        // Assert ancestors of main device service
        assert_eq!(
            store
                .ancestors(&EntityTopicId::default_main_service("collectd").unwrap())
                .unwrap(),
            ["device/main//"]
        );

        // Register immediate child of main
        store
            .update(
                EntityRegistrationMessage::new(&Message::new(
                    &Topic::new("te/device/child1//").unwrap(),
                    json!({"@type": "child-device"}).to_string(),
                ))
                .unwrap(),
            )
            .unwrap();

        // Assert ancestors of child1
        assert_eq!(
            store
                .ancestors(&EntityTopicId::default_child_device("child1").unwrap())
                .unwrap(),
            ["device/main//"]
        );

        // Register service on child1
        store
            .update(
                EntityRegistrationMessage::new(&Message::new(
                    &Topic::new("te/device/child1/service/collectd").unwrap(),
                    json!({"@type": "service"}).to_string(),
                ))
                .unwrap(),
            )
            .unwrap();

        // Assert ancestors of child1 service
        assert_eq!(
            store
                .ancestors(&EntityTopicId::default_child_service("child1", "collectd").unwrap())
                .unwrap(),
            ["device/child1//", "device/main//"]
        );

        // Register child2 as child of child1
        store
            .update(
                EntityRegistrationMessage::new(&Message::new(
                    &Topic::new("te/device/child2//").unwrap(),
                    json!({"@type": "child-device", "@parent": "device/child1//"}).to_string(),
                ))
                .unwrap(),
            )
            .unwrap();

        // Assert ancestors of child2
        assert_eq!(
            store
                .ancestors(&EntityTopicId::default_child_device("child2").unwrap())
                .unwrap(),
            ["device/child1//", "device/main//"]
        );

        // Register service on child2
        store
            .update(
                EntityRegistrationMessage::new(&Message::new(
                    &Topic::new("te/device/child2/service/collectd").unwrap(),
                    json!({"@type": "service"}).to_string(),
                ))
                .unwrap(),
            )
            .unwrap();

        // Assert ancestors of child2 service
        assert_eq!(
            store
                .ancestors(&EntityTopicId::default_child_service("child2", "collectd").unwrap())
                .unwrap(),
            ["device/child2//", "device/child1//", "device/main//"]
        );
    }

    #[test]
    fn list_ancestors_external_ids() {
        let mut store = new_entity_store();

        // Assert ancestor external ids of main device
        assert!(store
            .ancestors_external_ids(&EntityTopicId::default_main_device())
            .unwrap()
            .is_empty());

        // Register service on main
        store
            .update(
                EntityRegistrationMessage::new(&Message::new(
                    &Topic::new("te/device/main/service/collectd").unwrap(),
                    json!({"@type": "service"}).to_string(),
                ))
                .unwrap(),
            )
            .unwrap();

        // Assert ancestor external id of main device service
        assert_eq!(
            store
                .ancestors_external_ids(&EntityTopicId::default_main_service("collectd").unwrap())
                .unwrap(),
            ["test-device"]
        );

        // Register immediate child of main
        store
            .update(
                EntityRegistrationMessage::new(&Message::new(
                    &Topic::new("te/device/child1//").unwrap(),
                    json!({"@type": "child-device"}).to_string(),
                ))
                .unwrap(),
            )
            .unwrap();

        // Assert ancestor external ids of child1
        assert_eq!(
            store
                .ancestors_external_ids(&EntityTopicId::default_child_device("child1").unwrap())
                .unwrap(),
            ["test-device"]
        );

        // Register service on child1
        store
            .update(
                EntityRegistrationMessage::new(&Message::new(
                    &Topic::new("te/device/child1/service/collectd").unwrap(),
                    json!({"@type": "service"}).to_string(),
                ))
                .unwrap(),
            )
            .unwrap();

        // Assert ancestor external ids of child1 service
        assert_eq!(
            store
                .ancestors_external_ids(
                    &EntityTopicId::default_child_service("child1", "collectd").unwrap()
                )
                .unwrap(),
            ["device:child1", "test-device"]
        );

        // Register child2 as child of child1
        store
            .update(
                EntityRegistrationMessage::new(&Message::new(
                    &Topic::new("te/device/child2//").unwrap(),
                    json!({"@type": "child-device", "@parent": "device/child1//"}).to_string(),
                ))
                .unwrap(),
            )
            .unwrap();

        // Assert ancestor external ids of child2
        assert_eq!(
            store
                .ancestors_external_ids(&EntityTopicId::default_child_device("child2").unwrap())
                .unwrap(),
            ["device:child1", "test-device"]
        );

        // Register service on child2
        store
            .update(
                EntityRegistrationMessage::new(&Message::new(
                    &Topic::new("te/device/child2/service/collectd").unwrap(),
                    json!({"@type": "service"}).to_string(),
                ))
                .unwrap(),
            )
            .unwrap();

        // Assert ancestor external ids of child2 service
        assert_eq!(
            store
                .ancestors_external_ids(
                    &EntityTopicId::default_child_service("child2", "collectd").unwrap()
                )
                .unwrap(),
            ["device:child2", "device:child1", "test-device"]
        );
    }

    #[test]
    fn auto_register_service() {
        let mut store = new_entity_store();

        let service_topic_id = EntityTopicId::default_child_service("child1", "service1").unwrap();
        let res = store.auto_register_entity(&service_topic_id).unwrap();
        assert_eq!(
            res,
            [
                EntityRegistrationMessage {
                    topic_id: EntityTopicId::from_str("device/child1//").unwrap(),
                    r#type: EntityType::ChildDevice,
                    external_id: Some("device:child1".into()),
                    parent: None,
                    other: json!({ "name": "child1" }).as_object().unwrap().to_owned(),
                },
                EntityRegistrationMessage {
                    topic_id: EntityTopicId::from_str("device/child1/service/service1").unwrap(),
                    r#type: EntityType::Service,
                    external_id: Some("device:child1:service:service1".into()),
                    parent: Some(EntityTopicId::from_str("device/child1//").unwrap()),
                    other: json!({ "name": "service1",  "type": "service" })
                        .as_object()
                        .unwrap()
                        .to_owned(),
                }
            ]
        );
    }

    #[test]
    fn auto_register_child_device() {
        let mut store = new_entity_store();

        let child_topic_id = EntityTopicId::default_child_device("child2").unwrap();
        let res = store.auto_register_entity(&child_topic_id).unwrap();

        assert_eq!(
            res,
            [EntityRegistrationMessage {
                topic_id: EntityTopicId::from_str("device/child2//").unwrap(),
                r#type: EntityType::ChildDevice,
                external_id: Some("device:child2".into()),
                parent: None,
                other: json!({ "name": "child2" }).as_object().unwrap().to_owned(),
            },]
        );
    }

    #[test]
    fn auto_register_custom_topic_scheme_not_supported() {
        let mut store = new_entity_store();
        assert_matches!(
            store.auto_register_entity(&EntityTopicId::from_str("custom/child2//").unwrap()),
            Err(Error::NonDefaultTopicScheme(_))
        );
    }

    #[test]
    fn register_main_device_custom_scheme() {
        let mut store = new_entity_store();

        // Register main device with custom topic scheme
        let main_topic_id = EntityTopicId::from_str("custom/main//").unwrap();
        store
            .update(EntityRegistrationMessage {
                topic_id: main_topic_id.clone(),
                r#type: EntityType::MainDevice,
                external_id: None,
                parent: None,
                other: json!({}).as_object().unwrap().to_owned(),
            })
            .unwrap();

        let expected_entity_metadata = EntityMetadata {
            topic_id: main_topic_id.clone(),
            parent: None,
            r#type: EntityType::MainDevice,
            external_id: "test-device".into(),
            other: json!({}).as_object().unwrap().to_owned(),
            twin_data: Map::new(),
        };
        // Assert main device registered with custom topic scheme
        assert_eq!(
            store.get(&main_topic_id).unwrap(),
            &expected_entity_metadata
        );
        assert_eq!(
            store.get_by_external_id(&"test-device".into()).unwrap(),
            &expected_entity_metadata
        );

        // Register service on main device with custom scheme
        let service_topic_id = EntityTopicId::from_str("custom/main/service/collectd").unwrap();
        store
            .update(EntityRegistrationMessage {
                topic_id: service_topic_id.clone(),
                r#type: EntityType::Service,
                external_id: None,
                parent: Some(main_topic_id.clone()),
                other: json!({}).as_object().unwrap().to_owned(),
            })
            .unwrap();

        let expected_entity_metadata = EntityMetadata {
            topic_id: service_topic_id.clone(),
            parent: Some(main_topic_id),
            r#type: EntityType::Service,
            external_id: "custom:main:service:collectd".into(),
            other: json!({"type": "service"}).as_object().unwrap().to_owned(),
            twin_data: Map::new(),
        };
        // Assert service registered under main device with custom topic scheme
        assert_eq!(
            store.get(&service_topic_id).unwrap(),
            &expected_entity_metadata
        );
        assert_eq!(
            store
                .get_by_external_id(&"custom:main:service:collectd".into())
                .unwrap(),
            &expected_entity_metadata
        );
    }

    #[test]
    fn external_id_validation() {
        let mut store = new_entity_store();

        let entity_topic_id = EntityTopicId::default_child_device("child1").unwrap();
        let res = store.update(EntityRegistrationMessage {
            topic_id: entity_topic_id.clone(),
            r#type: EntityType::ChildDevice,
            external_id: Some("bad+id".into()),
            parent: None,
            other: Map::new(),
        });

        // Assert service registered under main device with custom topic scheme
        assert_eq!(
            res,
            Err(Error::InvalidExternalIdError(InvalidExternalIdError {
                external_id: "bad+id".into(),
                invalid_char: '+'
            }))
        );
    }

    #[test]
    fn update_twin_data_new_fragment() {
        let mut store = new_entity_store();

        let topic_id = EntityTopicId::default_main_device();
        let updated = store
            .update_twin_data(&topic_id, "hardware".into(), json!({ "version": 5 }))
            .unwrap();
        assert!(
            updated,
            "Inserting new key should have resulted in an update"
        );

        let entity_metadata = store.get(&topic_id).unwrap();
        assert_eq!(
            entity_metadata.twin_data.get("hardware").unwrap(),
            &json!({ "version": 5 })
        );
    }

    #[test]
    fn update_twin_data_update_existing_fragment() {
        let mut store = new_entity_store();

        let topic_id = EntityTopicId::default_main_device();
        let _ = store
            .update_twin_data(&topic_id, "hardware".into(), json!({ "version": 5 }))
            .unwrap();

        let updated = store
            .update_twin_data(&topic_id, "hardware".into(), json!({ "version": 6 }))
            .unwrap();
        assert!(
            updated,
            "Updating an existing key should have resulted in an update"
        );

        let entity_metadata = store.get(&topic_id).unwrap();
        assert_eq!(
            entity_metadata.twin_data.get("hardware").unwrap(),
            &json!({ "version": 6 })
        );
    }

    #[test]
    fn update_twin_data_remove_fragment() {
        let mut store = new_entity_store();

        let topic_id = EntityTopicId::default_main_device();

        let _ = store
            .update_twin_data(&topic_id, "foo".into(), json!("bar"))
            .unwrap();

        let updated = store
            .update_twin_data(&topic_id, "foo".into(), json!(null))
            .unwrap();
        assert!(
            updated,
            "Removing an existing key should have resulted in an update"
        );

        let entity_metadata = store.get(&topic_id).unwrap();
        assert!(entity_metadata.twin_data.get("foo").is_none());
    }

    #[test]
    fn updated_registration_message_after_twin_updates() {
        // Create an entity store with main device having an explicit `name` fragment
        let topic_id = EntityTopicId::default_main_device();
        let mut store = EntityStore::with_main_device(
            EntityRegistrationMessage {
                topic_id: topic_id.clone(),
                external_id: Some("test-device".into()),
                r#type: EntityType::MainDevice,
                parent: None,
                other: json!({ "name" : "test-name", "type": "test-type" })
                    .as_object()
                    .unwrap()
                    .to_owned(),
            },
            dummy_external_id_mapper,
            dummy_external_id_sanitizer,
        )
        .unwrap();

        // Add some additional fragments to the device twin data
        let _ = store
            .update_twin_data(&topic_id, "hardware".into(), json!({ "version": 5 }))
            .unwrap();

        // Update the name of the device with
        let new_reg = EntityRegistrationMessage {
            topic_id: EntityTopicId::default_main_device(),
            external_id: Some("test-device".into()),
            r#type: EntityType::MainDevice,
            parent: None,
            other: json!({ "name" : "new-test-device" })
                .as_object()
                .unwrap()
                .to_owned(),
        };
        store.update(new_reg).unwrap();

        // Assert that the old and new twin data are merged
        let entity_metadata = store.get(&topic_id).unwrap();
        assert_eq!(
            entity_metadata.other.get("name").unwrap(),
            &json!("new-test-device"),
            "Expected new name in twin data"
        );
        assert_eq!(
            entity_metadata.other.get("type").unwrap(),
            &json!("test-type"),
            "Expected old type in twin data"
        );
        assert_eq!(
            entity_metadata.twin_data.get("hardware").unwrap(),
            &json!({ "version": 5 }),
            "Expected old hardware fragment in twin data"
        );
    }

    fn new_entity_store() -> EntityStore {
        EntityStore::with_main_device(
            EntityRegistrationMessage {
                topic_id: EntityTopicId::default_main_device(),
                external_id: Some("test-device".into()),
                r#type: EntityType::MainDevice,
                parent: None,
                other: Map::new(),
            },
            dummy_external_id_mapper,
            dummy_external_id_sanitizer,
        )
        .unwrap()
    }
}
