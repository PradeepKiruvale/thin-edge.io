# Auto Registration of an entity

Before any data messages from an entity can be processed, the entity has to be registered first.
The entity can be registered either explicitly or implicitly (Auto registration).

In `auto-registration`, the registration does not need a separate registration message, but the registration message will be derived from the first message that is sent by that entity.

For example, sending a measurement message to te/device/child1///m/temperature, will result in the auto-registration of the entity with topic id: device/child1// and the auto-generated external id: <main-device-id>:device:child1, derived from the topic id. Similarly, a measurement message on te/device/child1/service/my-service/m/temperature, results in the auto-registration of both the device entity: device/child1// and the service entity: device/child1/service/my-service with their respective auto-generated external IDs, in that order.

Pros:

No need to have a separate registration message for an entity.

Cons:

When the message is sent for an nested child device or a service, then the registration will be done under the child device not under the nested child device.
Because the mapper can’t understand is it for the child device or for the nested child device.
	
To address the above-mentioned issue, one can configure the registration to be explicit, by setting c8y.entity_store.auto_register. to false.
Now to register an entity, one has to send an explicit registration message. Refer <here> https://thin-edge.github.io/thin-edge.io/references/mqtt-api/#entity-registration for more information about registration messages.

The auto-registration can be disabled using the tedge config command as follows:
```sh
sudo tedge config set c8y.entity_store.auto_register false
```

:::note When the auto registration is disabled, and if the device is not already registered. Then the c8y-mapper will not forward the incoming telemetry/health-status messages to the c8y cloud. But, logs/publishes an error message on the te/errors topic telling the entity is not registered. :::
The auto-registration can be enabled as below
sudo tedge config set c8y.entity_store.auto_register true

:::note By default the auto-registration is enabled. :::
