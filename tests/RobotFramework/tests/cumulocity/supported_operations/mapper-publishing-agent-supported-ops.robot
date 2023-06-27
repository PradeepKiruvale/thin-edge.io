*** Settings ***
Resource    ../../../../resources/common.resource
Library    Cumulocity
Library    ThinEdgeIO

Test Tags    theme:c8y    theme:operation    theme:tedge-agent
Test Setup    Custom Setup
Test Teardown    Get Logs

*** Test Cases ***
Publish the tedge agent supported operations delete restart
    # wait till list request is pushed out
    Should Have MQTT Messages    tedge/commands/req/software/list
    # stop mapper and remove the supported operations
    Execute Command    sudo systemctl stop tedge-mapper-c8y
    Execute Command    sudo rm -rf /etc/tedge/operations/c8y/*
  
    # the operation files must not exist
    ThinEdgeIO.File Should Not Exist    /etc/tedge/operations/c8y/c8y_SoftwareUpdate
    ThinEdgeIO.File Should Not Exist    /etc/tedge/operations/c8y/c8y_Restart
    
    # now restart the mapper
    Execute Command    sudo systemctl start tedge-mapper-c8y
    Should Have MQTT Messages    tedge/health/tedge-mapper-c8y
    
    # Check if the `c8y_SoftwareUpdate` and `c8y_Restart` ops files exists in `/etc/tedge/operations/c8y` directory
    ThinEdgeIO.File Should Exist    /etc/tedge/operations/c8y/c8y_SoftwareUpdate
    ThinEdgeIO.File Should Exist    /etc/tedge/operations/c8y/c8y_Restart

    # Check if the tedge-agent supported operations exists in c8y cloud
    Cumulocity.Should Contain Supported Operations    c8y_Restart    c8y_SoftwareUpdate   


*** Keywords ***
Custom Setup
    ${DEVICE_SN}=    Setup
    Set Suite Variable    $DEVICE_SN
    Device Should Exist                      ${DEVICE_SN}
   