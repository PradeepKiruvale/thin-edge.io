*** Settings ***
Resource    ../../../resources/common.resource
Library    Cumulocity
Library    ThinEdgeIO

Test Tags    theme:c8y    theme:registration
Suite Setup    Custom Setup
Test Setup    Test Setup
Test Teardown    Get Logs    ${DEVICE_SN}

*** Test Cases ***

Main device registration
    ${mo}=    Device Should Exist              ${DEVICE_SN}
    ${mo}=    Cumulocity.Device Should Have Fragment Values    name\=${DEVICE_SN}
    Should Be Equal    ${mo["owner"]}    device_${DEVICE_SN}
    Should Be Equal    ${mo["name"]}    ${DEVICE_SN}


Child device registration
    Execute Command    mkdir -p /etc/tedge/operations/c8y/${CHILD_SN}
    Restart Service    tedge-mapper-c8y

    # Check registration
    ${child_mo}=    Device Should Exist        ${CHILD_SN}
    ${child_mo}=    Cumulocity.Device Should Have Fragment Values    name\=${CHILD_SN}
    Should Be Equal    ${child_mo["owner"]}    device_${DEVICE_SN}    # The parent is the owner of the child
    Should Be Equal    ${child_mo["name"]}     ${CHILD_SN}

    # Check child device relationship
    Cumulocity.Set Device    ${DEVICE_SN}
    Cumulocity.Should Be A Child Device Of Device    ${CHILD_SN}

Auto register disabled
    ${timestamp}=        Get Unix Timestamp
    Execute Command    sudo tedge config set c8y.entity_store.auto_register false
    Restart Service    tedge-mapper-c8y    
    Service Health Status Should Be Up    tedge-mapper-c8y    
    Execute Command    sudo tedge mqtt pub 'te/device/auto_reg_device///a/temperature_high' '{ "severity": "critical", "text": "Temperature is very high", "time": "2021-01-01T05:30:45+00:00" }' -q 2 -r
    Should Have MQTT Messages    te/errors    message_contains=The provided entity: device/auto_reg_device// was not found and could not be auto-registered either, because it is disabled    date_from=${timestamp}   minimum=1    maximum=1
    Execute Command    sudo tedge config unset c8y.entity_store.auto_register
    Restart Service    tedge-mapper-c8y


Register child device with defaults via MQTT
    Execute Command    tedge mqtt pub --retain 'te/device/${CHILD_SN}//' '{"@type":"child-device"}'
    Check Child Device    parent_sn=${DEVICE_SN}    child_sn=${CHILD_XID}    child_name=${CHILD_XID}    child_type=thin-edge.io-child

Register child device with custom name and type via MQTT
    Execute Command    tedge mqtt pub --retain 'te/device/${CHILD_SN}//' '{"@type":"child-device","name":"${CHILD_SN}","type":"linux-device-Aböut"}'
    Check Child Device    parent_sn=${DEVICE_SN}    child_sn=${CHILD_XID}    child_name=${CHILD_SN}    child_type=linux-device-Aböut

Register child device with custom id via MQTT
    Execute Command    tedge mqtt pub --retain 'te/device/${CHILD_SN}//' '{"@type":"child-device","@id":"custom-${CHILD_XID}","name":"custom-${CHILD_SN}"}'
    Check Child Device    parent_sn=${DEVICE_SN}    child_sn=custom-${CHILD_XID}    child_name=custom-${CHILD_SN}    child_type=thin-edge.io-child

Register nested child device using default topic schema via MQTT
    ${child_level1}=    Get Random Name
    ${child_level2}=    Get Random Name
    ${child_level3}=    Get Random Name

    Execute Command    tedge mqtt pub --retain 'te/device/${child_level1}//' '{"@type":"child-device","@parent":"device/main//"}'
    Execute Command    tedge mqtt pub --retain 'te/device/${child_level2}//' '{"@type":"child-device","@parent":"device/${child_level1}//","name":"${child_level2}"}'
    Execute Command    tedge mqtt pub --retain 'te/device/${child_level3}//' '{"@type":"child-device","@parent":"device/${child_level2}//","type":"child_level3"}'
    Execute Command    tedge mqtt pub --retain 'te/device/${child_level3}/service/custom-app' '{"@type":"service","@parent":"device/${child_level3}//","name":"custom-app","type":"service-level3"}'

    # Level 1
    Check Child Device    parent_sn=${DEVICE_SN}    child_sn=${DEVICE_SN}:device:${child_level1}    child_name=${DEVICE_SN}:device:${child_level1}    child_type=thin-edge.io-child

    # Level 2
    Check Child Device    parent_sn=${DEVICE_SN}:device:${child_level1}    child_sn=${DEVICE_SN}:device:${child_level2}    child_name=${child_level2}    child_type=thin-edge.io-child

    # Level 3
    Check Child Device    parent_sn=${DEVICE_SN}:device:${child_level2}    child_sn=${DEVICE_SN}:device:${child_level3}    child_name=${DEVICE_SN}:device:${child_level3}    child_type=child_level3
    Check Service    child_sn=${DEVICE_SN}:device:${child_level3}    service_sn=${DEVICE_SN}:device:${child_level3}:service:custom-app    service_name=custom-app    service_type=service-level3    service_status=up


Register service on a child device via MQTT
    Execute Command    tedge mqtt pub --retain 'te/device/${CHILD_SN}//' '{"@type":"child-device","name":"${CHILD_SN}","type":"linux-device-Aböut"}'
    Execute Command    tedge mqtt pub --retain 'te/device/${CHILD_SN}/service/custom-app' '{"@type":"service","@parent":"device/${CHILD_SN}//","name":"custom-app","type":"custom-type"}'

    # Check child registration
    Check Child Device    parent_sn=${DEVICE_SN}    child_sn=${CHILD_XID}    child_name=${CHILD_SN}    child_type=linux-device-Aböut

    # Check service registration
    Check Service    child_sn=${CHILD_XID}    service_sn=${CHILD_XID}:service:custom-app    service_name=custom-app    service_type=custom-type    service_status=up


Register devices using custom MQTT schema
    [Documentation]    Complex example showing how to use custom MQTT topics to register devices/services using
        ...            custom identity schemas

    Execute Command    tedge mqtt pub --retain 'te/base///' '{"@type":"device","name":"base","type":"te_gateway"}'

    Execute Command    tedge mqtt pub --retain 'te/factory1/shop1/plc1/sensor1' '{"@type":"child-device","name":"sensor1","type":"SmartSensor"}'
    Execute Command    tedge mqtt pub --retain 'te/factory1/shop1/plc1/sensor2' '{"@type":"child-device","name":"sensor2","type":"SmartSensor"}'

    # Service of main device
    Execute Command    tedge mqtt pub --retain 'te/factory1/shop1/plc1/metrics' '{"@type":"service","name":"metrics","type":"PLCApplication"}'

    # Service of child device
    Execute Command    tedge mqtt pub --retain 'te/factory1/shop1/apps/sensor1' '{"@type":"service","@parent":"factory1/shop1/plc1/sensor1","name":"metrics","type":"PLCMonitorApplication"}'
    Execute Command    tedge mqtt pub --retain 'te/factory1/shop1/apps/sensor2' '{"@type":"service","@parent":"factory1/shop1/plc1/sensor2","name":"metrics","type":"PLCMonitorApplication"}'

    Check Child Device    parent_sn=${DEVICE_SN}    child_sn=${DEVICE_SN}:factory1:shop1:plc1:sensor1    child_name=sensor1    child_type=SmartSensor
    Check Child Device    parent_sn=${DEVICE_SN}    child_sn=${DEVICE_SN}:factory1:shop1:plc1:sensor2    child_name=sensor2    child_type=SmartSensor

    # Check main device services
    Cumulocity.Set Device    ${DEVICE_SN}
    Should Have Services    name=metrics    service_type=PLCApplication    status=up

    # Check child services
    Cumulocity.Set Device    ${DEVICE_SN}:factory1:shop1:plc1:sensor1
    Should Have Services    name=metrics    service_type=PLCMonitorApplication    status=up

    Cumulocity.Set Device    ${DEVICE_SN}:factory1:shop1:plc1:sensor2
    Should Have Services    name=metrics    service_type=PLCMonitorApplication    status=up

    # Publish to main device on custom topic
    Execute Command    cmd=tedge mqtt pub te/base////m/gateway_stats '{"runtime":1001}'
    Cumulocity.Set Device    ${DEVICE_SN}
    Cumulocity.Device Should Have Measurements    type=gateway_stats    minimum=1    maximum=1


Register tedge-agent when tedge-mapper-c8y is not running #2389
    [Teardown]    Start Service    tedge-mapper-c8y
    Device Should Exist    ${DEVICE_SN}

    Stop Service    tedge-mapper-c8y
    Execute Command    cmd=timeout 5 env TEDGE_RUN_LOCK_FILES=false tedge-agent --mqtt-device-topic-id device/offlinechild1//    ignore_exit_code=${True}
    Start Service    tedge-mapper-c8y

    Should Be A Child Device Of Device    ${DEVICE_SN}:device:offlinechild1
    Should Have MQTT Messages    te/device/offlinechild1//    minimum=1

    Device Should Exist    ${DEVICE_SN}:device:offlinechild1
    Cumulocity.Restart Device
    Should Have MQTT Messages    te/device/offlinechild1///cmd/restart/+


Register tedge-configuration-plugin when tedge-mapper-c8y is not running #2389
    [Teardown]    Start Service    tedge-mapper-c8y
    Device Should Exist    ${DEVICE_SN}

    Stop Service    tedge-mapper-c8y
    Execute Command    cmd=timeout 5 tedge-configuration-plugin --mqtt-device-topic-id device/offlinechild2//    ignore_exit_code=${True}
    Start Service    tedge-mapper-c8y

    Should Be A Child Device Of Device    ${DEVICE_SN}:device:offlinechild2
    Should Have MQTT Messages    te/device/offlinechild2//    minimum=1

    Device Should Exist    ${DEVICE_SN}:device:offlinechild2
    Cumulocity.Get Configuration    dummy1
    Should Have MQTT Messages    te/device/offlinechild2///cmd/config_snapshot/+


Register tedge-log-plugin when tedge-mapper-c8y is not running #2389
    [Teardown]    Start Service    tedge-mapper-c8y
    Device Should Exist    ${DEVICE_SN}

    Stop Service    tedge-mapper-c8y
    Execute Command    cmd=timeout 5 tedge-log-plugin --mqtt-device-topic-id device/offlinechild3//    ignore_exit_code=${True}
    Start Service    tedge-mapper-c8y

    Should Be A Child Device Of Device    ${DEVICE_SN}:device:offlinechild3
    Should Have MQTT Messages    te/device/offlinechild3//    minimum=1

    Device Should Exist    ${DEVICE_SN}:device:offlinechild3
    Cumulocity.Create Operation
    ...    description=Log file request
    ...    fragments={"c8y_LogfileRequest":{"dateFrom":"2023-01-01T01:00:00+0000","dateTo":"2023-01-02T01:00:00+0000","logFile":"example1","searchText":"first","maximumLines":10}}
    Should Have MQTT Messages    te/device/offlinechild3///cmd/log_upload/+
    
*** Keywords ***

Check Child Device
    [Arguments]    ${parent_sn}    ${child_sn}    ${child_name}    ${child_type}
    ${child_mo}=    Device Should Exist        ${child_sn}

    ${child_mo}=    Cumulocity.Device Should Have Fragment Values    name\=${child_name}
    Should Be Equal    ${child_mo["owner"]}    device_${DEVICE_SN}    # The parent is the owner of the child
    Should Be Equal    ${child_mo["name"]}     ${child_name}
    Should Be Equal    ${child_mo["type"]}     ${child_type}

    # Check child device relationship
    Cumulocity.Device Should Exist    ${parent_sn}
    Cumulocity.Should Be A Child Device Of Device    ${child_sn}

Check Service
    [Arguments]    ${child_sn}    ${service_sn}    ${service_name}    ${service_type}    ${service_status}=up
    Cumulocity.Device Should Exist    ${service_sn}    show_info=${False}
    Cumulocity.Device Should Exist    ${child_sn}    show_info=${False}
    Should Have Services    name=${service_name}    service_type=${service_type}    status=${service_status}


Test Setup
    ${CHILD_SN}=    Get Random Name
    Set Test Variable    $CHILD_SN
    Set Test Variable    $CHILD_XID    ${DEVICE_SN}:device:${CHILD_SN}

    ThinEdgeIO.Set Device Context    ${DEVICE_SN}

Custom Setup
    ${DEVICE_SN}=                    Setup
    Set Suite Variable               $DEVICE_SN
