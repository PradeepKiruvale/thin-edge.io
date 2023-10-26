---
title: Sending Measurements
tags: [Getting Started, Telemetry]
sidebar_position: 5
---

# Sending Measurements

Once your Thin Edge device is configured and connected to an IoT cloud provider, you can start sending measurements.
Refer to [Connecting to Cumulocity](./connect-c8y.md) or tutorials for other cloud providers 
to learn how to connect your Thin Edge device to an IoT cloud provider. 

In this tutorial, we'll see how different kinds of measurements are represented in Thin Edge JSON format and 
how they can be sent to the connected cloud provider.
For a more detailed specification of this data format, refer to [Thin Edge JSON Specification](../understand/thin-edge-json.md).

## Sending measurements

A simple single-valued measurement like a temperature measurement, can be represented in Thin Edge JSON as follows:

```json
{"temperature": 25}
```

with the key-value pair representing the measurement type and the numeric value of the measurement.

This measurement can be sent from the Thin Edge device to the cloud by publishing this message to the `tedge/measurements` MQTT topic.
Processes running on the Thin Edge device can publish messages to the local MQTT broker using any MQTT client or library.
In this tutorial, we'll be using the `tedge mqtt pub` command line utility for demonstration purposes.

The temperature measurement described above can be sent using the `tedge mqtt pub` command as follows:

```sh te2mqtt
tedge mqtt pub tedge/measurements '{"temperature": 25}'
```

The first argument to the `tedge mqtt pub` command is the topic to which the measurements must be published to.
The second argument is the Thin Edge JSON representation of the measurement itself.

When connected to a cloud provider, a message mapper component for that cloud provider would be running as a daemon, 
listening to any measurements published to `tedge/measurements`.
The mapper, on receipt of these Thin Edge JSON measurements, will map those measurements to their equivalent
cloud provider native representation and send it to that cloud.

For example, when the device is connected to Cumulocity, the Cumulocity mapper component will be performing these actions.
To check if these measurements have reached Cumulocity, login to your Cumulocity dashboard and navigate to:

**Device Management** &rarr; **Devices** &rarr; **All devices** &rarr; `device-id` &rarr; **Measurements**

You can see if your temperature measurement is appearing in the dashboard.

## Complex measurements

You can represent measurements that are far more complex than the single-valued ones described above using the Thin Edge JSON format.

A multi-valued measurement like `three_phase_current` that consists of `L1`, `L2` and `L3` values,
representing the current on each phase can be represented as follows:

```json
{
  "three_phase_current": {
    "L1": 9.5,
    "L2": 10.3,
    "L3": 8.8
  }
}
```

Here is another complex message consisting of single-valued measurements: `temperature` and `pressure` 
along with a multi-valued `coordinate` measurement, all sharing a single timestamp captured as `time`.

```json
{
  "time": "2020-10-15T05:30:47+00:00",
  "temperature": 25,
  "current": {
    "L1": 9.5,
    "L2": 10.3,
    "L3": 8.8
  },
  "pressure": 98
}
```

The `time` field is not a regular measurement like `temperature` or `pressure` but a special reserved field.
Refer to [Thin Edge JSON Specification](../understand/thin-edge-json.md) for more details on the kinds of telemetry 
data that can be represented in Thin Edge JSON format and the reserved fields like `time` used in the above example.

## Sending measurements to child devices

If valid Thin Edge JSON measurements are published to the `tedge/measurements/<child-id>` topic,
the measurements are recorded under a child device of your thin-edge.io device.

Given your desired child device ID is `child1`, publish a Thin Edge JSON message to the `tedge/measurements/child1` topic:

```sh te2mqtt
tedge mqtt pub tedge/measurements/child1 '{"temperature": 25}'
```

Then, you will see a child device with the name `child1` is created in your Cumulocity IoT tenant,
and the measurement is recorded in `Measurements` of the `child1` device.

## Sending measurements to nested-child devices

If valid Thin Edge JSON measurements are published to the `te/device/<nested-child-device>///m/<measurement-type>` topic,
the measurements are recorded under a child device of your thin-edge.io device.

Given your desired child device ID is `nested-child-device`, publish a Thin Edge JSON message to the `te/device/nested-child-device///m/` topic:

```sh te2mqtt
tedge mqtt pub te/device/nested-child-device///m/ '{"temperature": 25}'
```

Then, you will see a child device with the name `nested-child-device` is created in your Cumulocity IoT tenant,
and the measurement is recorded in `Measurements` of the `nested-child-device` device.

> Note: Before sending the measurement to the nested-child-device has to be registered/created under a child device if not registered already.

## Sending measurements to thin-edge device service

If valid Thin Edge JSON measurements are published to the `te/device/main/service/main-device-service/m/<measurement-type>` topic,
the measurements are recorded under a child device of your thin-edge.io device.

Given your desired child device ID is `main`, publish a Thin Edge JSON message to the `te/device/main/service/main-device-service/m/` topic:

```sh te2mqtt
tedge mqtt pub te/device/main///m/ '{"temperature": 25}'
```

Then, you will see a thin-edge device with the name `device name`(SN) is created in your Cumulocity IoT tenant,
and the measurement is recorded in `Measurements` of the `thin-edge` device.

## Sending measurements to child device service

If valid Thin Edge JSON measurements are published to the `te/device/<child-device>/service/<child-device-service>/m/<measurement-type>` topic,
the measurements are recorded under a child device's service of your thin-edge.io device.

Given your desired child device ID is `child-device`, publish a Thin Edge JSON message to the `te/device/<child-device>/service/<child-device-service>/m/` topic:

```sh te2mqtt
tedge mqtt pub te/device/<child-device>/service/<child-device-service>/m/ '{"temperature": 25}'
```

Then, you will see a service created with the name `child-device-service` under the child device with the name `child-device` in your Cumulocity IoT tenant,
and the measurement is recorded in `Measurements` of the `child-device-service` service.

> Note: Before sending the measurement to the nested-child-device's service, the service has to be registered/created under a child device if not registered already.


## Sending measurements to nested child device service

If valid Thin Edge JSON measurements are published to the `te/device/<nested-child-device>/service/<nested-child-device-service>/m/<measurement-type>` topic,
the measurements are recorded under a nested child device's service of your thin-edge.io device.

Given your desired nested child device ID is `nested-child-device`, publish a Thin Edge JSON message to the `te/device/<nested-child-device>/service/<nested-child-device-service>/m/` topic:

```sh te2mqtt
tedge mqtt pub te/device/<nested-child-device>/service/<nested-child-device-service>/m/ '{"temperature": 25}'
```

Then, you will see a service created with the name `nested-child-device-service` under the child device with the name `nested-child-device` in your Cumulocity IoT tenant,
and the measurement is recorded in `Measurements` of the `nested-child-device-service` service.

> Note: Before sending the measurement to the nested-child-device's service, the child device has to be created and then the nested child device has to be created
 and then the service has to be registered/created under a nested child device if not registered already.


## Error detection

If the data published to the `tedge/measurements` topic are not valid Thin Edge JSON measurements, those won't be
sent to the cloud but instead you'll get a feedback on the `tedge/errors` topic, if you subscribe to it.
The error messages published to this topic will be highly verbose and may change in the future.
So, use it only for debugging purposes during the development phase and it should **NOT** be used for any automation.

You can subscribe to the error topic as follows:

```sh te2mqtt
tedge mqtt sub tedge/errors
```
