*** Settings ***

Resource    ../../../resources/common.resource
Library     Cumulocity
Library     ThinEdgeIO
Library     DebugLibrary

Test Tags    theme:c8y    theme:monitoring    theme:mqtt
Test Setup    Custom Setup
Test Teardown    Get Logs


*** Variables ***

@{SERVICES}    tedge-mapper-c8y    tedge-agent
${TEDGE_SERVICE}    HELLO


*** Test Cases ***


Test Service
    [Documentation]    Loops over list of thin-edge services

    FOR    ${service}    IN  @{SERVICES}       
        Log    "Hello" ${service}
        Log    "Device ID" ${DEVICE_SN}
        Check up health status of a service    ${service}
        Check down health status of a service    ${service}
    END
    Test down health status of tedge-mapper-c8y service on broker restart

*** Keywords ***

Custom Setup
    ${DEVICE_SN}=    Setup
    Set Suite Variable    $DEVICE_SN
    Device Should Exist                      ${DEVICE_SN}  

Check up health status of a service
    [Arguments]    ${service_name}
    ThinEdgeIO.Start Service    ${service_name}
    ThinEdgeIO.Service Should Be Running    ${service_name}
       
    Device Should Exist                      ${DEVICE_SN}_${service_name}    show_info=False 
    ${SERVICE}=    Cumulocity.Device Should Have Fragment Values    status\=up
    
    Should Be Equal    ${SERVICE["name"]}    ${service_name}
    Should Be Equal    ${SERVICE["serviceType"]}    service
    Should Be Equal    ${SERVICE["status"]}    up
    Should Be Equal    ${SERVICE["type"]}    c8y_Service
    ThinEdgeIO.Stop Service    ${service_name}
    Log to console    Item ${service_name}


Check down health status of a service
    [Arguments]    ${service_name}
    ThinEdgeIO.Start Service    tedge-mapper-c8y
    ThinEdgeIO.Start Service    ${service_name}
    Sleep    2
    ThinEdgeIO.Stop Service    ${service_name}
    Sleep    2
    ThinEdgeIO.Service Should Be Stopped  ${service_name}
       
    Device Should Exist                      ${DEVICE_SN}_${service_name}    show_info=False 
    ${SERVICE}=    Cumulocity.Device Should Have Fragment Values    status\=down
    
    Should Be Equal    ${SERVICE["name"]}    ${service_name}
    Log    "service type" ${SERVICE["serviceType"]}
    Should Be Equal    ${SERVICE["serviceType"]}    service
    Should Be Equal    ${SERVICE["status"]}    down
    Should Be Equal    ${SERVICE["type"]}    c8y_Service
   
    Log to console    Item ${service_name}
    
Test down health status of tedge-mapper-c8y service on broker restart
  
    ThinEdgeIO.Start Service   tedge-mapper-c8y
    ThinEdgeIO.Service Should Be Running  tedge-mapper-c8y
     
    Device Should Exist                      ${DEVICE_SN}_tedge-mapper-c8y    show_info=False 
    ${SERVICE}=    Cumulocity.Device Should Have Fragment Values    status\=up
    Should Be Equal    ${SERVICE["status"]}    up
     
    ThinEdgeIO.Restart Service    mosquitto.service
    ThinEdgeIO.Service Should Be Running  mosquitto.service

    Device Should Exist                      ${DEVICE_SN}_tedge-mapper-c8y    show_info=False
    ${SERVICE}=    Cumulocity.Device Should Have Fragment Values    status\=down
    Should Be Equal    ${SERVICE["status"]}    down
        