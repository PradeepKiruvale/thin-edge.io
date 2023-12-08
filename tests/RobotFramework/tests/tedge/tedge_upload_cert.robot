*** Settings ***
Documentation    Run certificate upload test and fails to upload the cert because there is no root certificate directory,
...              Then check the negative response in stderr              

Resource    ../../resources/common.resource
Library    ThinEdgeIO

Test Tags    theme:cli    theme:mqtt    theme:c8y
Suite Setup    Custom Setup
Suite Teardown         Get Logs

*** Test Cases ***
Create the certificate    
    #You can then check the content of that certificate.
    ${output}=    Execute Command    sudo tedge cert show    #You can then check the content of that certificate.
    Should Contain    ${output}    Device certificate: /etc/tedge/device-certs/tedge-certificate.pem
    Should Contain    ${output}    Subject: CN=${DEVICE_SN}, O=Thin Edge, OU=Test Device
    Should Contain    ${output}    Issuer: CN=${DEVICE_SN}, O=Thin Edge, OU=Test Device
    Should Contain    ${output}    Valid from:
    Should Contain    ${output}    Valid up to:
    Should Contain    ${output}    Thumbprint:

Renew the certificate
    Execute Command    sudo tedge disconnect c8y 
    ${output}=    Execute Command    sudo tedge cert renew    stderr=${True}    stdout=${False}    ignore_exit_code=${True}    
    Should Contain    ${output}    Certificate was successfully renewed, for un-interrupted service, the certificate has to be uploaded to the cloud
    Execute Command    sudo env C8YPASS\='${C8Y_CONFIG.password}' tedge cert upload c8y --user ${C8Y_CONFIG.username}
    ${output}=    Execute Command    sudo tedge connect c8y    
    Should Contain    ${output}    Connection check is successful.


Renew certificate fails
    Execute Command    sudo tedge cert remove    
    ${output}=    Execute Command    sudo tedge cert renew    stderr=${True}    stdout=${False}    ignore_exit_code=${True}    
    Should Contain    ${output}    Missing file: "/etc/tedge/device-certs/tedge-certificate.pem"
    # Restore the certificate
    Execute Command    sudo tedge cert create --device-id test-user    

tedge cert upload c8y command fails
    Execute Command    tedge config set c8y.root_cert_path /etc/ssl/certs_test    
    ${output}=    Execute Command    sudo env C8YPASS\='password' tedge cert upload c8y --user testuser    ignore_exit_code=${True}    stdout=${False}    stderr=${True}
    Execute Command    tedge config unset c8y.root_cert_path
    Should Contain    ${output}    Root certificate path /etc/ssl/certs_test does not exist

*** Keywords ***
Custom Setup
    ${DEVICE_SN}=                    Setup
    Set Suite Variable               $DEVICE_SN